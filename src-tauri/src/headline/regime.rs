//! Volatility-regime classifier.
//!
//! Adaptive terciles per plan paper 6 (and per `vector-a-v3-lp-
//! backtester.md` §M2.8 open decision):
//!
//! - Compute monthly realized vol = std-dev of daily ETH returns.
//! - Classify each month into low / medium / high by terciles within
//!   the lookback window.
//!
//! Adaptive terciles auto-balance regime sizes and adapt to whatever
//! window the user chose; fixed thresholds (2%/4%) break across regimes.
//! GARCH/HMM are out of scope (per the contrasting source — both break
//! during the COVID shock).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VolRegime {
    Low,
    Medium,
    High,
}

impl VolRegime {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

/// Classifies an ordered series of monthly vol values into per-month
/// regimes via adaptive terciles. Returns one regime per input month.
///
/// Inputs: monthly_vol — `(year_month_string, vol)` pairs in order.
/// Outputs: same length as input; the regime tag for each month.
pub fn classify_terciles(monthly_vol: &[(String, f64)]) -> Vec<VolRegime> {
    let n = monthly_vol.len();
    if n == 0 {
        return Vec::new();
    }
    let mut sorted: Vec<f64> = monthly_vol.iter().map(|(_, v)| *v).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let t1 = sorted[(n as f64 / 3.0).floor() as usize];
    let t2 = sorted[((2.0 * n as f64) / 3.0).floor() as usize];

    monthly_vol
        .iter()
        .map(|(_, v)| {
            if *v < t1 {
                VolRegime::Low
            } else if *v < t2 {
                VolRegime::Medium
            } else {
                VolRegime::High
            }
        })
        .collect()
}

/// Computes monthly realized vol from a daily-return series.
/// `daily` is `(YYYY-MM-DD, fractional_return)` pairs in order. Returns
/// `(YYYY-MM, std_dev_of_daily_returns)` per month.
pub fn monthly_realized_vol(daily: &[(String, f64)]) -> Vec<(String, f64)> {
    if daily.is_empty() {
        return Vec::new();
    }
    let mut by_month: Vec<(String, Vec<f64>)> = Vec::new();
    for (date, r) in daily {
        if date.len() < 7 {
            continue;
        }
        let ym = date[..7].to_string();
        if let Some(last) = by_month.last_mut() {
            if last.0 == ym {
                last.1.push(*r);
                continue;
            }
        }
        by_month.push((ym, vec![*r]));
    }
    by_month
        .into_iter()
        .map(|(ym, returns)| {
            let mean: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
            let var: f64 = returns
                .iter()
                .map(|r| {
                    let d = r - mean;
                    d * d
                })
                .sum::<f64>()
                / returns.len() as f64;
            (ym, var.sqrt())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terciles_split_evenly_for_uniform_input() {
        let v: Vec<(String, f64)> = (0..9)
            .map(|i| (format!("2024-{:02}", i + 1), i as f64))
            .collect();
        let regs = classify_terciles(&v);
        // First 3 → Low, middle 3 → Medium, last 3 → High.
        assert_eq!(regs[0], VolRegime::Low);
        assert_eq!(regs[2], VolRegime::Low);
        assert_eq!(regs[3], VolRegime::Medium);
        assert_eq!(regs[5], VolRegime::Medium);
        assert_eq!(regs[6], VolRegime::High);
        assert_eq!(regs[8], VolRegime::High);
    }

    #[test]
    fn empty_input_returns_empty() {
        assert!(classify_terciles(&[]).is_empty());
        assert!(monthly_realized_vol(&[]).is_empty());
    }

    #[test]
    fn monthly_vol_groups_by_year_month() {
        let daily = vec![
            ("2024-01-01".to_string(), 0.01),
            ("2024-01-02".to_string(), -0.01),
            ("2024-02-01".to_string(), 0.02),
            ("2024-02-02".to_string(), -0.02),
        ];
        let monthly = monthly_realized_vol(&daily);
        assert_eq!(monthly.len(), 2);
        assert_eq!(monthly[0].0, "2024-01");
        assert_eq!(monthly[1].0, "2024-02");
        assert!(monthly[0].1 > 0.0);
        assert!(monthly[1].1 > monthly[0].1);
    }
}
