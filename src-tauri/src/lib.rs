mod backtest;
mod benchmarks;
mod commands;
mod config;
mod dex;
mod ethereum;
mod headline;
mod ingest;
mod market;
mod math;
mod storage;
mod strategies;
mod validation;

pub use storage::{DbLocation, Storage};

use std::sync::Arc;

/// Resolves the path the production app uses for `aurix.sqlite`.
/// Honours `AURIX_DB_PATH` for overrides; otherwise falls back to
/// `~/.aurix/aurix.sqlite` (creating the directory if missing).
fn resolve_db_path() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("AURIX_DB_PATH") {
        return std::path::PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = std::path::PathBuf::from(home).join(".aurix");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("aurix.sqlite")
}

async fn open_storage() -> Arc<Storage> {
    let path = resolve_db_path();
    Arc::new(
        Storage::open(DbLocation::path(&path))
            .await
            .expect("failed to open Aurix storage"),
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime for storage init");
    let storage = runtime.block_on(open_storage());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(storage)
        .invoke_handler(tauri::generate_handler![
            commands::market::fetch_market_overview,
            commands::market::list_pairs,
            commands::market::runtime_config,
            commands::lp::run_lp_ingestion,
            commands::lp::run_lp_synthetic_ingest,
            commands::lp::run_lp_backtest,
            commands::lp::run_lp_grid,
            commands::lp::run_lp_headline,
            commands::lp::lp_get_equity_curve,
            commands::lp::lp_query_strategies,
            commands::lp::lp_query_headline_monthly,
            commands::lp::lp_fetch_benchmark_series,
            commands::lp::lp_query_benchmark_range,
            commands::lp::lp_get_chain_head,
            commands::lp::lp_query_first_swap_price,
            commands::lp::lp_pool_metadata,
            commands::lp::lp_token_usd_prices,
            commands::telemetry::telemetry_log_path,
            commands::telemetry::telemetry_persist,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
