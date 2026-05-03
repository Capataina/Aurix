//! Alpha decomposition + cross-window robustness.
//!
//! Computes per-benchmark alpha (`LP_return - benchmark_return`) at the
//! period level and across rolling 30/60/90 day windows. Reports the
//! median, p25, p75 of the rolling spread distribution — the
//! cross-window robustness check the M2.7 plan calls for.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlphaSummary {
    pub benchmark_label: String,
    pub period_alpha_pct: f64,
    pub rolling_30d_median: f64,
    pub rolling_30d_p25: f64,
    pub rolling_30d_p75: f64,
    pub rolling_60d_median: f64,
    pub rolling_60d_p25: f64,
    pub rolling_60d_p75: f64,
    pub rolling_90d_median: f64,
    pub rolling_90d_p25: f64,
    pub rolling_90d_p75: f64,
}

/// Computes the period alpha and per-window rolling distributions for
/// a single benchmark.
///
/// `lp_daily_returns` and `benchmark_daily_returns` must be aligned by
/// date and the same length. Each value is a daily fractional return.
pub fn alpha_summary(
    benchmark_label: &str,
    lp_daily_returns: &[f64],
    benchmark_daily_returns: &[f64],
) -> AlphaSummary {
    let aligned_len = lp_daily_returns.len().min(benchmark_daily_returns.len());
    let lp = &lp_daily_returns[..aligned_len];
    let bench = &benchmark_daily_returns[..aligned_len];

    let period_alpha_pct = if !lp.is_empty() {
        let lp_total: f64 = lp.iter().map(|r| (1.0 + r).ln()).sum();
        let bench_total: f64 = bench.iter().map(|r| (1.0 + r).ln()).sum();
        100.0 * (lp_total.exp() - 1.0 - (bench_total.exp() - 1.0))
    } else {
        0.0
    };

    let (m30, p25_30, p75_30) = rolling_distribution(lp, bench, 30);
    let (m60, p25_60, p75_60) = rolling_distribution(lp, bench, 60);
    let (m90, p25_90, p75_90) = rolling_distribution(lp, bench, 90);

    AlphaSummary {
        benchmark_label: benchmark_label.to_string(),
        period_alpha_pct,
        rolling_30d_median: m30,
        rolling_30d_p25: p25_30,
        rolling_30d_p75: p75_30,
        rolling_60d_median: m60,
        rolling_60d_p25: p25_60,
        rolling_60d_p75: p75_60,
        rolling_90d_median: m90,
        rolling_90d_p25: p25_90,
        rolling_90d_p75: p75_90,
    }
}

/// Rolling-window distribution of (lp_window_return - benchmark_window_return)
/// in percentage points. Returns (median, p25, p75); zeros if the
/// series is shorter than the window.
fn rolling_distribution(lp: &[f64], bench: &[f64], window: usize) -> (f64, f64, f64) {
    if lp.len() < window {
        return (0.0, 0.0, 0.0);
    }
    let n = lp.len();
    let mut spreads: Vec<f64> = Vec::with_capacity(n - window + 1);
    for end in window..=n {
        let start = end - window;
        let lp_w: f64 = lp[start..end].iter().map(|r| (1.0 + r).ln()).sum();
        let bench_w: f64 = bench[start..end].iter().map(|r| (1.0 + r).ln()).sum();
        let spread = 100.0 * (lp_w.exp() - 1.0 - (bench_w.exp() - 1.0));
        spreads.push(spread);
    }
    spreads.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = percentile(&spreads, 50.0);
    let p25 = percentile(&spreads, 25.0);
    let p75 = percentile(&spreads, 75.0);
    (median, p25, p75)
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let n = sorted.len();
    let rank = (p / 100.0 * (n as f64 - 1.0)).round() as usize;
    sorted[rank.min(n - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn period_alpha_zero_when_streams_match() {
        let r = vec![0.001; 10];
        let s = alpha_summary("test", &r, &r);
        assert!(s.period_alpha_pct.abs() < 1e-9);
    }

    #[test]
    fn lp_outperforms_benchmark_period_alpha_positive() {
        let lp = vec![0.002; 30];
        let bench = vec![0.001; 30];
        let s = alpha_summary("test", &lp, &bench);
        assert!(s.period_alpha_pct > 0.0);
    }

    #[test]
    fn rolling_window_short_series_returns_zeros() {
        let lp = vec![0.01; 10];
        let bench = vec![0.005; 10];
        let s = alpha_summary("test", &lp, &bench);
        assert_eq!(s.rolling_30d_median, 0.0);
        assert_eq!(s.rolling_60d_median, 0.0);
        assert_eq!(s.rolling_90d_median, 0.0);
    }

    #[test]
    fn rolling_window_distribution_centred_on_constant_spread() {
        // Constant 0.1pp spread per day → 30-day rolling spread should be
        // approximately 30 * 0.1 = 3pp (compounding gives slightly more).
        let lp = vec![0.002; 60];
        let bench = vec![0.001; 60];
        let s = alpha_summary("test", &lp, &bench);
        // 30-day rolling alpha should be positive and consistent.
        assert!(s.rolling_30d_median > 0.0);
        assert!(s.rolling_30d_p25 >= 0.0);
        assert!((s.rolling_30d_p75 - s.rolling_30d_p25).abs() < 1e-6);
    }

    #[test]
    fn percentile_handles_empty() {
        assert_eq!(percentile(&[], 50.0), 0.0);
    }

    #[test]
    fn percentile_handles_single() {
        assert_eq!(percentile(&[5.0], 0.0), 5.0);
        assert_eq!(percentile(&[5.0], 100.0), 5.0);
    }

    #[test]
    fn percentile_basic() {
        let v = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(percentile(&v, 50.0), 3.0);
        assert_eq!(percentile(&v, 0.0), 1.0);
        assert_eq!(percentile(&v, 100.0), 5.0);
    }
}
