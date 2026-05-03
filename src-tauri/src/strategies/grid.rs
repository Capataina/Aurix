//! Grid search over (range_width × rebalance_rule × deposit × period).
//!
//! Reference: `vector-a-v3-lp-backtester.md` §M2.5.

use num_bigint::BigUint;
use serde::{Deserialize, Serialize};

use crate::backtest::{Engine, PositionConfig, RebalanceRule};
use crate::math::tick::sqrt_price_x96_to_tick;
use crate::backtest::price::sqrt_price_x96_to_human_price;
use crate::backtest::metrics::deflated_sharpe;
use crate::storage::strategy::StrategyResultRow;
use crate::storage::Storage;

use super::error::StrategyError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridConfig {
    pub grid_id: String,
    pub pool_address: String,
    /// Range half-widths as a percentage of entry price. 5.0 means
    /// ±5% (10% total span); 50.0 means ±50% (essentially full-range).
    pub range_widths_pct: Vec<f64>,
    pub rebalance_rules: Vec<RebalanceRule>,
    /// Deposit sizes in USD; split 50/50 across token0/token1 at entry.
    pub deposits_usd: Vec<f64>,
    /// Lookback windows in days. Each window ends at `period_end_block`.
    pub periods_days: Vec<i64>,
    pub fee_tier_bps: u32,
    pub token0_decimals: u8,
    pub token1_decimals: u8,
    pub mev_haircut_bps: f64,
    pub period_end_block: u64,
    /// Approximate blocks-per-day for the chain (Ethereum: 7200).
    pub blocks_per_day: u64,
}

impl GridConfig {
    pub fn cell_count(&self) -> usize {
        self.range_widths_pct.len()
            * self.rebalance_rules.len()
            * self.deposits_usd.len()
            * self.periods_days.len()
    }

    pub fn validate(&self) -> Result<(), StrategyError> {
        if self.range_widths_pct.is_empty() {
            return Err(StrategyError::InvalidConfig("range_widths_pct empty".into()));
        }
        if self.rebalance_rules.is_empty() {
            return Err(StrategyError::InvalidConfig("rebalance_rules empty".into()));
        }
        if self.deposits_usd.is_empty() {
            return Err(StrategyError::InvalidConfig("deposits_usd empty".into()));
        }
        if self.periods_days.is_empty() {
            return Err(StrategyError::InvalidConfig("periods_days empty".into()));
        }
        if self.blocks_per_day == 0 {
            return Err(StrategyError::InvalidConfig("blocks_per_day must be > 0".into()));
        }
        Ok(())
    }
}

pub struct GridRunner<'a> {
    pub storage: &'a Storage,
}

impl<'a> GridRunner<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }

    pub async fn run_grid(
        &self,
        config: GridConfig,
    ) -> Result<Vec<StrategyResultRow>, StrategyError> {
        config.validate()?;
        let n_strategies = config.cell_count();
        let engine = Engine::new(self.storage);
        let mut results = Vec::with_capacity(n_strategies);

        for &period_days in &config.periods_days {
            let blocks_back = (period_days as u64) * config.blocks_per_day;
            let entry_block = config.period_end_block.saturating_sub(blocks_back);
            let exit_block = config.period_end_block;
            // Resolve the entry sqrtPrice from the first swap in the
            // period. We pull the entry block's swap row to anchor the
            // position composition.
            let entry_sqrt = self
                .resolve_entry_sqrt(&config.pool_address, entry_block, exit_block)
                .await?;
            let entry_tick = sqrt_price_x96_to_tick(&entry_sqrt)
                .map_err(|e| StrategyError::Backtest(e.into()))?;
            let entry_price = sqrt_price_x96_to_human_price(
                &entry_sqrt,
                config.token0_decimals,
                config.token1_decimals,
            );

            for &deposit_usd in &config.deposits_usd {
                // Split 50/50 by USD at entry.
                let half_usd = deposit_usd / 2.0;
                let token0_human = half_usd / entry_price.max(1e-12);
                let token1_human = half_usd;
                let deposit_token0 = human_to_raw(token0_human, config.token0_decimals);
                let deposit_token1 = human_to_raw(token1_human, config.token1_decimals);

                for &half_width_pct in &config.range_widths_pct {
                    // 1 tick ≈ 1 bp on price → 100 ticks per percentage point.
                    let half_ticks = (half_width_pct * 100.0) as i32;
                    let tick_lower = entry_tick - half_ticks;
                    let tick_upper = entry_tick + half_ticks;

                    for rule in &config.rebalance_rules {
                        let position_config = PositionConfig {
                            pool_address: config.pool_address.clone(),
                            tick_lower,
                            tick_upper,
                            deposit_token0: deposit_token0.clone(),
                            deposit_token1: deposit_token1.clone(),
                            entry_block,
                            exit_block,
                            fee_tier_bps: config.fee_tier_bps,
                            token0_decimals: config.token0_decimals,
                            token1_decimals: config.token1_decimals,
                            mev_haircut_bps: config.mev_haircut_bps,
                        };
                        let out = match engine
                            .simulate(position_config.clone(), rule.clone())
                            .await
                        {
                            Ok(o) => o,
                            Err(crate::backtest::error::BacktestError::EmptyData { .. }) => {
                                continue;
                            }
                            Err(e) => return Err(StrategyError::Backtest(e)),
                        };
                        let summary = out.summary;

                        let dsr = deflated_sharpe(summary.sharpe, n_strategies);
                        let period_start_unix_ms = summary
                            .completed_at_unix_ms
                            - (period_days * 86_400_000);
                        let net_return_vs_hold =
                            summary.final_value_usd - summary.hold_only_value_usd;
                        let row = StrategyResultRow {
                            grid_id: config.grid_id.clone(),
                            pool_address: config.pool_address.clone(),
                            range_width_pct: half_width_pct * 2.0,
                            rebalance_rule: rule.label(),
                            deposit_usd,
                            period_days,
                            period_start_unix_ms,
                            period_end_unix_ms: summary.completed_at_unix_ms,
                            fees_usd: summary.total_fees_usd,
                            il_usd: summary.total_il_usd,
                            lvr_usd: summary.total_lvr_usd,
                            mgmt_gas_usd: summary.total_mgmt_gas_usd,
                            net_return_usd: summary.net_pnl_usd,
                            net_return_vs_hold,
                            time_in_range_pct: summary.time_in_range_pct,
                            rebalance_count: summary.rebalance_count,
                            max_drawdown_pct: summary.max_drawdown_pct,
                            sharpe: summary.sharpe,
                            sortino: summary.sortino,
                            calmar: summary.calmar,
                            deflated_sharpe: dsr,
                            completed_at_unix_ms: summary.completed_at_unix_ms,
                        };
                        results.push(row);
                    }
                }
            }
        }

        if !results.is_empty() {
            self.storage
                .insert_strategy_results_batch(results.clone())
                .await
                .map_err(StrategyError::Storage)?;
        }
        Ok(results)
    }

    async fn resolve_entry_sqrt(
        &self,
        pool: &str,
        entry_block: u64,
        exit_block: u64,
    ) -> Result<BigUint, StrategyError> {
        let swaps = self
            .storage
            .query_swaps_for_pool_range(pool.to_string(), entry_block as i64, exit_block as i64)
            .await
            .map_err(StrategyError::Storage)?;
        let first = swaps.into_iter().next().ok_or(StrategyError::NoData)?;
        BigUint::parse_bytes(first.sqrt_price_x96.as_bytes(), 10)
            .ok_or_else(|| StrategyError::InvalidConfig("malformed sqrt_price_x96".into()))
    }
}

fn human_to_raw(human: f64, decimals: u8) -> String {
    if human <= 0.0 || !human.is_finite() {
        return "0".to_string();
    }
    let scaled = human * 10f64.powi(decimals as i32);
    let big = BigUint::from(scaled.round() as u128);
    big.to_string()
}

#[cfg(test)]
mod tests {
    // Avoid unused-import warnings — the function `tick_to_sqrt_price_x96`
    // is referenced in test helpers via the `crate::math::*` module path
    // already imported elsewhere.
    use super::super::tests as parent_tests;
    use super::*;

    #[test]
    fn cell_count_multiplies_axes() {
        let c = GridConfig {
            grid_id: "g1".into(),
            pool_address: "0x".into(),
            range_widths_pct: vec![1.0, 5.0, 10.0],
            rebalance_rules: vec![RebalanceRule::Static, RebalanceRule::Schedule { every_n_blocks: 10 }],
            deposits_usd: vec![1_000.0, 10_000.0],
            periods_days: vec![30, 60],
            fee_tier_bps: 5,
            token0_decimals: 18,
            token1_decimals: 6,
            mev_haircut_bps: 0.0,
            period_end_block: 1000,
            blocks_per_day: 7200,
        };
        assert_eq!(c.cell_count(), 3 * 2 * 2 * 2);
    }

    #[test]
    fn human_to_raw_rounds_to_decimals() {
        assert_eq!(human_to_raw(1.0, 18), "1000000000000000000");
        assert_eq!(human_to_raw(100.0, 6), "100000000");
        assert_eq!(human_to_raw(0.0, 18), "0");
        assert_eq!(human_to_raw(-1.0, 6), "0");
    }

    #[test]
    fn validate_rejects_empty_axes() {
        let mut c = GridConfig {
            grid_id: "g".into(),
            pool_address: "0x".into(),
            range_widths_pct: vec![1.0],
            rebalance_rules: vec![RebalanceRule::Static],
            deposits_usd: vec![1.0],
            periods_days: vec![30],
            fee_tier_bps: 5,
            token0_decimals: 18,
            token1_decimals: 6,
            mev_haircut_bps: 0.0,
            period_end_block: 1000,
            blocks_per_day: 7200,
        };
        assert!(c.validate().is_ok());
        c.range_widths_pct = Vec::new();
        assert!(c.validate().is_err());
    }

    // ensure parent test re-export still resolves
    #[allow(dead_code)]
    fn _link() {
        let _ = parent_tests::dummy_link();
    }
}
