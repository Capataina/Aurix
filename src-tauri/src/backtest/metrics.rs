//! Per-strategy risk-adjusted metrics.
//!
//! All metrics are computed on the equity curve's daily-aggregated returns
//! (we resample blocks→days for cross-asset comparability with the M2.7
//! benchmark series).
//!
//! - Sharpe = (mean(r) - rf) / stdev(r) * sqrt(252)
//! - Sortino = (mean(r) - rf) / downside_dev(r) * sqrt(252)
//! - Calmar = annualised_return / max_drawdown
//! - Deflated Sharpe (per Bailey-López de Prado) — corrects for selection
//!   bias when the strategy was the best of N tested. Approximation:
//!   `DSR ≈ Sharpe - sqrt(2*log(N)/N)` in the white-noise null.
//!
//! Reference: `context/references/backtest-statistical-methodology.md`.

const TRADING_DAYS_PER_YEAR: f64 = 252.0;

/// Sharpe ratio. Returns 0 when stdev is zero (constant series).
pub fn sharpe_ratio(returns: &[f64], risk_free_rate_per_day: f64) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let var = returns
        .iter()
        .map(|r| {
            let d = r - mean;
            d * d
        })
        .sum::<f64>()
        / returns.len() as f64;
    let std = var.sqrt();
    // Treat sub-femtoscale std as zero — it's the residual of f64 mean
    // arithmetic on a constant series, not real volatility.
    if std < 1e-12 {
        return 0.0;
    }
    (mean - risk_free_rate_per_day) / std * TRADING_DAYS_PER_YEAR.sqrt()
}

/// Sortino — uses downside deviation (only negative deviations from mean).
pub fn sortino_ratio(returns: &[f64], risk_free_rate_per_day: f64) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let downside_var = returns
        .iter()
        .filter_map(|r| {
            let d = r - mean;
            if d < 0.0 {
                Some(d * d)
            } else {
                None
            }
        })
        .sum::<f64>()
        / returns.len() as f64;
    let down_std = downside_var.sqrt();
    if down_std < 1e-12 {
        // No downside = infinite ratio in theory; cap at a large value.
        return if mean > risk_free_rate_per_day {
            10.0
        } else {
            0.0
        };
    }
    (mean - risk_free_rate_per_day) / down_std * TRADING_DAYS_PER_YEAR.sqrt()
}

/// Calmar = annualised return / max drawdown.
pub fn calmar_ratio(annualised_return: f64, max_drawdown_pct: f64) -> f64 {
    if max_drawdown_pct.abs() < 1e-9 {
        return 0.0;
    }
    annualised_return / max_drawdown_pct.abs()
}

/// Annualised return from a per-day return series.
pub fn annualise(daily_returns: &[f64]) -> f64 {
    if daily_returns.is_empty() {
        return 0.0;
    }
    let mean = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
    mean * TRADING_DAYS_PER_YEAR
}

/// Maximum drawdown of an equity curve, as a positive percentage.
/// `equity` is the running USD value of the position over time.
pub fn max_drawdown_pct(equity: &[f64]) -> f64 {
    if equity.is_empty() {
        return 0.0;
    }
    let mut peak = equity[0];
    let mut max_dd = 0.0;
    for &v in equity.iter() {
        if v > peak {
            peak = v;
        }
        if peak > 0.0 {
            let dd = (peak - v) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }
    max_dd
}

/// Deflated Sharpe Ratio — selection-bias corrected.
///
/// Approximation per Bailey-López de Prado: when the strategy was the
/// best of `n` strategies tested, the white-noise expected maximum
/// Sharpe is roughly `sqrt(2*log(n)/n)`. The deflated value subtracts
/// this baseline.
pub fn deflated_sharpe(sharpe: f64, n_strategies: usize) -> f64 {
    if n_strategies <= 1 {
        return sharpe;
    }
    let n = n_strategies as f64;
    let baseline = (2.0 * n.ln() / n).sqrt();
    sharpe - baseline
}

/// Convert a per-block equity series into per-day returns by aggregating
/// at day boundaries.
pub fn block_equity_to_daily_returns(
    timestamps_unix_seconds: &[i64],
    equity_usd: &[f64],
) -> Vec<f64> {
    if timestamps_unix_seconds.len() != equity_usd.len() || equity_usd.len() < 2 {
        return Vec::new();
    }
    let mut daily: Vec<(i64, f64)> = Vec::new();
    for (i, &ts) in timestamps_unix_seconds.iter().enumerate() {
        let day = ts / 86_400;
        if let Some(last) = daily.last_mut() {
            if last.0 == day {
                last.1 = equity_usd[i];
                continue;
            }
        }
        daily.push((day, equity_usd[i]));
    }
    let mut returns = Vec::with_capacity(daily.len().saturating_sub(1));
    for w in daily.windows(2) {
        let prev = w[0].1;
        let cur = w[1].1;
        if prev.abs() < 1e-12 {
            continue;
        }
        returns.push((cur - prev) / prev);
    }
    returns
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sharpe_zero_for_constant_returns() {
        let r = vec![0.001; 30];
        assert_eq!(sharpe_ratio(&r, 0.0), 0.0);
    }

    #[test]
    fn sharpe_higher_for_lower_variance_at_same_mean() {
        let lo_var = vec![0.001, 0.0011, 0.0009, 0.001, 0.0009];
        let hi_var = vec![0.005, -0.003, 0.004, -0.002, 0.001];
        let s_lo = sharpe_ratio(&lo_var, 0.0);
        let s_hi = sharpe_ratio(&hi_var, 0.0);
        assert!(s_lo > s_hi);
    }

    #[test]
    fn max_drawdown_simple() {
        // 100 → 90 → 110 → 70 → 80
        // Peak before 70 is 110; DD = (110-70)/110 = 0.3636...
        let dd = max_drawdown_pct(&[100.0, 90.0, 110.0, 70.0, 80.0]);
        assert!((dd - 0.3636363636).abs() < 1e-6);
    }

    #[test]
    fn max_drawdown_zero_for_monotonic_increase() {
        let dd = max_drawdown_pct(&[100.0, 110.0, 120.0]);
        assert_eq!(dd, 0.0);
    }

    #[test]
    fn deflated_sharpe_below_raw_when_n_large() {
        let raw = 1.0;
        let dsr = deflated_sharpe(raw, 100);
        assert!(dsr < raw);
        assert!(dsr > 0.0);
    }

    #[test]
    fn deflated_sharpe_unchanged_for_single_strategy() {
        assert_eq!(deflated_sharpe(0.5, 1), 0.5);
    }

    #[test]
    fn calmar_zero_when_no_drawdown() {
        assert_eq!(calmar_ratio(0.2, 0.0), 0.0);
    }

    #[test]
    fn block_to_daily_returns_collapses_per_day() {
        // 5 blocks: ts 0, 100, 86_400, 86_500, 172_800. Equity 100, 101,
        // 110, 105, 95.
        let ts = vec![0, 100, 86_400, 86_500, 172_800];
        let eq = vec![100.0, 101.0, 110.0, 105.0, 95.0];
        let r = block_equity_to_daily_returns(&ts, &eq);
        // Day 0 closes at 101 (last sample of day 0).
        // Day 1 closes at 105.
        // Day 2 closes at 95.
        // Returns: (105-101)/101 ≈ 0.0396, (95-105)/105 ≈ -0.0952.
        assert_eq!(r.len(), 2);
        assert!((r[0] - 4.0 / 101.0).abs() < 1e-9);
        assert!((r[1] - (-10.0 / 105.0)).abs() < 1e-9);
    }
}
