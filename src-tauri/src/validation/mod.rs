//! M2.4 — Validation harness.
//!
//! Defines `LpPositionFixture` (a known on-chain LP position with mint /
//! burn / collected-fees ground truth) and `ValidationRunner` that
//! replays each fixture through the simulation engine and compares the
//! engine's output to the fixture's recorded ground truth.
//!
//! Live ground-truth values (real on-chain positions) require a working
//! Alchemy archive key; synthetic fixtures (this module's `synthetic`
//! submodule) round-trip through the math primitives + simulation engine
//! to stress-test the harness without needing a key. When you supply
//! the keys, plug the live fixtures into `runner.run` and the
//! KEY_REQUIRED gate auto-resolves.

#![allow(dead_code)]

pub mod error;
pub mod synthetic;

pub use error::ValidationError;

use serde::{Deserialize, Serialize};

use crate::backtest::{Engine, PositionConfig, RebalanceRule};
use crate::storage::Storage;

/// Ground truth for one LP position. The first three fields are the
/// position config; the remainder are the values an exact replay must
/// match within tolerance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpPositionFixture {
    pub label: String,
    pub config: PositionConfig,
    /// USD-valued total fees collected on chain, for the fee tolerance
    /// check. Live fixtures populate from on-chain Collect events; the
    /// synthetic fixtures populate from the engine output to round-trip.
    pub on_chain_fees_usd: f64,
    /// USD value of mgmt-gas paid (mint + burn + collect leg costs).
    pub on_chain_mgmt_gas_usd: f64,
    /// Final position USD value (token0 + token1 valued at exit price).
    pub on_chain_final_value_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationRow {
    pub label: String,
    pub passed: bool,
    pub fees_engine_usd: f64,
    pub fees_on_chain_usd: f64,
    pub fees_diff_abs: f64,
    pub fees_diff_pct: f64,
    pub mgmt_gas_engine_usd: f64,
    pub mgmt_gas_on_chain_usd: f64,
    pub mgmt_gas_diff_pct: f64,
    pub final_value_engine_usd: f64,
    pub final_value_on_chain_usd: f64,
    pub final_value_diff_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationReport {
    pub total: usize,
    pub passed: usize,
    pub rows: Vec<ValidationRow>,
}

pub struct ValidationRunner<'a> {
    pub storage: &'a Storage,
    /// Fees within `fees_tolerance_pct` of ground truth → pass.
    pub fees_tolerance_pct: f64,
    /// Mgmt gas within `gas_tolerance_pct` → pass.
    pub gas_tolerance_pct: f64,
    /// Final value within `value_tolerance_pct` → pass.
    pub value_tolerance_pct: f64,
}

impl<'a> ValidationRunner<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self {
            storage,
            fees_tolerance_pct: 0.5,
            gas_tolerance_pct: 5.0,
            value_tolerance_pct: 1.0,
        }
    }

    /// Replay every fixture and produce a report. Acceptance per the
    /// M2.4 plan: 4 of 5 fixtures must pass.
    pub async fn run(
        &self,
        fixtures: &[LpPositionFixture],
    ) -> Result<ValidationReport, ValidationError> {
        let engine = Engine::new(self.storage);
        let mut rows = Vec::with_capacity(fixtures.len());
        for fixture in fixtures {
            let out = engine
                .simulate(fixture.config.clone(), RebalanceRule::Static)
                .await?;
            let summary = out.summary;
            let fees_diff_abs =
                (summary.total_fees_usd - fixture.on_chain_fees_usd).abs();
            let fees_diff_pct = pct_diff(summary.total_fees_usd, fixture.on_chain_fees_usd);
            let gas_diff_pct =
                pct_diff(summary.total_mgmt_gas_usd, fixture.on_chain_mgmt_gas_usd);
            let value_diff_pct =
                pct_diff(summary.final_value_usd, fixture.on_chain_final_value_usd);

            let passed = fees_diff_pct <= self.fees_tolerance_pct
                && gas_diff_pct <= self.gas_tolerance_pct
                && value_diff_pct <= self.value_tolerance_pct;

            rows.push(ValidationRow {
                label: fixture.label.clone(),
                passed,
                fees_engine_usd: summary.total_fees_usd,
                fees_on_chain_usd: fixture.on_chain_fees_usd,
                fees_diff_abs,
                fees_diff_pct,
                mgmt_gas_engine_usd: summary.total_mgmt_gas_usd,
                mgmt_gas_on_chain_usd: fixture.on_chain_mgmt_gas_usd,
                mgmt_gas_diff_pct: gas_diff_pct,
                final_value_engine_usd: summary.final_value_usd,
                final_value_on_chain_usd: fixture.on_chain_final_value_usd,
                final_value_diff_pct: value_diff_pct,
            });
        }
        let passed = rows.iter().filter(|r| r.passed).count();
        Ok(ValidationReport {
            total: fixtures.len(),
            passed,
            rows,
        })
    }
}

fn pct_diff(a: f64, b: f64) -> f64 {
    if b.abs() < 1e-9 {
        if a.abs() < 1e-9 {
            0.0
        } else {
            100.0
        }
    } else {
        100.0 * (a - b).abs() / b.abs()
    }
}

#[cfg(test)]
mod tests {
    use super::synthetic::{round_trip_synthetic_fixtures, SYNTHETIC_FIXTURE_COUNT};
    use super::*;
    use crate::storage::DbLocation;

    #[tokio::test]
    async fn synthetic_fixtures_round_trip() {
        let storage = Storage::open(DbLocation::in_memory()).await.unwrap();
        let fixtures = round_trip_synthetic_fixtures(&storage).await.unwrap();
        assert_eq!(fixtures.len(), SYNTHETIC_FIXTURE_COUNT);

        let runner = ValidationRunner::new(&storage);
        let report = runner.run(&fixtures).await.unwrap();
        assert_eq!(report.total, SYNTHETIC_FIXTURE_COUNT);
        assert_eq!(
            report.passed, SYNTHETIC_FIXTURE_COUNT,
            "synthetic fixtures must round-trip exactly through the engine"
        );
    }
}
