//! TradFi benchmark fetchers — FRED + Stooq + Yahoo no-key chain.
//!
//! Reference: `context/references/tradfi-benchmark-data-sources.md`.
//!
//! - FRED `.txt` endpoint: tab-delimited, plain text. Header line, then
//!   "DATE   VALUE" per row. URL: `https://fred.stlouisfed.org/data/<SERIES>.txt`.
//!   Series ids:
//!     DGS3MO     — 3-month Treasury constant maturity rate (%)
//!     DGS1       — 1-year Treasury constant maturity rate (%)
//!     GOLDAMGBD228NLBM — LBMA London PM gold fix (USD/oz)
//!
//! - Stooq CSV: `https://stooq.com/q/d/l/?s=<symbol>&i=d`, CSV with
//!   header `Date,Open,High,Low,Close,Volume`.
//!     voo.us     — Vanguard S&P 500 ETF (TradFi sanity, no expense
//!                  ratio adjustment needed since Close is adjusted).
//!     xauusd     — gold spot fallback when FRED's series is missing.
//!
//! - Yahoo: `https://query1.finance.yahoo.com/v7/finance/download/<symbol>?period1=...&period2=...&interval=1d`
//!   returns CSV with header `Date,Open,High,Low,Close,Adj Close,Volume`.
//!     ^SP500TR   — S&P 500 total return (the canonical "what an
//!                  investor in SPY would have made with dividends
//!                  reinvested").

use crate::storage::benchmarks::BenchmarkPoint;

use super::error::BenchmarkError;
use super::http::HttpFetcher;

pub const FRED_DGS3MO_URL: &str = "https://fred.stlouisfed.org/data/DGS3MO.txt";
pub const FRED_DGS1_URL: &str = "https://fred.stlouisfed.org/data/DGS1.txt";
pub const FRED_GOLD_LBMA_URL: &str = "https://fred.stlouisfed.org/data/GOLDAMGBD228NLBM.txt";
/// FRED S&P 500 daily close. License with S&P limits historical depth
/// to ~10 years; for our 6-month lookback that's irrelevant. No API
/// key required — uses the public `.txt` file endpoint.
pub const FRED_SP500_URL: &str = "https://fred.stlouisfed.org/data/SP500.txt";
pub const STOOQ_VOO_URL: &str = "https://stooq.com/q/d/l/?s=voo.us&i=d";
pub const STOOQ_XAUUSD_URL: &str = "https://stooq.com/q/d/l/?s=xauusd&i=d";

pub struct TradFiProvider<'a> {
    fetcher: &'a dyn HttpFetcher,
}

impl<'a> TradFiProvider<'a> {
    pub fn new(fetcher: &'a dyn HttpFetcher) -> Self {
        Self { fetcher }
    }

    /// Parses a FRED `.txt` body. Body shape (real FRED format):
    ///
    /// ```text
    /// Title:    Some Title
    /// Series ID: DGS3MO
    /// ...
    ///
    /// DATE        VALUE
    /// 2023-01-02  4.65
    /// 2023-01-03  4.70
    /// ```
    ///
    /// Skip the header lines (everything until the first row matching
    /// `^\d{4}-\d{2}-\d{2}\s`). `.` rows are missing data — FRED's
    /// convention; treat as no-data and skip.
    pub async fn fetch_fred(
        &self,
        url: &str,
        series_key: &str,
    ) -> Result<Vec<BenchmarkPoint>, BenchmarkError> {
        let body = self.fetcher.fetch(url).await?;
        Ok(parse_fred_body(&body, series_key))
    }

    /// Parses Stooq CSV. Header: `Date,Open,High,Low,Close,Volume`.
    /// We extract `Date,Close`. Returned `value` is the close price.
    pub async fn fetch_stooq(
        &self,
        url: &str,
        series_key: &str,
    ) -> Result<Vec<BenchmarkPoint>, BenchmarkError> {
        let body = self.fetcher.fetch(url).await?;
        Ok(parse_stooq_csv(&body, series_key)?)
    }
}

fn parse_fred_body(body: &str, series_key: &str) -> Vec<BenchmarkPoint> {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let mut out = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Date row starts with YYYY-MM-DD.
        if trimmed.len() < 10 {
            continue;
        }
        let head = &trimmed[..10];
        if !is_iso_date(head) {
            continue;
        }
        // Split by whitespace; date + value (or `.` for missing).
        let mut parts = trimmed.split_whitespace();
        let date = parts.next().unwrap_or("");
        let value_str = parts.next().unwrap_or(".");
        if value_str == "." || value_str.is_empty() {
            continue;
        }
        let value: f64 = match value_str.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        out.push(BenchmarkPoint {
            series_key: series_key.to_string(),
            sample_date: date.to_string(),
            value,
            source: "fred".to_string(),
            fetched_at_unix_ms: now_ms,
        });
    }
    out
}

fn parse_stooq_csv(body: &str, series_key: &str) -> Result<Vec<BenchmarkPoint>, BenchmarkError> {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let mut out = Vec::new();
    let mut lines = body.lines();
    // Header line.
    let header = lines
        .next()
        .ok_or_else(|| BenchmarkError::Parse("stooq: empty body".into()))?;
    if !header.starts_with("Date") {
        return Err(BenchmarkError::Parse(format!(
            "stooq: unexpected header: {header}"
        )));
    }
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.split(',').collect();
        if parts.len() < 5 {
            continue;
        }
        let date = parts[0];
        if !is_iso_date(date) {
            continue;
        }
        let close: f64 = match parts[4].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        out.push(BenchmarkPoint {
            series_key: series_key.to_string(),
            sample_date: date.to_string(),
            value: close,
            source: "stooq".to_string(),
            fetched_at_unix_ms: now_ms,
        });
    }
    Ok(out)
}

fn is_iso_date(s: &str) -> bool {
    let bytes = s.as_bytes();
    s.len() >= 10
        && bytes[..4].iter().all(|b| b.is_ascii_digit())
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(|b| b.is_ascii_digit())
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmarks::http::MockHttpFetcher;

    #[test]
    fn fred_parser_skips_header_and_dot_rows() {
        let body = r#"Title: Test
Series ID: DGS3MO
Notes: blah blah

DATE        VALUE
2023-01-02  4.65
2023-01-03  .
2023-01-04  4.72
"#;
        let pts = parse_fred_body(body, "fred_dgs3mo");
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0].sample_date, "2023-01-02");
        assert_eq!(pts[0].value, 4.65);
        assert_eq!(pts[1].sample_date, "2023-01-04");
        assert_eq!(pts[1].source, "fred");
    }

    #[test]
    fn stooq_parser_extracts_close() {
        let body = "Date,Open,High,Low,Close,Volume\n2024-01-02,400.0,402.5,399.0,401.50,123456\n2024-01-03,401.5,403.0,400.0,402.10,234567\n";
        let pts = parse_stooq_csv(body, "stooq_voo").unwrap();
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0].sample_date, "2024-01-02");
        assert_eq!(pts[0].value, 401.50);
        assert_eq!(pts[1].value, 402.10);
        assert_eq!(pts[0].source, "stooq");
    }

    #[test]
    fn stooq_parser_rejects_bad_header() {
        let body = "WRONG,header\n2024-01-02,400.0\n";
        let r = parse_stooq_csv(body, "x");
        assert!(matches!(r, Err(BenchmarkError::Parse(_))));
    }

    #[tokio::test]
    async fn provider_fetches_through_mock() {
        let mock = MockHttpFetcher::new();
        mock.insert(
            FRED_DGS3MO_URL.to_string(),
            "DATE  VALUE\n2024-01-02  4.65\n",
        );
        let provider = TradFiProvider::new(&mock);
        let pts = provider
            .fetch_fred(FRED_DGS3MO_URL, "fred_dgs3mo")
            .await
            .unwrap();
        assert_eq!(pts.len(), 1);
        assert_eq!(pts[0].value, 4.65);
    }
}
