//! Headline verdict synthesis.
//!
//! Per `vector-a-v3-lp-backtester.md` §M2.8: turn the per-month
//! grid into a defensible recommendation.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use super::error::HeadlineError;
use super::regime::{classify_terciles, monthly_realized_vol, VolRegime};
use crate::storage::headline::{HeadlineMonthlyRow, HeadlineRunSummary};
use crate::storage::Storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadlineConfig {
    pub pool_address: String,
    pub lookback_months: i64,
    /// Per-month inputs: best LP / naive LP / median LP / aave / lido /
    /// hodl returns (fractional, e.g. 0.005 = 0.5% / month). Caller is
    /// responsible for assembling these from M2.5 grid output + M2.7
    /// benchmark fetches.
    pub monthly_inputs: Vec<HeadlineMonthlyInput>,
    /// Daily ETH-spot returns for the same window. Used for vol regime
    /// classification (per plan §M2.8 — adaptive terciles).
    pub eth_daily_returns: Vec<(String, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadlineMonthlyInput {
    pub year_month: String, // YYYY-MM
    pub best_lp_return: f64,
    pub naive_lp_return: f64,
    pub median_lp_return: f64,
    pub aave_usdc_return: f64,
    pub lido_steth_return: f64,
    pub hodl_return: f64,
    /// Per-month return on the S&P 500 (via VOO close-to-close).
    /// Default 0.0 when the caller has no data for the month.
    #[serde(default)]
    pub sp500_return: f64,
    /// Per-month return on gold (LBMA London PM fix).
    #[serde(default)]
    pub gold_return: f64,
    /// Per-month return on a 3-month T-bill (annualised yield / 12).
    #[serde(default)]
    pub tbill_return: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadlineOutput {
    pub summary: HeadlineRunSummary,
    pub monthly: Vec<HeadlineMonthlyRow>,
}

pub struct HeadlineRunner<'a> {
    pub storage: &'a Storage,
}

impl<'a> HeadlineRunner<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }

    pub async fn run(&self, config: HeadlineConfig) -> Result<HeadlineOutput, HeadlineError> {
        if (config.monthly_inputs.len() as i64) < config.lookback_months {
            return Err(HeadlineError::InsufficientData {
                required_months: config.lookback_months,
                actual_months: config.monthly_inputs.len() as i64,
            });
        }

        // Compute per-month vol from daily ETH returns; align by
        // year-month with the monthly_inputs.
        let monthly_vol = monthly_realized_vol(&config.eth_daily_returns);
        let regimes = classify_terciles(&monthly_vol);

        // Build a YYYY-MM -> (vol, regime) lookup so we can join.
        let mut vol_by_ym: std::collections::HashMap<String, (f64, VolRegime)> =
            std::collections::HashMap::new();
        for ((ym, vol), reg) in monthly_vol.iter().zip(regimes.iter()) {
            vol_by_ym.insert(ym.clone(), (*vol, *reg));
        }

        // Per-month rows: classify regime, compute spreads.
        let mut rows = Vec::with_capacity(config.monthly_inputs.len());
        let mut months_lp_beat_lending = 0i64;
        let mut months_lp_beat_sp500 = 0i64;
        let mut months_lp_beat_gold = 0i64;
        let mut months_lp_beat_tbill = 0i64;
        let mut spreads_by_regime: std::collections::HashMap<VolRegime, Vec<f64>> =
            std::collections::HashMap::new();
        let mut sp500_spreads: Vec<f64> = Vec::new();
        let mut gold_spreads: Vec<f64> = Vec::new();
        let mut tbill_spreads: Vec<f64> = Vec::new();

        for input in &config.monthly_inputs {
            let (eth_vol, regime) = vol_by_ym
                .get(&input.year_month)
                .copied()
                .unwrap_or((0.0, VolRegime::Medium));
            let lending_return = input.aave_usdc_return.max(input.lido_steth_return);
            let spread = input.best_lp_return - lending_return;
            if input.best_lp_return > lending_return {
                months_lp_beat_lending += 1;
            }
            spreads_by_regime
                .entry(regime)
                .or_default()
                .push(spread);

            // Multi-asset comparisons. Only count months where the
            // benchmark has a non-zero value (zero means "no data").
            if input.sp500_return.abs() > f64::EPSILON {
                let s = input.best_lp_return - input.sp500_return;
                sp500_spreads.push(s);
                if s > 0.0 {
                    months_lp_beat_sp500 += 1;
                }
            }
            if input.gold_return.abs() > f64::EPSILON {
                let s = input.best_lp_return - input.gold_return;
                gold_spreads.push(s);
                if s > 0.0 {
                    months_lp_beat_gold += 1;
                }
            }
            if input.tbill_return.abs() > f64::EPSILON {
                let s = input.best_lp_return - input.tbill_return;
                tbill_spreads.push(s);
                if s > 0.0 {
                    months_lp_beat_tbill += 1;
                }
            }

            rows.push(HeadlineMonthlyRow {
                year_month: input.year_month.clone(),
                vol_regime: regime.label().to_string(),
                best_lp_return: input.best_lp_return,
                naive_lp_return: input.naive_lp_return,
                median_lp_return: input.median_lp_return,
                aave_usdc_return: input.aave_usdc_return,
                lido_steth_return: input.lido_steth_return,
                hodl_return: input.hodl_return,
                sp500_return: input.sp500_return,
                gold_return: input.gold_return,
                tbill_return: input.tbill_return,
                eth_vol_30d: eth_vol,
            });
        }

        let median_low = median_or_none(spreads_by_regime.get(&VolRegime::Low));
        let median_med = median_or_none(spreads_by_regime.get(&VolRegime::Medium));
        let median_high = median_or_none(spreads_by_regime.get(&VolRegime::High));
        let median_sp500 = median_or_none(Some(&sp500_spreads));
        let median_gold = median_or_none(Some(&gold_spreads));
        let median_tbill = median_or_none(Some(&tbill_spreads));

        let total_months = rows.len() as i64;
        let verdict_text = build_verdict(
            &config.pool_address,
            months_lp_beat_lending,
            months_lp_beat_sp500,
            months_lp_beat_gold,
            months_lp_beat_tbill,
            total_months,
            median_low,
            median_med,
            median_high,
            median_sp500,
            median_gold,
            median_tbill,
        );

        let now_ms = chrono::Utc::now().timestamp_millis();
        let summary = HeadlineRunSummary {
            config_hash: hash_config(&config),
            pool_address: config.pool_address.clone(),
            lookback_months: config.lookback_months,
            regime_method: "adaptive_terciles".to_string(),
            months_lp_beat_lending,
            months_lp_beat_sp500,
            months_lp_beat_gold,
            months_lp_beat_tbill,
            months_total: total_months,
            median_low_vol_spread: median_low,
            median_med_vol_spread: median_med,
            median_high_vol_spread: median_high,
            median_sp500_spread: median_sp500,
            median_gold_spread: median_gold,
            median_tbill_spread: median_tbill,
            verdict_text,
            completed_at_unix_ms: now_ms,
        };

        self.storage
            .persist_headline_run(summary.clone(), rows.clone())
            .await?;

        Ok(HeadlineOutput {
            summary,
            monthly: rows,
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn build_verdict(
    pool: &str,
    won_lending: i64,
    won_sp500: i64,
    won_gold: i64,
    won_tbill: i64,
    total: i64,
    low: Option<f64>,
    med: Option<f64>,
    high: Option<f64>,
    sp500: Option<f64>,
    gold: Option<f64>,
    tbill: Option<f64>,
) -> String {
    let mut s = format!(
        "LP ({pool}) outperformed stable lending in {won_lending} of {total} months."
    );

    // Per-asset month counts: only render the line when the benchmark
    // had any data (months won + spread populated).
    if sp500.is_some() {
        s.push_str(&format!(
            " Beat S&P 500 in {won_sp500}/{total}; median spread {:+.2} pp/mo.",
            sp500.unwrap_or(0.0) * 100.0
        ));
    }
    if gold.is_some() {
        s.push_str(&format!(
            " Beat gold in {won_gold}/{total}; median spread {:+.2} pp/mo.",
            gold.unwrap_or(0.0) * 100.0
        ));
    }
    if tbill.is_some() {
        s.push_str(&format!(
            " Beat 3-mo T-bill in {won_tbill}/{total}; median spread {:+.2} pp/mo.",
            tbill.unwrap_or(0.0) * 100.0
        ));
    }

    if let Some(h) = high {
        s.push_str(&format!(
            " High-vol regime: median spread {:+.2} pp/mo.",
            h * 100.0
        ));
    }
    if let Some(m) = med {
        s.push_str(&format!(
            " Medium-vol: {:+.2} pp/mo.",
            m * 100.0
        ));
    }
    if let Some(l) = low {
        s.push_str(&format!(
            " Low-vol: {:+.2} pp/mo.",
            l * 100.0
        ));
    }

    if won_lending > total - won_lending {
        s.push_str(" Conclusion: V3 LP on this pair was the better default over the lookback.");
    } else if won_lending < total - won_lending {
        let losses_high = high.unwrap_or(0.0) < 0.0;
        let losses_low = low.unwrap_or(0.0) < 0.0;
        if losses_low && !losses_high {
            s.push_str(" Conclusion: V3 LP is a vol-regime-conditional strategy — rotate into LP only when 30-day rolling ETH vol exceeds the third tercile.");
        } else {
            s.push_str(" Conclusion: stable lending was the better default over the lookback.");
        }
    }
    s
}

fn median_or_none(slice: Option<&Vec<f64>>) -> Option<f64> {
    let v = slice?;
    if v.is_empty() {
        return None;
    }
    let mut sorted = v.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    Some(if n % 2 == 1 {
        sorted[n / 2]
    } else {
        0.5 * (sorted[n / 2 - 1] + sorted[n / 2])
    })
}

fn hash_config(c: &HeadlineConfig) -> String {
    let mut hasher = DefaultHasher::new();
    c.pool_address.hash(&mut hasher);
    c.lookback_months.hash(&mut hasher);
    for input in &c.monthly_inputs {
        input.year_month.hash(&mut hasher);
        input.best_lp_return.to_bits().hash(&mut hasher);
        input.aave_usdc_return.to_bits().hash(&mut hasher);
    }
    format!("hl_{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::DbLocation;

    #[tokio::test]
    async fn headline_synthesises_verdict_and_persists() {
        let storage = Storage::open(DbLocation::in_memory()).await.unwrap();
        let mut inputs = Vec::new();
        let mut daily = Vec::new();
        for i in 0..6 {
            let ym = format!("2024-{:02}", i + 1);
            // alternating: best LP wins lending in months 0,2,4 (high vol)
            // and loses in months 1,3,5.
            let best = if i % 2 == 0 { 0.02 } else { -0.01 };
            inputs.push(HeadlineMonthlyInput {
                year_month: ym.clone(),
                best_lp_return: best,
                naive_lp_return: best - 0.005,
                median_lp_return: best - 0.01,
                aave_usdc_return: 0.005,
                lido_steth_return: 0.003,
                hodl_return: 0.0,
                sp500_return: 0.0,
                gold_return: 0.0,
                tbill_return: 0.0,
            });
            for d in 1..=28 {
                let date = format!("2024-{:02}-{:02}", i + 1, d);
                let r = if i % 2 == 0 { 0.05 } else { 0.005 };
                daily.push((date, r * (-1f64).powi(d as i32)));
            }
        }
        let config = HeadlineConfig {
            pool_address: "0xpool".into(),
            lookback_months: 6,
            monthly_inputs: inputs,
            eth_daily_returns: daily,
        };
        let runner = HeadlineRunner::new(&storage);
        let out = runner.run(config).await.unwrap();
        assert_eq!(out.summary.months_lp_beat_lending, 3);
        assert_eq!(out.summary.months_total, 6);
        assert_eq!(out.summary.regime_method, "adaptive_terciles");
        assert!(!out.summary.verdict_text.is_empty());

        // Persisted
        let stored = storage
            .get_headline_run(out.summary.config_hash.clone())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.months_total, 6);
        let monthly_stored = storage
            .get_headline_monthly(out.summary.config_hash)
            .await
            .unwrap();
        assert_eq!(monthly_stored.len(), 6);
    }

    #[tokio::test]
    async fn insufficient_data_returns_error() {
        let storage = Storage::open(DbLocation::in_memory()).await.unwrap();
        let config = HeadlineConfig {
            pool_address: "0xpool".into(),
            lookback_months: 24,
            monthly_inputs: vec![HeadlineMonthlyInput {
                year_month: "2024-01".into(),
                best_lp_return: 0.0,
                naive_lp_return: 0.0,
                median_lp_return: 0.0,
                aave_usdc_return: 0.0,
                lido_steth_return: 0.0,
                hodl_return: 0.0,
                sp500_return: 0.0,
                gold_return: 0.0,
                tbill_return: 0.0,
            }],
            eth_daily_returns: Vec::new(),
        };
        let runner = HeadlineRunner::new(&storage);
        let r = runner.run(config).await;
        assert!(matches!(r, Err(HeadlineError::InsufficientData { .. })));
    }

    #[test]
    fn median_of_odd_length() {
        let v = vec![1.0, 5.0, 3.0];
        assert_eq!(median_or_none(Some(&v)), Some(3.0));
    }

    #[test]
    fn median_of_even_length() {
        let v = vec![1.0, 2.0, 3.0, 4.0];
        assert_eq!(median_or_none(Some(&v)), Some(2.5));
    }

    #[test]
    fn median_of_empty_returns_none() {
        let v: Vec<f64> = Vec::new();
        assert_eq!(median_or_none(Some(&v)), None);
        assert_eq!(median_or_none(None), None);
    }
}
