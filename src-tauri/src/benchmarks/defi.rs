//! DefiLlama no-key benchmark fetchers.
//!
//! Endpoint: `https://yields.llama.fi/chart/<pool_uuid>` returns:
//!
//! ```json
//! { "status": "success", "data": [
//!   { "timestamp": "2024-01-01T00:00:00.000Z", "tvlUsd": ..., "apy": 4.5, ... },
//!   ...
//! ] }
//! ```
//!
//! Pool UUIDs (per `context/references/defi-yield-data-sources.md`):
//! - Aave V3 ETH USDC supply: `aa70268e-4b52-42bf-a116-608b370f9501`
//! - Compound V3 ETH USDC supply: `7da72d09-56ca-4ec5-a45f-59114353e487`
//! - Lido stETH:                 `747c1d2a-c668-4682-b9f9-296708a3dd90`

use serde::Deserialize;

use crate::storage::benchmarks::BenchmarkPoint;

use super::error::BenchmarkError;
use super::http::HttpFetcher;

pub const AAVE_V3_USDC_SUPPLY_POOL: &str = "aa70268e-4b52-42bf-a116-608b370f9501";
pub const COMPOUND_V3_USDC_SUPPLY_POOL: &str = "7da72d09-56ca-4ec5-a45f-59114353e487";
pub const LIDO_STETH_POOL: &str = "747c1d2a-c668-4682-b9f9-296708a3dd90";

const BASE_URL: &str = "https://yields.llama.fi/chart";

#[derive(Debug, Deserialize)]
struct DefillamaResponse {
    #[serde(default)]
    status: Option<String>,
    data: Vec<DefillamaPoint>,
}

#[derive(Debug, Deserialize)]
struct DefillamaPoint {
    timestamp: String, // ISO 8601, e.g. "2024-01-01T00:00:00.000Z"
    apy: Option<f64>,
}

pub struct DefiLlamaProvider<'a> {
    fetcher: &'a dyn HttpFetcher,
}

impl<'a> DefiLlamaProvider<'a> {
    pub fn new(fetcher: &'a dyn HttpFetcher) -> Self {
        Self { fetcher }
    }

    /// Returns the full APY history for the pool. Each point uses the
    /// timestamp's date (YYYY-MM-DD) as the canonical sample_date and
    /// the APY as the value (in percent — 4.5 means 4.5% APY).
    pub async fn fetch_pool_apy(
        &self,
        pool_uuid: &str,
        series_key: &str,
    ) -> Result<Vec<BenchmarkPoint>, BenchmarkError> {
        let url = format!("{BASE_URL}/{pool_uuid}");
        let body = self.fetcher.fetch(&url).await?;
        let parsed: DefillamaResponse = serde_json::from_str(&body)
            .map_err(|e| BenchmarkError::Parse(format!("defillama {pool_uuid}: {e}")))?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut out = Vec::with_capacity(parsed.data.len());
        for p in parsed.data {
            let sample_date = iso_timestamp_to_date(&p.timestamp)?;
            let apy = match p.apy {
                Some(v) => v,
                None => continue,
            };
            out.push(BenchmarkPoint {
                series_key: series_key.to_string(),
                sample_date,
                value: apy,
                source: "defillama".to_string(),
                fetched_at_unix_ms: now_ms,
            });
        }
        Ok(out)
    }
}

fn iso_timestamp_to_date(iso: &str) -> Result<String, BenchmarkError> {
    if iso.len() < 10 {
        return Err(BenchmarkError::Date(format!("malformed iso: {iso}")));
    }
    Ok(iso[..10].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmarks::http::MockHttpFetcher;

    #[tokio::test]
    async fn parses_minimal_response() {
        let mock = MockHttpFetcher::new();
        let body = r#"{
            "status": "success",
            "data": [
                {"timestamp": "2024-01-01T00:00:00.000Z", "apy": 4.5},
                {"timestamp": "2024-01-02T00:00:00.000Z", "apy": 4.6},
                {"timestamp": "2024-01-03T00:00:00.000Z", "apy": null}
            ]
        }"#;
        mock.insert(format!("{BASE_URL}/{AAVE_V3_USDC_SUPPLY_POOL}"), body);

        let provider = DefiLlamaProvider::new(&mock);
        let points = provider
            .fetch_pool_apy(AAVE_V3_USDC_SUPPLY_POOL, "aave_v3_usdc_supply_apy")
            .await
            .unwrap();
        assert_eq!(points.len(), 2, "null APY rows should be filtered");
        assert_eq!(points[0].sample_date, "2024-01-01");
        assert_eq!(points[0].value, 4.5);
        assert_eq!(points[1].sample_date, "2024-01-02");
        assert_eq!(points[0].source, "defillama");
        assert_eq!(points[0].series_key, "aave_v3_usdc_supply_apy");
    }

    #[tokio::test]
    async fn returns_error_on_unparseable_response() {
        let mock = MockHttpFetcher::new();
        mock.insert(format!("{BASE_URL}/{LIDO_STETH_POOL}"), "not json");
        let provider = DefiLlamaProvider::new(&mock);
        let r = provider.fetch_pool_apy(LIDO_STETH_POOL, "x").await;
        assert!(matches!(r, Err(BenchmarkError::Parse(_))));
    }
}
