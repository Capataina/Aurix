use serde::Serialize;

/// Normalised market snapshot returned from the backend to the GUI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceSnapshot {
    pub chain: String,
    pub dex_name: String,
    pub pair_label: String,
    pub price_usd: f64,
    pub pool_address: String,
    pub fee_tier_bps: u16,
    pub price_source_label: String,
    pub fetched_at_unix_ms: u64,
}

/// Aggregated market overview containing all venue snapshots for one sampling tick.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketOverview {
    pub chain: String,
    pub pair_label: String,
    pub fetched_at_unix_ms: u64,
    pub gas_price_gwei: f64,
    pub venues: Vec<PriceSnapshot>,
}
