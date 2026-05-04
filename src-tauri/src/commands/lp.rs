//! Tauri commands exposing the Vector A backend to the frontend.
//!
//! State management: a single `Arc<Storage>` is registered via
//! `tauri::Builder::default().manage(...)` at startup; every command
//! reads the handle through `tauri::State`.
//!
//! KEY_REQUIRED hooks: live ingestion + ETH.STORE benchmark fetches
//! return a `KeyRequired` error string when no key is configured. The
//! frontend can surface a "configure your key" prompt.

use std::sync::Arc;

use once_cell::sync::Lazy;
use serde::Serialize;
use tauri::State;

use crate::backtest::{Engine, PositionConfig, RebalanceRule, SimulationOutput};
use crate::benchmarks::{
    AAVE_V3_USDC_SUPPLY_POOL, COMPOUND_V3_USDC_SUPPLY_POOL, DefiLlamaProvider,
    LIDO_STETH_POOL, ReqwestFetcher, TradFiProvider,
    FRED_DGS3MO_URL, FRED_GOLD_LBMA_URL, FRED_SP500_URL,
};
use crate::config::chains::Protocol;
use crate::config::ChainId;
use crate::backtest::price::sqrt_price_x96_to_human_price;
use crate::math::sqrt_price_x96_to_tick;
use num_bigint::BigUint;
use crate::headline::{HeadlineConfig, HeadlineOutput, HeadlineRunner};
use crate::ingest::{
    AlchemyArchiveSource, ArchiveSource, AttemptedSource, IngestError, IngestionReport, Ingester,
    MockArchiveSource, PoolMetadata as IngestPoolMetadata, UniswapV3SubgraphSource,
};
use crate::storage::{
    benchmarks::BenchmarkPoint,
    headline::HeadlineMonthlyRow,
    runs::{EquityCurvePoint, PositionRunSummary},
    strategy::StrategyResultRow,
    swaps::SYNTHETIC_TX_HASH,
    Storage,
};
use crate::strategies::{GridConfig, GridRunner};

/// Process-wide HTTP client for the DefiLlama token-prices endpoint.
/// Constructed once on first use; subsequent calls reuse the underlying
/// connection pool (audit finding `code-health-audit/ipc-commands.md`
/// §"Reuse the reqwest::Client").
static TOKEN_PRICE_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("default reqwest::Client builder cannot fail")
});

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub message: String,
    pub key_required: Option<String>,
}

impl<E: std::fmt::Display> From<E> for CommandError {
    fn from(e: E) -> Self {
        Self {
            message: e.to_string(),
            key_required: None,
        }
    }
}

fn map_ingest_error(e: IngestError) -> CommandError {
    match e {
        IngestError::KeyRequired(name) => CommandError {
            message: format!("api key required: {name}"),
            key_required: Some(name.to_string()),
        },
        other => CommandError {
            message: other.to_string(),
            key_required: None,
        },
    }
}

/// Returns the latest finalized block on Ethereum mainnet. Used by the
/// frontend to default the LP backtester's block window to "the last N
/// blocks" instead of an arbitrary fixed range. Free of cost — just one
/// `eth_getBlockByNumber("finalized")` call.
///
/// Tries user's Alchemy first (mainnet only); on miss / error falls
/// back to the chain's free public RPC so the dashboard always has a
/// live chain head even without API keys configured.
#[allow(dead_code)] // referenced via tauri::generate_handler! macro
#[tauri::command]
pub async fn lp_get_chain_head(chain_id: Option<String>) -> Result<u64, CommandError> {
    let chain = parse_chain(chain_id.as_deref());
    if matches!(chain, ChainId::Ethereum) {
        if let Ok(src) = AlchemyArchiveSource::from_environment() {
            if let Ok(head) = src.latest_finalized_block().await {
                return Ok(head);
            }
        }
    }
    let public = AlchemyArchiveSource::with_rpc_url(chain.public_rpc_url());
    public
        .latest_finalized_block()
        .await
        .map_err(map_ingest_error)
}

fn parse_chain(chain_id: Option<&str>) -> ChainId {
    chain_id
        .and_then(ChainId::from_str)
        .unwrap_or(ChainId::Ethereum)
}

fn parse_protocol(protocol: Option<&str>) -> Protocol {
    protocol
        .and_then(Protocol::from_str)
        .unwrap_or(Protocol::UniswapV3)
}

/// Backfills the supplied block range for a pool on the supplied
/// chain. Tiered fallback designed to work without any API keys:
///
///   1. Uniswap V3 subgraph (chain-specific URL) — fast.
///   2. User's configured Alchemy key, when set in env (mainnet only).
///   3. Free public RPC for the chain — chunked eth_getLogs.
///
/// On total failure, surfaces an error to the caller — we deliberately
/// do not fall back to synthetic data in user-facing flows. Empty/error
/// state is a more honest signal than fabricated numbers.
///
/// The successful report's `source_label` records which tier ultimately
/// succeeded; `attempted_sources` carries any tiers that errored along
/// the way so the frontend can show a "subgraph failed → fell through
/// to public RPC" status without resorting to stderr (audit finding
/// `code-health-audit/ipc-commands.md` §"Inconsistent Patterns").
#[tauri::command]
pub async fn run_lp_ingestion(
    storage: State<'_, Arc<Storage>>,
    pool_address: String,
    from_block: u64,
    to_block: u64,
    chain_id: Option<String>,
    protocol: Option<String>,
) -> Result<IngestionReport, CommandError> {
    let chain = parse_chain(chain_id.as_deref());
    let proto = parse_protocol(protocol.as_deref());
    let mut attempted: Vec<AttemptedSource> = Vec::new();

    // Path 1 — subgraph (chain + protocol specific).
    let subgraph: Arc<dyn ArchiveSource> =
        Arc::new(UniswapV3SubgraphSource::for_protocol(chain, proto));
    let ingester = Ingester::new((**storage).clone(), subgraph);
    match ingester.backfill(&pool_address, from_block, to_block).await {
        Ok(mut report) => {
            report.source_label = Some(format!("subgraph:{}", chain.label()));
            return Ok(report);
        }
        Err(e) => {
            attempted.push(AttemptedSource {
                source_label: format!("subgraph:{}", chain.label()),
                error: e.to_string(),
            });
        }
    }
    // Path 2 — user's Alchemy key, if configured. Only meaningful on
    // mainnet given how AppConfig resolves the URL today.
    if matches!(chain, ChainId::Ethereum) {
        if let Ok(alchemy) = AlchemyArchiveSource::from_environment() {
            let source: Arc<dyn ArchiveSource> = Arc::new(alchemy.with_payg_unbounded(true));
            let ingester = Ingester::new((**storage).clone(), source);
            match ingester.backfill(&pool_address, from_block, to_block).await {
                Ok(mut report) => {
                    report.source_label = Some("alchemy:ethereum".to_string());
                    report.attempted_sources = attempted;
                    return Ok(report);
                }
                Err(e) => {
                    attempted.push(AttemptedSource {
                        source_label: "alchemy:ethereum".to_string(),
                        error: e.to_string(),
                    });
                }
            }
        }
    }
    // Path 3 — free public RPC for the chain. The adaptive chunk-
    // size logic in get_pool_logs handles tier-specific range caps.
    let public: Arc<dyn ArchiveSource> = Arc::new(
        AlchemyArchiveSource::with_rpc_url(chain.public_rpc_url()).with_payg_unbounded(true),
    );
    let ingester = Ingester::new((**storage).clone(), public);
    let mut report = ingester
        .backfill(&pool_address, from_block, to_block)
        .await
        .map_err(map_ingest_error)?;
    report.source_label = Some(format!("public-rpc:{}", chain.label()));
    report.attempted_sources = attempted;
    Ok(report)
}

/// Returns token0/token1 metadata for a pool — addresses, symbols,
/// decimals, fee tier. Frontend uses this to display human pool
/// names + drive the deposit-split math without hardcoded decimals.
#[tauri::command]
pub async fn lp_pool_metadata(
    pool_address: String,
    chain_id: Option<String>,
    protocol: Option<String>,
) -> Result<PoolMetadataDto, CommandError> {
    let chain = parse_chain(chain_id.as_deref());
    let proto = parse_protocol(protocol.as_deref());
    let source = UniswapV3SubgraphSource::for_protocol(chain, proto);
    let meta = source
        .pool_metadata(&pool_address)
        .await
        .map_err(map_ingest_error)?;
    Ok(meta.into())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolMetadataDto {
    pub pool_address: String,
    pub token0_address: String,
    pub token0_symbol: String,
    pub token0_decimals: u8,
    pub token1_address: String,
    pub token1_symbol: String,
    pub token1_decimals: u8,
    pub fee_tier_bps: u32,
    /// Heuristic — true when token1's symbol matches a known USD-pegged
    /// stablecoin. Frontend uses this to default the deposit split's
    /// quote-token assumption; tier 4 generalises this.
    pub is_token1_usd_pegged: bool,
}

impl From<IngestPoolMetadata> for PoolMetadataDto {
    fn from(m: IngestPoolMetadata) -> Self {
        let is_token1_usd_pegged = is_usd_stable(&m.token1_symbol);
        Self {
            pool_address: m.pool_address,
            token0_address: m.token0_address,
            token0_symbol: m.token0_symbol,
            token0_decimals: m.token0_decimals,
            token1_address: m.token1_address,
            token1_symbol: m.token1_symbol,
            token1_decimals: m.token1_decimals,
            fee_tier_bps: m.fee_tier_bps,
            is_token1_usd_pegged,
        }
    }
}

fn is_usd_stable(symbol: &str) -> bool {
    matches!(
        symbol.to_uppercase().as_str(),
        "USDC" | "USDT" | "DAI" | "FRAX" | "LUSD" | "USDS" | "USDC.E" | "GUSD" | "BUSD" | "TUSD"
    )
}

/// Backfills using a fixture/mock source — used when no Alchemy key is
/// configured and the user clicks "ingest a synthetic dataset" in the
/// GUI to demo the rest of the pipeline. Generates a sinusoidal swap
/// stream over the requested block range.
///
/// Wipes any prior synthetic rows in the range before inserting fresh
/// ones. Without this, a tweak to the synthetic generator (e.g. tick
/// anchor change) wouldn't take effect on overlapping ranges because
/// `INSERT OR IGNORE` would silently keep the stale rows. Live data
/// is identified by a real tx hash and is never touched.
#[tauri::command]
pub async fn run_lp_synthetic_ingest(
    storage: State<'_, Arc<Storage>>,
    pool_address: String,
    from_block: u64,
    to_block: u64,
) -> Result<IngestionReport, CommandError> {
    if from_block > to_block {
        return Err(CommandError {
            message: "from_block > to_block".into(),
            key_required: None,
        });
    }
    let _purged = storage
        .delete_synthetic_swaps_in_range(pool_address.clone(), from_block, to_block)
        .await
        .map_err(|e| CommandError {
            message: format!("synthetic purge failed: {e}"),
            key_required: None,
        })?;
    let mock = synthetic_mock(&pool_address, from_block, to_block);
    let ingester = Ingester::new((**storage).clone(), mock);
    ingester
        .backfill(&pool_address, from_block, to_block)
        .await
        .map_err(map_ingest_error)
}

/// Returns the synthetic-data tick anchor used as the centre of the
/// sinusoidal walk. This is **internal** to the synthetic dataset —
/// nothing downstream depends on knowing it. The frontend reads the
/// realised first-swap tick + price from the DB and adapts accordingly.
///
/// The value `-195_580` corresponds to ~3000 USDC per WETH given the
/// pool's WETH(18)/USDC(6) layout, but the rest of the system is
/// agnostic to which value is chosen.
fn synthetic_anchor_tick() -> i32 {
    -195_580
}

fn synthetic_mock(pool: &str, from: u64, to: u64) -> Arc<MockArchiveSource> {
    use num_bigint::BigUint;

    use crate::ingest::decoder::SWAP_TOPIC0;
    use crate::ingest::EthLog;
    use crate::math::tick_to_sqrt_price_x96;

    let mock = Arc::new(MockArchiveSource::new(to + 100));
    let anchor = synthetic_anchor_tick();
    let two_192: BigUint = BigUint::from(1u8) << 192;
    let two_256: BigUint = BigUint::from(1u8) << 256;
    let amount0_raw = BigUint::from(1_000_000_000_000_000_000u64); // 1 token0
    for b in from..=to {
        // Smooth ±300-tick sinusoidal walk (~±3% on price) around the
        // anchor. The frontend's default position range is realised-
        // entry-tick ± 300 — derived from the first swap, not from
        // anything baked in here.
        let phase = ((b - from) as f64 / (to - from + 1) as f64) * std::f64::consts::TAU;
        let tick = anchor + (phase.sin() * 300.0) as i32;
        let sqrt = match tick_to_sqrt_price_x96(tick) {
            Ok(s) => s,
            Err(_) => continue,
        };
        // Derive amount1 from the tick's implied price so each swap is
        // internally self-consistent: a swap of `amount0_raw` token0
        // crosses the pool at exactly the spot price the tick encodes.
        //   raw_price (token1/token0) = sqrt^2 / 2^192
        //   amount1 = amount0 × raw_price
        let amount1_implied = (&sqrt * &sqrt * &amount0_raw) / &two_192;
        let amount1_neg = &two_256 - &amount1_implied;

        let topic0 = format!("0x{SWAP_TOPIC0}");
        let topic1 = format!("0x{:0>64}", "1");
        let topic2 = format!("0x{:0>64}", "2");
        let amount0_hex = format!("{:0>64}", amount0_raw.to_str_radix(16));
        let amount1_hex = format!("{:0>64}", amount1_neg.to_str_radix(16));
        let sqrt_hex = format!("{:0>64}", sqrt.to_str_radix(16));
        // Active pool liquidity. Sized so a typical $10k position is a
        // few-percent share of the pool — keeping fee_share = pos_L /
        // active_L well below 1 (which is the invariant real V3 pools
        // maintain by construction). With 1e12 the previous synthetic
        // generator over-attributed fees by ~5000× because position L
        // for a $10k stake at ±3% comes in at ~5.6e15. 1e17 puts the
        // position at ~5.6% of pool, making demo fees realistic.
        let liq_hex = format!("{:0>64x}", 100_000_000_000_000_000u128);
        let raw = (tick as u32) & 0x00FF_FFFF;
        let tick_hex = if tick < 0 {
            format!("{:0>58}{:06x}", "f".repeat(58), raw)
        } else {
            format!("{:0>64x}", raw)
        };
        let log = EthLog {
            address: pool.to_string(),
            block_number: b,
            log_index: 0,
            transaction_hash: SYNTHETIC_TX_HASH.to_string(),
            block_timestamp: 1_700_000_000 + (b as i64) * 12,
            topics: vec![topic0, topic1, topic2],
            data: format!("0x{amount0_hex}{amount1_hex}{sqrt_hex}{liq_hex}{tick_hex}"),
        };
        mock.add_log(log);
        mock.set_block_gas(b, 20.0);
    }
    mock
}

/// Runs a single LP backtest. Persists to storage and returns the run
/// summary + equity curve.
#[tauri::command]
pub async fn run_lp_backtest(
    storage: State<'_, Arc<Storage>>,
    config: PositionConfig,
    rule: RebalanceRule,
) -> Result<BacktestResponse, CommandError> {
    let engine = Engine::new(&storage);
    let out: SimulationOutput = engine.simulate(config, rule).await?;
    storage
        .persist_position_run(out.summary.clone(), out.equity_curve.clone())
        .await?;
    Ok(BacktestResponse {
        summary: out.summary,
        equity_curve: out.equity_curve,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestResponse {
    pub summary: PositionRunSummary,
    pub equity_curve: Vec<EquityCurvePoint>,
}

/// Runs the strategy comparison grid.
#[tauri::command]
pub async fn run_lp_grid(
    storage: State<'_, Arc<Storage>>,
    config: GridConfig,
) -> Result<Vec<StrategyResultRow>, CommandError> {
    let runner = GridRunner::new(&storage);
    Ok(runner.run_grid(config).await?)
}

/// Runs the M2.8 capital-allocation headline analysis.
#[tauri::command]
pub async fn run_lp_headline(
    storage: State<'_, Arc<Storage>>,
    config: HeadlineConfig,
) -> Result<HeadlineOutput, CommandError> {
    let runner = HeadlineRunner::new(&storage);
    Ok(runner.run(config).await?)
}

/// Returns the equity curve for a previously-persisted run.
#[tauri::command]
pub async fn lp_get_equity_curve(
    storage: State<'_, Arc<Storage>>,
    config_hash: String,
) -> Result<Vec<EquityCurvePoint>, CommandError> {
    Ok(storage.get_equity_curve(config_hash).await?)
}

/// Returns the persisted strategy grid results for a grid id.
#[tauri::command]
pub async fn lp_query_strategies(
    storage: State<'_, Arc<Storage>>,
    grid_id: String,
) -> Result<Vec<StrategyResultRow>, CommandError> {
    Ok(storage.query_strategy_results(grid_id).await?)
}

/// Returns persisted headline monthly rows for a config hash.
#[tauri::command]
pub async fn lp_query_headline_monthly(
    storage: State<'_, Arc<Storage>>,
    config_hash: String,
) -> Result<Vec<HeadlineMonthlyRow>, CommandError> {
    Ok(storage.get_headline_monthly(config_hash).await?)
}

/// Fetches a benchmark series live (DefiLlama / FRED / Stooq) and
/// persists into storage::benchmark_series. Returns the points written.
#[tauri::command]
pub async fn lp_fetch_benchmark_series(
    storage: State<'_, Arc<Storage>>,
    series_key: String,
) -> Result<Vec<BenchmarkPoint>, CommandError> {
    let fetcher = ReqwestFetcher::default();
    let points: Vec<BenchmarkPoint> = match series_key.as_str() {
        "aave_v3_usdc_supply_apy" => {
            let p = DefiLlamaProvider::new(&fetcher);
            p.fetch_pool_apy(AAVE_V3_USDC_SUPPLY_POOL, &series_key)
                .await?
        }
        "compound_v3_usdc_supply_apy" => {
            let p = DefiLlamaProvider::new(&fetcher);
            p.fetch_pool_apy(COMPOUND_V3_USDC_SUPPLY_POOL, &series_key)
                .await?
        }
        "lido_steth_apy" => {
            let p = DefiLlamaProvider::new(&fetcher);
            p.fetch_pool_apy(LIDO_STETH_POOL, &series_key).await?
        }
        "fred_dgs3mo" => {
            let p = TradFiProvider::new(&fetcher);
            p.fetch_fred(FRED_DGS3MO_URL, &series_key).await?
        }
        "stooq_voo" => {
            // Legacy series key kept for back-compat. Stooq now gates
            // behind an API key; transparently route to FRED's SP500
            // series (no key required, same shape post-parse).
            let p = TradFiProvider::new(&fetcher);
            p.fetch_fred(FRED_SP500_URL, &series_key).await?
        }
        "fred_sp500" => {
            let p = TradFiProvider::new(&fetcher);
            p.fetch_fred(FRED_SP500_URL, &series_key).await?
        }
        "fred_gold_lbma" => {
            let p = TradFiProvider::new(&fetcher);
            p.fetch_fred(FRED_GOLD_LBMA_URL, &series_key).await?
        }
        other => {
            return Err(CommandError {
                message: format!("unsupported benchmark series: {other}"),
                key_required: None,
            });
        }
    };
    if !points.is_empty() {
        storage
            .insert_benchmark_points_batch(points.clone())
            .await?;
    }
    Ok(points)
}

/// Returns benchmark series rows persisted in storage.
#[tauri::command]
pub async fn lp_query_benchmark_range(
    storage: State<'_, Arc<Storage>>,
    series_key: String,
    start_date: String,
    end_date: String,
) -> Result<Vec<BenchmarkPoint>, CommandError> {
    Ok(storage
        .query_benchmark_range(series_key, start_date, end_date)
        .await?)
}

/// Returns the first swap (chronologically) inside `[from_block, to_block]`
/// for `pool_address`, decoded into the realised tick + sqrtPriceX96 +
/// the human-decimal price implied at the supplied decimals.
///
/// The frontend uses this immediately after ingestion to derive the
/// position tick range and deposit split from actual data instead of
/// hardcoded constants. Returns `None` if the range is empty.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstSwapInfo {
    pub block_number: i64,
    pub block_timestamp: i64,
    pub tick: i32,
    pub sqrt_price_x96: String,
    pub human_price: f64,
}

#[tauri::command]
pub async fn lp_query_first_swap_price(
    storage: State<'_, Arc<Storage>>,
    pool_address: String,
    from_block: i64,
    to_block: i64,
    token0_decimals: u8,
    token1_decimals: u8,
) -> Result<Option<FirstSwapInfo>, CommandError> {
    let swaps = storage
        .query_swaps_for_pool_range(pool_address, from_block, to_block)
        .await?;
    let Some(first) = swaps.into_iter().next() else {
        return Ok(None);
    };
    let sqrt = BigUint::parse_bytes(first.sqrt_price_x96.as_bytes(), 10).ok_or_else(|| {
        CommandError {
            message: format!("malformed sqrt_price_x96: {}", first.sqrt_price_x96),
            key_required: None,
        }
    })?;
    // Prefer the swap row's stored tick; fall back to deriving from sqrt.
    let tick = if first.tick != 0 {
        first.tick
    } else {
        sqrt_price_x96_to_tick(&sqrt).map_err(|e| CommandError {
            message: e.to_string(),
            key_required: None,
        })?
    };
    let human_price =
        sqrt_price_x96_to_human_price(&sqrt, token0_decimals, token1_decimals);
    Ok(Some(FirstSwapInfo {
        block_number: first.block_number,
        block_timestamp: first.block_timestamp,
        tick,
        sqrt_price_x96: first.sqrt_price_x96,
        human_price,
    }))
}

/// Returns USD spot prices for the supplied tokens via DefiLlama's
/// free coins API. Used by the frontend to value non-USD-quote pool
/// positions (WBTC/ETH, LDO/ETH, etc.) where neither token is a
/// stablecoin.
///
/// API: `https://coins.llama.fi/prices/current/<chain>:<addr>,<chain>:<addr2>`
/// returns `{ coins: { "<chain>:<addr>": { price, symbol, ... } } }`.
/// No auth required. Generous rate limits.
#[tauri::command]
pub async fn lp_token_usd_prices(
    chain_id: Option<String>,
    addresses: Vec<String>,
) -> Result<TokenPricesDto, CommandError> {
    if addresses.is_empty() {
        return Ok(TokenPricesDto::default());
    }
    let chain = parse_chain(chain_id.as_deref());
    let chain_label = match chain {
        ChainId::Ethereum => "ethereum",
        ChainId::Arbitrum => "arbitrum",
        ChainId::Optimism => "optimism",
        ChainId::Base => "base",
        ChainId::Polygon => "polygon",
    };
    let joined = addresses
        .iter()
        .map(|a| format!("{chain_label}:{}", a.to_lowercase()))
        .collect::<Vec<_>>()
        .join(",");
    let url = format!("https://coins.llama.fi/prices/current/{joined}");
    let resp = TOKEN_PRICE_CLIENT
        .get(&url)
        .send()
        .await
        .map_err(|e| CommandError {
            message: format!("token prices: transport: {e}"),
            key_required: None,
        })?;
    if !resp.status().is_success() {
        return Err(CommandError {
            message: format!("token prices: http {}", resp.status()),
            key_required: None,
        });
    }
    let body: DefiLlamaPricesResponse = resp.json().await.map_err(|e| CommandError {
        message: format!("token prices: parse: {e}"),
        key_required: None,
    })?;
    let mut prices = std::collections::HashMap::new();
    for (key, entry) in body.coins {
        // Key is "chain:addr" — extract the address.
        if let Some(addr) = key.split(':').nth(1) {
            prices.insert(addr.to_lowercase(), entry.price);
        }
    }
    Ok(TokenPricesDto { prices })
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenPricesDto {
    /// Lowercase-address → USD price, in human dollars.
    pub prices: std::collections::HashMap<String, f64>,
}

#[derive(Debug, serde::Deserialize)]
struct DefiLlamaPricesResponse {
    coins: std::collections::HashMap<String, DefiLlamaPriceEntry>,
}

#[derive(Debug, serde::Deserialize)]
struct DefiLlamaPriceEntry {
    price: f64,
}
