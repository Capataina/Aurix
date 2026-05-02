use std::sync::Arc;

use tokio::task::JoinSet;

use crate::config::{
    find_pair, list_pairs as list_pair_configs, AppConfig, PairConfig, PairSummary, RuntimeConfig,
    VenueProtocol,
};
use crate::dex::{uniswap_v2, uniswap_v3};
use crate::ethereum::client::EthereumRpcClient;
use crate::market::types::{MarketOverview, PriceSnapshot};

/// Returns the catalog of pairs the backend can read prices for.
///
/// Inputs: none.
/// Outputs: a vector of pair summaries ordered as registered in `config::pairs`.
/// Errors: never.
/// Side effects: none.
#[tauri::command]
pub fn list_pairs() -> Vec<PairSummary> {
    list_pair_configs()
        .iter()
        .map(PairSummary::from)
        .collect()
}

/// Returns the runtime parameters the frontend needs to mirror (gas units
/// estimate, chain label). Frontend reads this on startup so a single source
/// of truth lives in the backend.
#[tauri::command]
pub fn runtime_config() -> RuntimeConfigPayload {
    let runtime = RuntimeConfig::default();
    RuntimeConfigPayload {
        chain_label: runtime.chain_label,
        gas_units_estimate: runtime.gas_units_estimate,
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfigPayload {
    pub chain_label: String,
    pub gas_units_estimate: u64,
}

/// Fetches the current multi-venue market overview for a given pair.
///
/// Inputs: `pair_id` — id from the pair catalog. When `None` defaults to the
/// first registered pair.
/// Outputs: a normalised market overview containing one snapshot per venue
/// for the resolved pair, in catalog order (`venues[0]` is the hero).
/// Errors: returned when configuration is missing, the pair id is unknown,
/// or any required venue cannot be fetched safely.
/// Side effects: performs read-only network requests to Ethereum mainnet.
#[tauri::command]
pub async fn fetch_market_overview(
    pair_id: Option<String>,
) -> Result<MarketOverview, String> {
    let pairs = list_pair_configs();
    let resolved_pair = match pair_id {
        Some(id) => find_pair(&id).ok_or_else(|| format!("unknown pair id: {id}"))?,
        None => pairs
            .into_iter()
            .next()
            .ok_or_else(|| "no pairs registered".to_string())?,
    };

    let configuration = AppConfig::from_environment().map_err(|error| error.to_string())?;
    let rpc_client = EthereumRpcClient::new(configuration.ethereum_mainnet_rpc_url());
    let runtime = RuntimeConfig::default();
    let pair = Arc::new(resolved_pair);

    let snapshots = fetch_all_venues(&rpc_client, Arc::clone(&pair), &runtime.chain_label).await?;
    let gas_price_gwei = rpc_client
        .gas_price_gwei()
        .await
        .map_err(|error| error.to_string())?;

    let fetched_at_unix_ms = snapshots
        .first()
        .map(|snapshot| snapshot.fetched_at_unix_ms)
        .unwrap_or(0);

    Ok(MarketOverview {
        chain: runtime.chain_label,
        pair_id: pair.id.clone(),
        pair_label: pair.label.clone(),
        fetched_at_unix_ms,
        gas_price_gwei,
        venues: snapshots,
    })
}

/// Concurrently fetches a snapshot for every venue in `pair`. Tasks are
/// spawned with their original index and results are reordered after join so
/// the returned vector preserves catalog order — the `venues[0] = hero`
/// contract relied on by the frontend.
async fn fetch_all_venues(
    rpc_client: &EthereumRpcClient,
    pair: Arc<PairConfig>,
    chain_label: &str,
) -> Result<Vec<PriceSnapshot>, String> {
    let venue_count = pair.venues.len();
    let mut join_set: JoinSet<(usize, Result<PriceSnapshot, String>)> = JoinSet::new();

    for (idx, venue) in pair.venues.iter().enumerate() {
        let venue = venue.clone();
        let pair = Arc::clone(&pair);
        let rpc = rpc_client.clone();
        let chain = chain_label.to_string();

        join_set.spawn(async move {
            let snapshot = match &venue.protocol {
                VenueProtocol::UniswapV3 {
                    pool_address,
                    fee_tier_bps,
                } => uniswap_v3::fetch_snapshot(
                    &rpc,
                    pair.as_ref(),
                    &venue.dex_name,
                    pool_address,
                    *fee_tier_bps,
                    &chain,
                )
                .await
                .map_err(|error| error.to_string()),
                VenueProtocol::UniswapV2 {
                    factory_address,
                    fee_tier_bps,
                } => uniswap_v2::fetch_snapshot(
                    &rpc,
                    pair.as_ref(),
                    &venue.dex_name,
                    factory_address,
                    *fee_tier_bps,
                    &chain,
                )
                .await
                .map_err(|error| error.to_string()),
            };
            (idx, snapshot)
        });
    }

    let mut indexed: Vec<(usize, Result<PriceSnapshot, String>)> = Vec::with_capacity(venue_count);
    while let Some(join_result) = join_set.join_next().await {
        let outcome = join_result.map_err(|error| format!("venue task panicked: {error}"))?;
        indexed.push(outcome);
    }
    indexed.sort_by_key(|(idx, _)| *idx);

    indexed
        .into_iter()
        .map(|(_, snapshot)| snapshot)
        .collect::<Result<Vec<_>, String>>()
}
