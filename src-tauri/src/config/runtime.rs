//! Cross-cutting runtime parameters.
//!
//! Values used by the backend pipeline that aren't pair- or venue-specific.
//! When more than one chain is supported in the future, this struct is the
//! natural home for `chain_label` to vary per request.

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Display label for the active chain. Surfaced verbatim in
    /// `MarketOverview.chain` and `PriceSnapshot.chain`.
    pub chain_label: String,
    /// Default gas-units estimate used by the frontend's gas-adjusted
    /// analytics. Backend doesn't apply it; the value is exposed via
    /// the `runtime_config` command so the GUI can read a single source.
    pub gas_units_estimate: u64,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            chain_label: "Ethereum Mainnet".to_string(),
            gas_units_estimate: 220_000,
        }
    }
}
