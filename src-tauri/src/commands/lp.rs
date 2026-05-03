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

use serde::Serialize;
use tauri::State;

use crate::backtest::{Engine, PositionConfig, RebalanceRule, SimulationOutput};
use crate::benchmarks::{
    AAVE_V3_USDC_SUPPLY_POOL, COMPOUND_V3_USDC_SUPPLY_POOL, DefiLlamaProvider,
    LIDO_STETH_POOL, MockHttpFetcher, ReqwestFetcher, TradFiProvider,
    FRED_DGS3MO_URL, STOOQ_VOO_URL,
};
use crate::headline::{HeadlineConfig, HeadlineOutput, HeadlineRunner};
use crate::ingest::{
    AlchemyArchiveSource, ArchiveSource, IngestError, IngestionReport, Ingester, MockArchiveSource,
};
use crate::storage::{
    benchmarks::BenchmarkPoint,
    headline::HeadlineMonthlyRow,
    runs::{EquityCurvePoint, PositionRunSummary},
    strategy::StrategyResultRow,
    Storage,
};
use crate::strategies::{GridConfig, GridRunner};

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
/// Errors with `KeyRequired` when no Alchemy/RPC key is configured;
/// the frontend handles that by falling back to its own default window.
#[allow(dead_code)] // referenced via tauri::generate_handler! macro
#[tauri::command]
pub async fn lp_get_chain_head() -> Result<u64, CommandError> {
    let source =
        AlchemyArchiveSource::from_environment().map_err(map_ingest_error)?;
    source
        .latest_finalized_block()
        .await
        .map_err(map_ingest_error)
}

/// Backfills the supplied block range for a pool. Uses the Alchemy
/// archive source if a key is configured, otherwise falls back to the
/// mock source and reports `KeyRequired`.
#[tauri::command]
pub async fn run_lp_ingestion(
    storage: State<'_, Arc<Storage>>,
    pool_address: String,
    from_block: u64,
    to_block: u64,
) -> Result<IngestionReport, CommandError> {
    let source: Arc<dyn ArchiveSource> = match AlchemyArchiveSource::from_environment() {
        Ok(src) => Arc::new(src.with_payg_unbounded(true)),
        Err(e) => return Err(map_ingest_error(e)),
    };
    let ingester = Ingester::new((**storage).clone(), source);
    ingester
        .backfill(&pool_address, from_block, to_block)
        .await
        .map_err(map_ingest_error)
}

/// Backfills using a fixture/mock source — used when no Alchemy key is
/// configured and the user clicks "ingest a synthetic dataset" in the
/// GUI to demo the rest of the pipeline. Generates a sinusoidal swap
/// stream over the requested block range.
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
    let mock = synthetic_mock(&pool_address, from_block, to_block);
    let ingester = Ingester::new((**storage).clone(), mock);
    ingester
        .backfill(&pool_address, from_block, to_block)
        .await
        .map_err(map_ingest_error)
}

fn synthetic_mock(pool: &str, from: u64, to: u64) -> Arc<MockArchiveSource> {
    use num_bigint::BigUint;

    use crate::ingest::decoder::SWAP_TOPIC0;
    use crate::ingest::EthLog;
    use crate::math::tick_to_sqrt_price_x96;

    let mock = Arc::new(MockArchiveSource::new(to + 100));
    for b in from..=to {
        // Smooth tick walk between -300 and +300 over the period.
        let phase = ((b - from) as f64 / (to - from + 1) as f64) * std::f64::consts::TAU;
        let tick = (phase.sin() * 300.0) as i32;
        let sqrt = match tick_to_sqrt_price_x96(tick) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let topic0 = format!("0x{SWAP_TOPIC0}");
        let topic1 = format!("0x{:0>64}", "1");
        let topic2 = format!("0x{:0>64}", "2");
        let amount0_hex = format!("{:0>64x}", 1_000_000_000_000_000_000u64);
        let two_to_256: BigUint = BigUint::from(1u8) << 256;
        let amount1_neg = &two_to_256 - BigUint::from(3_000_000_000u64);
        let amount1_hex = format!("{:0>64}", amount1_neg.to_str_radix(16));
        let sqrt_hex = format!("{:0>64}", sqrt.to_str_radix(16));
        let liq_hex = format!("{:0>64x}", 1_000_000_000_000u128);
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
            transaction_hash: format!("0x{:0>64}", "deadbeef"),
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
            let p = TradFiProvider::new(&fetcher);
            p.fetch_stooq(STOOQ_VOO_URL, &series_key).await?
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

// Keep the unused-import lint quiet for the mock-only synthetic helper.
#[allow(dead_code)]
fn _link_mock(_: MockHttpFetcher) {}
