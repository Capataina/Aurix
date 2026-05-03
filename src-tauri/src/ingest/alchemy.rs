//! Alchemy archive RPC source.
//!
//! Live data ingestion via JSON-RPC. The free tier caps `eth_getLogs`
//! ranges to 10 blocks; this adapter enforces the cap and exposes the
//! batched fetch via the `ArchiveSource` trait. PAYG (~$1 per 30-day
//! backfill, ~13 sec) does NOT have the cap and is recommended for the
//! M2.4 validation runs (per
//! `context/references/ethereum-archive-log-ingestion.md`).
//!
//! KEY_REQUIRED — instantiating this source needs a working `MAINNET_RPC_URL`
//! that resolves to an archive node, OR an `ALCHEMY_API_KEY` env var that
//! `AppConfig::from_environment()` constructs the URL from. Without one,
//! `AlchemyArchiveSource::from_environment()` returns `KeyRequired`.

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::config::AppConfig;
use crate::ethereum::client::EthereumRpcClient;

use super::error::IngestError;
use super::source::{ArchiveSource, EthLog};

/// Maximum block range per `eth_getLogs` call on the Alchemy free tier.
/// PAYG is unbounded; conservative default keeps the free tier happy.
pub const FREE_TIER_LOG_RANGE_CAP: u64 = 10;

pub struct AlchemyArchiveSource {
    client: EthereumRpcClient,
    /// When `true`, splits log queries into <=10-block chunks. Toggleable
    /// via `with_payg_unbounded(false)` for free-tier safety.
    payg_unbounded: bool,
}

impl AlchemyArchiveSource {
    /// Builds an archive source from the existing app config. Returns
    /// `KeyRequired` if neither `MAINNET_RPC_URL` nor `ALCHEMY_API_KEY` is
    /// set. The caller is responsible for catching this error and either
    /// falling back to mock fixtures (tests) or surfacing to the user.
    pub fn from_environment() -> Result<Self, IngestError> {
        let cfg = AppConfig::from_environment()
            .map_err(|_| IngestError::KeyRequired("MAINNET_RPC_URL or ALCHEMY_API_KEY"))?;
        let client = EthereumRpcClient::new(cfg.ethereum_mainnet_rpc_url());
        Ok(Self {
            client,
            payg_unbounded: false,
        })
    }

    /// Disable the free-tier 10-block chunking. Set this when running
    /// against PAYG keys; the backfill becomes ~13 seconds instead of
    /// ~50 minutes for a 30-day window.
    pub fn with_payg_unbounded(mut self, payg_unbounded: bool) -> Self {
        self.payg_unbounded = payg_unbounded;
        self
    }

    fn chunk_size(&self) -> u64 {
        if self.payg_unbounded {
            // 10k is a generous PAYG limit; if Alchemy's hard ceiling is
            // higher we still fall well below it.
            10_000
        } else {
            FREE_TIER_LOG_RANGE_CAP
        }
    }
}

#[async_trait]
impl ArchiveSource for AlchemyArchiveSource {
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
        let chunk = self.chunk_size();
        let mut out = Vec::new();
        let mut start = from_block;
        while start <= to_block {
            let end = (start + chunk - 1).min(to_block);
            let topics_json: Vec<Value> = topic0_filters
                .iter()
                .map(|t| json!(format!("0x{}", t.trim_start_matches("0x"))))
                .collect();
            let topic0_value = if topics_json.is_empty() {
                Value::Null
            } else if topics_json.len() == 1 {
                topics_json[0].clone()
            } else {
                Value::Array(topics_json)
            };
            let params = json!([{
                "address": pool_address,
                "fromBlock": format!("0x{:x}", start),
                "toBlock": format!("0x{:x}", end),
                "topics": [topic0_value]
            }]);
            let raw: Value = self.client.rpc_call("eth_getLogs", params).await?;
            let arr = raw.as_array().ok_or_else(|| {
                IngestError::MalformedLog("eth_getLogs returned non-array".into())
            })?;
            for entry in arr {
                let log = parse_eth_log(entry)?;
                out.push(log);
            }
            start = end + 1;
        }
        // Re-fetch block timestamps if the eth_getLogs response didn't
        // include them (most providers don't; we resolve via getBlockByNumber).
        for log in out.iter_mut() {
            if log.block_timestamp == 0 {
                log.block_timestamp = self.block_timestamp(log.block_number).await?;
            }
        }
        Ok(out)
    }

    async fn latest_finalized_block(&self) -> Result<u64, IngestError> {
        let params = json!(["finalized", false]);
        let raw: Value = self.client.rpc_call("eth_getBlockByNumber", params).await?;
        let number_hex = raw
            .get("number")
            .and_then(|v| v.as_str())
            .ok_or(IngestError::NoResult("eth_getBlockByNumber.number"))?;
        let n = parse_hex_u64(number_hex)?;
        Ok(n)
    }

    async fn block_median_gas_gwei(&self, block: u64) -> Result<f64, IngestError> {
        // Approximation: use the block's baseFeePerGas (post-EIP-1559) as
        // the proxy for median gas. Honest enough for mgmt-gas modelling
        // (per plan paper 2 §gas economics); the alternative is a full
        // tx receipt scan which is expensive.
        let params = json!([format!("0x{:x}", block), false]);
        let raw: Value = self.client.rpc_call("eth_getBlockByNumber", params).await?;
        let base_fee_hex = raw
            .get("baseFeePerGas")
            .and_then(|v| v.as_str())
            .ok_or(IngestError::NoResult("eth_getBlockByNumber.baseFeePerGas"))?;
        let wei = parse_hex_u128(base_fee_hex)?;
        Ok(wei as f64 / 1e9)
    }

    async fn block_timestamp(&self, block: u64) -> Result<i64, IngestError> {
        let params = json!([format!("0x{:x}", block), false]);
        let raw: Value = self.client.rpc_call("eth_getBlockByNumber", params).await?;
        let ts_hex = raw
            .get("timestamp")
            .and_then(|v| v.as_str())
            .ok_or(IngestError::NoResult("eth_getBlockByNumber.timestamp"))?;
        Ok(parse_hex_u64(ts_hex)? as i64)
    }
}

fn parse_eth_log(entry: &Value) -> Result<EthLog, IngestError> {
    let address = entry
        .get("address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| IngestError::MalformedLog("missing address".into()))?
        .to_string();
    let block_number = parse_hex_u64(
        entry
            .get("blockNumber")
            .and_then(|v| v.as_str())
            .ok_or_else(|| IngestError::MalformedLog("missing blockNumber".into()))?,
    )?;
    let log_index = parse_hex_u64(
        entry
            .get("logIndex")
            .and_then(|v| v.as_str())
            .ok_or_else(|| IngestError::MalformedLog("missing logIndex".into()))?,
    )?;
    let transaction_hash = entry
        .get("transactionHash")
        .and_then(|v| v.as_str())
        .ok_or_else(|| IngestError::MalformedLog("missing transactionHash".into()))?
        .to_string();
    let topics: Vec<String> = entry
        .get("topics")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let data = entry
        .get("data")
        .and_then(|v| v.as_str())
        .unwrap_or("0x")
        .to_string();
    Ok(EthLog {
        address,
        block_number,
        log_index,
        transaction_hash,
        block_timestamp: 0, // populated by the caller via block_timestamp()
        topics,
        data,
    })
}

fn parse_hex_u64(s: &str) -> Result<u64, IngestError> {
    let s = s.trim_start_matches("0x");
    u64::from_str_radix(s, 16).map_err(|_| IngestError::MalformedLog(format!("bad hex u64: {s}")))
}

fn parse_hex_u128(s: &str) -> Result<u128, IngestError> {
    let s = s.trim_start_matches("0x");
    u128::from_str_radix(s, 16).map_err(|_| IngestError::MalformedLog(format!("bad hex u128: {s}")))
}
