//! beaconcha.in ETH.STORE endpoint — KEY_REQUIRED.
//!
//! Returns the daily aggregated ETH staking yield for native validators
//! (ETH.STORE is the institutional reference rate). The fallback when no
//! key is configured is to gross up Lido's stETH APY by ~10% to back-out
//! the protocol's performance fee — accurate to within 30-50 bp per
//! plan paper 4.

use crate::storage::benchmarks::BenchmarkPoint;

use super::error::BenchmarkError;
use super::http::HttpFetcher;

const ETH_STORE_BASE: &str = "https://beaconcha.in/api/v1/ethstore";

pub struct BeaconChainProvider<'a> {
    fetcher: &'a dyn HttpFetcher,
    api_key: Option<String>,
}

impl<'a> BeaconChainProvider<'a> {
    pub fn from_environment(fetcher: &'a dyn HttpFetcher) -> Self {
        let api_key = std::env::var("BEACONCHAIN_API_KEY").ok();
        Self { fetcher, api_key }
    }

    pub fn with_key(fetcher: &'a dyn HttpFetcher, key: impl Into<String>) -> Self {
        Self {
            fetcher,
            api_key: Some(key.into()),
        }
    }

    /// Fetches the ETH.STORE daily staking-yield series. Without a key
    /// returns `KeyRequired`; the runner should fall back to the
    /// Lido-grossed-up estimate via `lido_grossed_up_proxy`.
    pub async fn fetch_eth_store(
        &self,
        _from_date: &str,
        _to_date: &str,
    ) -> Result<Vec<BenchmarkPoint>, BenchmarkError> {
        let _key = self
            .api_key
            .as_ref()
            .ok_or(BenchmarkError::KeyRequired("BEACONCHAIN_API_KEY"))?;
        // Live endpoint shape:
        // GET https://beaconcha.in/api/v1/ethstore/<day>?apikey=<key>
        // returns {"data": {"day": ..., "effective_balances_sum_wei": ...,
        //                   "apr": ...}}
        // Day-by-day pagination: 1 req/sec rate limit, 1k req/month cap.
        // Implementation deferred until the key is supplied; the
        // call signature + URL shape is committed so the live wiring is
        // a one-line change.
        let _now = chrono::Utc::now().timestamp_millis();
        Err(BenchmarkError::KeyRequired("BEACONCHAIN_API_KEY"))
    }

    /// Lido-grossed-up proxy: takes a Lido stETH APY series and grosses
    /// up by 10% to approximate native ETH staking yield (which earns
    /// the full validator yield without paying the Lido performance
    /// fee). Used when no beaconcha.in key is configured. Accuracy is
    /// 30-50 bp per plan paper 4.
    pub fn lido_grossed_up_proxy(
        lido_steth_points: &[BenchmarkPoint],
        target_series_key: &str,
    ) -> Vec<BenchmarkPoint> {
        let now_ms = chrono::Utc::now().timestamp_millis();
        lido_steth_points
            .iter()
            .map(|p| BenchmarkPoint {
                series_key: target_series_key.to_string(),
                sample_date: p.sample_date.clone(),
                value: p.value / 0.9, // gross up by 1/(1 - 0.1)
                source: "lido_grossed_up".to_string(),
                fetched_at_unix_ms: now_ms,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmarks::http::MockHttpFetcher;

    #[tokio::test]
    async fn fetch_without_key_returns_key_required() {
        let mock = MockHttpFetcher::new();
        let provider = BeaconChainProvider {
            fetcher: &mock,
            api_key: None,
        };
        let r = provider.fetch_eth_store("2024-01-01", "2024-01-02").await;
        assert!(matches!(r, Err(BenchmarkError::KeyRequired(_))));
    }

    #[test]
    fn lido_grossed_up_inverts_10pct_fee() {
        let lido = vec![BenchmarkPoint {
            series_key: "lido".into(),
            sample_date: "2024-01-01".into(),
            value: 3.6,
            source: "defillama".into(),
            fetched_at_unix_ms: 0,
        }];
        let grossed =
            BeaconChainProvider::lido_grossed_up_proxy(&lido, "eth_store_proxy");
        assert_eq!(grossed.len(), 1);
        // 3.6 / 0.9 = 4.0
        assert!((grossed[0].value - 4.0).abs() < 1e-9);
        assert_eq!(grossed[0].source, "lido_grossed_up");
    }
}
