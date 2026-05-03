//! V2 LP full-range benchmark.
//!
//! Constant-product (x*y=k) LP equity curve over a price series. For a
//! position seeded with notional N at entry-price P0:
//!
//! - At entry: x_0 = N / (2 * P0), y_0 = N / 2 (split 50/50 by USD).
//!   k = x_0 * y_0.
//! - At any later price P_t: x_t = sqrt(k / P_t), y_t = sqrt(k * P_t).
//!   Equity_t = x_t * P_t + y_t = 2 * sqrt(k * P_t).
//!
//! Compare to HODL: equity_hodl = x_0 * P_t + y_0.
//!
//! IL_v2 = equity_v2 / equity_hodl - 1 = 2*sqrt(P_t/P_0) / (1 + P_t/P_0) - 1
//! (matches `math::il::il_v2`).
//!
//! Per plan paper 3 + paper 1 §6, this is the canonical "what would V2
//! have done?" benchmark and is part of the M2.7 deliverable as a
//! DeFi-native primary benchmark.

use crate::storage::benchmarks::BenchmarkPoint;

/// Builds the V2 LP daily equity series from a price series.
/// `prices` is `(YYYY-MM-DD, price)` pairs in chronological order.
/// `notional_usd` is the entry-time deposit. Returns daily equity USD
/// per the constant-product formula.
pub fn v2_lp_equity_series(
    prices: &[(String, f64)],
    notional_usd: f64,
    series_key: &str,
) -> Vec<BenchmarkPoint> {
    if prices.is_empty() || notional_usd <= 0.0 {
        return Vec::new();
    }
    let p0 = prices[0].1;
    if p0 <= 0.0 {
        return Vec::new();
    }
    // Split notional 50/50.
    let x0 = (notional_usd / 2.0) / p0;
    let y0 = notional_usd / 2.0;
    let k = x0 * y0;
    let now_ms = chrono::Utc::now().timestamp_millis();
    prices
        .iter()
        .map(|(date, p)| {
            let p_safe = p.max(1e-12);
            let equity = 2.0 * (k * p_safe).sqrt();
            BenchmarkPoint {
                series_key: series_key.to_string(),
                sample_date: date.clone(),
                value: equity,
                source: "synthetic_v2lp".to_string(),
                fetched_at_unix_ms: now_ms,
            }
        })
        .collect()
}

/// Hold-only equity series for a 50/50 entry. Useful as the V2 baseline.
pub fn hodl_equity_series(
    prices: &[(String, f64)],
    notional_usd: f64,
    series_key: &str,
) -> Vec<BenchmarkPoint> {
    if prices.is_empty() || notional_usd <= 0.0 {
        return Vec::new();
    }
    let p0 = prices[0].1;
    if p0 <= 0.0 {
        return Vec::new();
    }
    let x0 = (notional_usd / 2.0) / p0;
    let y0 = notional_usd / 2.0;
    let now_ms = chrono::Utc::now().timestamp_millis();
    prices
        .iter()
        .map(|(date, p)| BenchmarkPoint {
            series_key: series_key.to_string(),
            sample_date: date.clone(),
            value: x0 * p + y0,
            source: "synthetic_hodl".to_string(),
            fetched_at_unix_ms: now_ms,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v2_lp_at_entry_equals_notional() {
        let prices = vec![("2024-01-01".to_string(), 3000.0)];
        let pts = v2_lp_equity_series(&prices, 10_000.0, "v2lp");
        assert_eq!(pts.len(), 1);
        assert!((pts[0].value - 10_000.0).abs() < 1e-6);
    }

    #[test]
    fn v2_lp_2x_price_grows_by_factor_sqrt2() {
        let prices = vec![
            ("2024-01-01".to_string(), 3000.0),
            ("2024-01-02".to_string(), 6000.0),
        ];
        let pts = v2_lp_equity_series(&prices, 10_000.0, "v2lp");
        // At P_0 = 3000: equity = $10,000.
        // At P_t = 6000: equity = 2*sqrt(k * 6000) where k = (10000/2/3000) * (10000/2)
        //              = (5000/3000) * 5000 = 8333.33...
        // Equity = 2*sqrt(8333.33 * 6000) = 2*sqrt(50_000_000) = 2*7071.06 ≈ 14142.13.
        // That's the V2 IL pattern: 10000 * sqrt(2) ≈ 14142.13.
        let expected = 10_000.0 * (2.0_f64).sqrt();
        assert!((pts[1].value - expected).abs() < 1.0);
    }

    #[test]
    fn hodl_at_entry_equals_notional() {
        let prices = vec![("2024-01-01".to_string(), 3000.0)];
        let pts = hodl_equity_series(&prices, 10_000.0, "hodl");
        assert!((pts[0].value - 10_000.0).abs() < 1e-6);
    }

    #[test]
    fn hodl_at_2x_price_grows_by_factor_1_5() {
        // 50/50 split: 5000 USD + 5000 USD worth of token0 (1.667 tokens).
        // At 2x price: 1.667 * 6000 + 5000 = 10000 + 5000 = 15000 → 1.5x.
        let prices = vec![
            ("2024-01-01".to_string(), 3000.0),
            ("2024-01-02".to_string(), 6000.0),
        ];
        let pts = hodl_equity_series(&prices, 10_000.0, "hodl");
        assert!((pts[1].value - 15_000.0).abs() < 1.0);
    }

    #[test]
    fn il_at_2x_matches_v2_closed_form() {
        // V2 LP at 2x: 14142.13. HODL at 2x: 15000.
        // IL = 14142.13 / 15000 - 1 ≈ -5.72%.
        let prices = vec![
            ("2024-01-01".to_string(), 3000.0),
            ("2024-01-02".to_string(), 6000.0),
        ];
        let v2 = v2_lp_equity_series(&prices, 10_000.0, "v2");
        let hodl = hodl_equity_series(&prices, 10_000.0, "hodl");
        let il = v2[1].value / hodl[1].value - 1.0;
        let expected = 2.0 * (2.0_f64).sqrt() / 3.0 - 1.0;
        assert!((il - expected).abs() < 1e-6);
    }

    #[test]
    fn empty_prices_returns_empty_series() {
        assert!(v2_lp_equity_series(&[], 10_000.0, "x").is_empty());
        assert!(hodl_equity_series(&[], 10_000.0, "x").is_empty());
    }
}
