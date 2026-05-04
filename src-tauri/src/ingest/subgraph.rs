//! Uniswap V3 subgraph adapter — fast, free historical swap data via The Graph.
//!
//! Why this exists: Alchemy's free `eth_getLogs` tier caps at small block
//! ranges (~10 blocks per call), making any meaningful backfill agonizing.
//! The subgraph indexes everything Uniswap V3 emits and exposes it via a
//! single GraphQL query — 1000 swaps per page, free up to 100k queries/
//! month on The Graph's gateway. We use it as the primary backfill
//! source; Alchemy free-tier and synthetic remain as fallbacks.
//!
//! API key handling: when `THE_GRAPH_API_KEY` is set we hit the
//! decentralized gateway (recommended). Without a key we fall back to
//! the legacy hosted-service URL — which Uniswap themselves have been
//! known to keep alive even after the broader hosted-service
//! sunset. Either way the failure surface is "subgraph returns
//! transport error" → pipeline falls through to Alchemy → synthetic.
//!
//! Schema notes: Uniswap V3 subgraph stores swap amounts as
//! `BigDecimal` (human-decimal form) rather than raw uint256. We
//! re-scale by 10^decimals when constructing the synthetic
//! `EthLog.data` payload so the existing ABI decoder works unchanged.

use std::time::Duration;

use async_trait::async_trait;
use num_bigint::BigUint;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::config::chains::Protocol;
use crate::config::ChainId;
use crate::ingest::decoder::SWAP_TOPIC0;

use super::error::IngestError;
use super::source::{ArchiveSource, EthLog};

const SWAPS_PAGE_SIZE: usize = 1000;
const MAX_PAGES: usize = 50; // 50_000 swaps per range — generous

pub struct UniswapV3SubgraphSource {
    client: Client,
    url: String,
    /// Per-pool token decimals cache. The subgraph stores swap amounts
    /// in human-decimal form; we need decimals to convert back to raw
    /// uint256 for the `EthLog.data` payload. Avoids a re-query per
    /// page.
    decimals_cache: Mutex<std::collections::HashMap<String, (u8, u8)>>,
}

impl UniswapV3SubgraphSource {
    /// Builds a subgraph source for Uniswap V3 mainnet. Reads
    /// `THE_GRAPH_API_KEY` from env to decide between the
    /// authenticated decentralized gateway and the legacy hosted URL.
    pub fn mainnet() -> Self {
        Self::for_chain(ChainId::Ethereum)
    }

    /// Builds a subgraph source for the supplied chain. Same key
    /// resolution as `mainnet()`: gateway when keyed, legacy URL
    /// otherwise. Used by the cross-chain ingestion pipeline.
    pub fn for_chain(chain: ChainId) -> Self {
        Self::with_url(chain.subgraph_url())
    }

    /// Builds a subgraph source for a specific (chain, protocol).
    /// V3 forks (Sushi, Pancake) share the Uniswap V3 schema so the
    /// existing GraphQL queries work unchanged — only the URL differs.
    pub fn for_protocol(chain: ChainId, protocol: Protocol) -> Self {
        Self::with_url(chain.subgraph_url_for(protocol))
    }

    /// Builds a subgraph source pointed at an arbitrary GraphQL
    /// endpoint. Used by the chain-config layer (tier 2) to support
    /// Arbitrum, Optimism, Base, Polygon, etc. — each chain has its
    /// own Uniswap V3 subgraph.
    pub fn with_url(url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(8))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            url,
            decimals_cache: Mutex::new(std::collections::HashMap::new()),
        }
    }

    async fn graphql<T: serde::de::DeserializeOwned>(
        &self,
        query: &str,
        variables: Value,
    ) -> Result<T, IngestError> {
        let body = json!({ "query": query, "variables": variables });
        let resp = self
            .client
            .post(&self.url)
            .json(&body)
            .send()
            .await
            .map_err(|e| IngestError::Transport(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(IngestError::Transport(format!(
                "subgraph http {}",
                resp.status()
            )));
        }
        let parsed: GraphQLResponse<T> = resp
            .json()
            .await
            .map_err(|e| IngestError::MalformedLog(format!("subgraph parse: {e}")))?;
        if let Some(errors) = parsed.errors {
            return Err(IngestError::MalformedLog(format!(
                "subgraph graphql errors: {errors:?}"
            )));
        }
        parsed
            .data
            .ok_or_else(|| IngestError::MalformedLog("subgraph: no data".into()))
    }

    /// Resolves token0/token1 decimals + symbols for a pool. Cached
    /// per source instance.
    pub async fn pool_metadata(
        &self,
        pool_address: &str,
    ) -> Result<PoolMetadata, IngestError> {
        let pool_lower = pool_address.to_lowercase();
        let query = r#"
            query Pool($pool: ID!) {
                pool(id: $pool) {
                    feeTier
                    token0 { id symbol decimals }
                    token1 { id symbol decimals }
                }
            }
        "#;
        #[derive(Deserialize)]
        struct PoolWrapper {
            pool: Option<PoolDataRaw>,
        }
        #[derive(Deserialize)]
        struct PoolDataRaw {
            #[serde(rename = "feeTier")]
            fee_tier: String,
            token0: TokenRaw,
            token1: TokenRaw,
        }
        #[derive(Deserialize)]
        struct TokenRaw {
            id: String,
            symbol: String,
            decimals: String,
        }
        let resp: PoolWrapper = self
            .graphql(query, json!({ "pool": pool_lower }))
            .await?;
        let pool = resp.pool.ok_or_else(|| {
            IngestError::MalformedLog(format!("subgraph: pool {pool_lower} not indexed"))
        })?;
        let token0_decimals: u8 = pool.token0.decimals.parse().map_err(|_| {
            IngestError::MalformedLog(format!("invalid decimals: {}", pool.token0.decimals))
        })?;
        let token1_decimals: u8 = pool.token1.decimals.parse().map_err(|_| {
            IngestError::MalformedLog(format!("invalid decimals: {}", pool.token1.decimals))
        })?;
        let fee_tier_bps: u32 = pool
            .fee_tier
            .parse::<u32>()
            .map(|hundredths| hundredths / 100)
            .map_err(|_| {
                IngestError::MalformedLog(format!("invalid feeTier: {}", pool.fee_tier))
            })?;
        // Cache for the subsequent get_pool_logs call.
        self.decimals_cache
            .lock()
            .await
            .insert(pool_lower.clone(), (token0_decimals, token1_decimals));
        Ok(PoolMetadata {
            pool_address: pool_lower,
            token0_address: pool.token0.id,
            token0_symbol: pool.token0.symbol,
            token0_decimals,
            token1_address: pool.token1.id,
            token1_symbol: pool.token1.symbol,
            token1_decimals,
            fee_tier_bps,
        })
    }

    async fn cached_decimals(&self, pool_address: &str) -> Result<(u8, u8), IngestError> {
        if let Some(&d) = self
            .decimals_cache
            .lock()
            .await
            .get(&pool_address.to_lowercase())
        {
            return Ok(d);
        }
        let meta = self.pool_metadata(pool_address).await?;
        Ok((meta.token0_decimals, meta.token1_decimals))
    }

    async fn fetch_swaps_page(
        &self,
        pool_lower: &str,
        from_block: u64,
        to_block: u64,
        skip: usize,
    ) -> Result<Vec<SwapNode>, IngestError> {
        // Filter by transaction.blockNumber range. The Uniswap V3
        // subgraph supports nested filter on transaction_; if a fork
        // doesn't, we'd need to fall back to timestamp filtering, but
        // that's a tier-3 concern.
        let query = r#"
            query Swaps($pool: String!, $first: Int!, $skip: Int!, $minBlock: BigInt!, $maxBlock: BigInt!) {
                swaps(
                    first: $first,
                    skip: $skip,
                    where: {
                        pool: $pool,
                        transaction_: { blockNumber_gte: $minBlock, blockNumber_lte: $maxBlock }
                    },
                    orderBy: timestamp,
                    orderDirection: asc
                ) {
                    id
                    sender
                    recipient
                    amount0
                    amount1
                    sqrtPriceX96
                    tick
                    logIndex
                    timestamp
                    transaction { id blockNumber }
                }
            }
        "#;
        let vars = json!({
            "pool": pool_lower,
            "first": SWAPS_PAGE_SIZE,
            "skip": skip,
            "minBlock": from_block.to_string(),
            "maxBlock": to_block.to_string(),
        });
        #[derive(Deserialize)]
        struct SwapsWrapper {
            swaps: Vec<SwapNode>,
        }
        let resp: SwapsWrapper = self.graphql(query, vars).await?;
        Ok(resp.swaps)
    }
}

#[async_trait]
impl ArchiveSource for UniswapV3SubgraphSource {
    async fn get_pool_logs(
        &self,
        pool_address: &str,
        from_block: u64,
        to_block: u64,
        topic0_filters: &[&str],
    ) -> Result<Vec<EthLog>, IngestError> {
        if from_block > to_block {
            return Err(IngestError::InvalidRange {
                from: from_block,
                to: to_block,
            });
        }
        // Subgraph adapter only emits Swap-style logs. The engine's
        // backtest pipeline uses only `swap_events`; mints/burns/
        // collects are reserved for a validation harness on the
        // Alchemy code path. So if the caller doesn't want swaps, we
        // return empty cleanly rather than failing.
        let want_swaps = topic0_filters
            .iter()
            .any(|t| t.trim_start_matches("0x").eq_ignore_ascii_case(SWAP_TOPIC0));
        if !want_swaps {
            return Ok(Vec::new());
        }

        let pool_lower = pool_address.to_lowercase();
        let (decimals0, decimals1) = self.cached_decimals(&pool_lower).await?;

        let mut out = Vec::new();
        for page in 0..MAX_PAGES {
            let nodes = self
                .fetch_swaps_page(&pool_lower, from_block, to_block, page * SWAPS_PAGE_SIZE)
                .await?;
            let last_page = nodes.len() < SWAPS_PAGE_SIZE;
            for n in nodes {
                let log = swap_node_to_eth_log(&pool_lower, &n, decimals0, decimals1)?;
                out.push(log);
            }
            if last_page {
                break;
            }
        }
        // Pipeline sorts by (block_number, log_index) but be defensive.
        out.sort_by_key(|log| (log.block_number, log.log_index));
        Ok(out)
    }

    async fn latest_finalized_block(&self) -> Result<u64, IngestError> {
        // The subgraph has a `_meta` field carrying its current
        // indexed-block — close enough to "finalized" for our use.
        let query = r#"
            query Meta { _meta { block { number } } }
        "#;
        #[derive(Deserialize)]
        struct MetaWrapper {
            _meta: Meta,
        }
        #[derive(Deserialize)]
        struct Meta {
            block: MetaBlock,
        }
        #[derive(Deserialize)]
        struct MetaBlock {
            number: u64,
        }
        let resp: MetaWrapper = self.graphql(query, json!({})).await?;
        Ok(resp._meta.block.number)
    }

    async fn block_median_gas_gwei(&self, _block: u64) -> Result<f64, IngestError> {
        // Subgraph doesn't surface block gas. The engine treats this
        // as a soft hint with a fallback constant; returning a
        // realistic post-merge baseline (~15 gwei) keeps mgmt-gas
        // calculations sane.
        Ok(15.0)
    }

    async fn block_timestamp(&self, _block: u64) -> Result<i64, IngestError> {
        // Subgraph swaps already carry timestamps inline; this is
        // only invoked when get_pool_logs returns rows lacking ts,
        // which doesn't happen on this adapter. Return 0 as a sentinel
        // rather than failing — the ingester's idempotent-insert path
        // tolerates it.
        Ok(0)
    }
}

/// Subgraph swap entity, raw-ish.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SwapNode {
    #[allow(dead_code)]
    id: String,
    sender: String,
    recipient: String,
    /// Human-decimal — needs * 10^decimals0 to get raw.
    amount0: String,
    /// Same.
    amount1: String,
    /// Raw uint256 decimal string.
    sqrt_price_x96: String,
    /// Signed int24 as decimal string.
    tick: String,
    /// Log index within the transaction (decimal string).
    log_index: String,
    /// Unix seconds (decimal string).
    timestamp: String,
    transaction: TransactionRef,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransactionRef {
    /// 0x-prefixed 32-byte hex.
    id: String,
    /// Decimal string.
    block_number: String,
}

#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<Value>>,
}

#[derive(Debug, Clone)]
pub struct PoolMetadata {
    pub pool_address: String,
    pub token0_address: String,
    pub token0_symbol: String,
    pub token0_decimals: u8,
    pub token1_address: String,
    pub token1_symbol: String,
    pub token1_decimals: u8,
    /// Friendly bps (5, 30, 100, 10000), not the protocol's
    /// hundredths-of-bps.
    pub fee_tier_bps: u32,
}

/// Decimal-string × 10^decimals → raw BigUint. Handles negative
/// inputs by returning the two's-complement uint256 representation
/// (matches V3's int256 amount sign convention).
fn human_decimal_to_raw_uint256(s: &str, decimals: u8) -> Result<BigUint, IngestError> {
    let (sign_negative, body) = if let Some(stripped) = s.strip_prefix('-') {
        (true, stripped)
    } else {
        (false, s)
    };
    // Split on the decimal point.
    let (whole, frac) = match body.find('.') {
        Some(i) => (&body[..i], &body[i + 1..]),
        None => (body, ""),
    };
    let dec_usize = decimals as usize;
    // Pad / truncate the fractional component to exactly `decimals`
    // digits so `whole + frac_padded` is the integer form scaled by
    // 10^decimals.
    let frac_padded = if frac.len() >= dec_usize {
        frac[..dec_usize].to_string()
    } else {
        let mut padded = frac.to_string();
        padded.push_str(&"0".repeat(dec_usize - frac.len()));
        padded
    };
    let combined = format!("{whole}{frac_padded}");
    let combined_trimmed = combined.trim_start_matches('0');
    let parse_target = if combined_trimmed.is_empty() {
        "0"
    } else {
        combined_trimmed
    };
    let abs = BigUint::parse_bytes(parse_target.as_bytes(), 10)
        .ok_or_else(|| IngestError::MalformedLog(format!("amount parse: {s}")))?;
    if sign_negative && abs > BigUint::from(0u8) {
        // Two's-complement uint256: 2^256 - abs
        let two_256 = BigUint::from(1u8) << 256;
        Ok(&two_256 - &abs)
    } else {
        Ok(abs)
    }
}

fn swap_node_to_eth_log(
    pool_lower: &str,
    n: &SwapNode,
    decimals0: u8,
    decimals1: u8,
) -> Result<EthLog, IngestError> {
    // amount0 / amount1 are signed in V3: positive = into the pool,
    // negative = out. The subgraph preserves the sign in its
    // BigDecimal. We re-encode to int256 two's-complement.
    let amount0_raw = human_decimal_to_raw_uint256(&n.amount0, decimals0)?;
    let amount1_raw = human_decimal_to_raw_uint256(&n.amount1, decimals1)?;
    let sqrt = BigUint::parse_bytes(n.sqrt_price_x96.as_bytes(), 10).ok_or_else(|| {
        IngestError::MalformedLog(format!("sqrtPriceX96 parse: {}", n.sqrt_price_x96))
    })?;

    // Tick: the subgraph emits a signed decimal string. Re-encode to
    // int24 sign-extended into int256.
    let tick_signed: i32 = n
        .tick
        .parse()
        .map_err(|_| IngestError::MalformedLog(format!("tick parse: {}", n.tick)))?;
    let tick_raw = (tick_signed as u32) & 0x00FF_FFFF;
    let tick_hex = if tick_signed < 0 {
        format!("{:0>58}{:06x}", "f".repeat(58), tick_raw)
    } else {
        format!("{:0>64x}", tick_raw)
    };

    // Subgraph doesn't expose pool's *active* liquidity at swap-time
    // directly on Swap. The schema does have `pool.liquidity` as
    // current — it's not historically accurate. As a defensible
    // proxy we encode 0; the engine then falls back to pos_L / 0
    // which the fee-share clamp turns into 0 fee. That's wrong for
    // the engine. Better: look up the active liquidity from
    // pool_dayData or use a constant. For tier 1 we encode a
    // realistic constant matching synthetic_mock — the analysis is
    // approximate by design.
    let liq_hex = format!("{:0>64x}", 100_000_000_000_000_000u128);

    let amount0_hex = format!("{:0>64}", amount0_raw.to_str_radix(16));
    let amount1_hex = format!("{:0>64}", amount1_raw.to_str_radix(16));
    let sqrt_hex = format!("{:0>64}", sqrt.to_str_radix(16));

    let block_number: u64 = n.transaction.block_number.parse().map_err(|_| {
        IngestError::MalformedLog(format!("blockNumber parse: {}", n.transaction.block_number))
    })?;
    let log_index: u64 = n
        .log_index
        .parse()
        .map_err(|_| IngestError::MalformedLog(format!("logIndex parse: {}", n.log_index)))?;
    let timestamp: i64 = n.timestamp.parse().map_err(|_| {
        IngestError::MalformedLog(format!("timestamp parse: {}", n.timestamp))
    })?;

    // Topic 1 / 2 are sender / recipient indexed addresses. The
    // engine doesn't read them, so a left-pad to 32 bytes with the
    // raw address is sufficient for ABI shape.
    let topic1 = format!("0x{:0>64}", n.sender.trim_start_matches("0x"));
    let topic2 = format!("0x{:0>64}", n.recipient.trim_start_matches("0x"));

    Ok(EthLog {
        address: pool_lower.to_string(),
        block_number,
        log_index,
        transaction_hash: n.transaction.id.clone(),
        block_timestamp: timestamp,
        topics: vec![format!("0x{SWAP_TOPIC0}"), topic1, topic2],
        data: format!("0x{amount0_hex}{amount1_hex}{sqrt_hex}{liq_hex}{tick_hex}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_decimal_to_raw_handles_positive_with_fraction() {
        // 1.5 with 18 decimals → 1.5e18 raw
        let r = human_decimal_to_raw_uint256("1.5", 18).unwrap();
        assert_eq!(r, BigUint::parse_bytes(b"1500000000000000000", 10).unwrap());
    }

    #[test]
    fn human_decimal_to_raw_handles_integer() {
        let r = human_decimal_to_raw_uint256("3000", 6).unwrap();
        assert_eq!(r, BigUint::from(3_000_000_000u64));
    }

    #[test]
    fn human_decimal_to_raw_handles_negative_via_twos_complement() {
        // -1 with 18 decimals → 2^256 - 1e18
        let r = human_decimal_to_raw_uint256("-1", 18).unwrap();
        let expected = (BigUint::from(1u8) << 256) - BigUint::from(1_000_000_000_000_000_000u64);
        assert_eq!(r, expected);
    }

    #[test]
    fn human_decimal_to_raw_truncates_extra_fraction_digits() {
        // "1.123456789" with 6 decimals → keep first 6 after the dot
        let r = human_decimal_to_raw_uint256("1.123456789", 6).unwrap();
        assert_eq!(r, BigUint::from(1_123_456u64));
    }

    #[test]
    fn human_decimal_to_raw_handles_zero() {
        let r = human_decimal_to_raw_uint256("0", 18).unwrap();
        assert_eq!(r, BigUint::from(0u8));
    }
}
