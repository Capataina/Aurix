mod commands;
mod config;
mod dex;
mod ethereum;
mod ingest;
mod market;
mod math;
mod storage;

pub use storage::{DbLocation, Storage};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::market::fetch_market_overview,
            commands::market::list_pairs,
            commands::market::runtime_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
