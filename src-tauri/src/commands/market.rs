use crate::config::AppConfig;
use crate::dex::uniswap_v2;
use crate::dex::uniswap_v3;
use crate::ethereum::client::EthereumRpcClient;
use crate::market::types::MarketOverview;

/// Fetches the current multi-venue market overview exposed to the GUI.
///
/// Inputs: none from the frontend; configuration is resolved from process
/// environment or a local `.env` file.
/// Outputs: a normalised market overview containing three live venue snapshots.
/// Errors: returned when configuration is missing or any required venue cannot
/// be fetched safely.
/// Side effects: performs read-only network requests to Ethereum mainnet.
#[tauri::command]
pub async fn fetch_market_overview() -> Result<MarketOverview, String> {
    let configuration = AppConfig::from_environment().map_err(|error| error.to_string())?;
    let rpc_client = EthereumRpcClient::new(configuration.ethereum_mainnet_rpc_url());

    let uniswap_v3_5bps = uniswap_v3::fetch_weth_usdc_snapshot(&rpc_client);
    let uniswap_v3_30bps = uniswap_v3::fetch_weth_usdc_30bps_snapshot(&rpc_client);
    let uniswap_v2 = uniswap_v2::fetch_uniswap_v2_snapshot(&rpc_client);
    let sushiswap = uniswap_v2::fetch_sushiswap_snapshot(&rpc_client);
    let gas_price_gwei = rpc_client.gas_price_gwei();
    let (uniswap_v3_5bps, uniswap_v3_30bps, uniswap_v2, sushiswap, gas_price_gwei) =
        tokio::join!(
            uniswap_v3_5bps,
            uniswap_v3_30bps,
            uniswap_v2,
            sushiswap,
            gas_price_gwei
        );

    let uniswap_v3_5bps = uniswap_v3_5bps.map_err(|error| error.to_string())?;
    let uniswap_v3_30bps = uniswap_v3_30bps.map_err(|error| error.to_string())?;
    let uniswap_v2 = uniswap_v2.map_err(|error| error.to_string())?;
    let sushiswap = sushiswap.map_err(|error| error.to_string())?;
    let gas_price_gwei = gas_price_gwei.map_err(|error| error.to_string())?;

    Ok(MarketOverview {
        chain: "Ethereum Mainnet".to_string(),
        pair_label: "WETH / USDC".to_string(),
        fetched_at_unix_ms: uniswap_v3_5bps.fetched_at_unix_ms,
        gas_price_gwei,
        venues: vec![uniswap_v3_5bps, uniswap_v3_30bps, uniswap_v2, sushiswap],
    })
}
