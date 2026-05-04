//! Position simulation engine — the core of M2.3.
//!
//! Walks every swap in `[entry_block, exit_block]` for a single pool,
//! computes per-swap fee accrual when the position is in range, tracks
//! impermanent loss vs the hold-only baseline, accumulates LVR per the
//! Milionis-Moallemi-Roughgarden discrete approximation, applies
//! management-gas costs at chain-historical block-level prices, and
//! emits a per-sample equity curve.
//!
//! Reference: `vector-a-v3-lp-backtester.md` §M2.3.
//!
//! Audit findings applied (see `context/plans/code-health-audit/backtest.md`):
//!   - Pre-parse swap rows once into `ParsedSwap` (Data Layout / Memory Access).
//!   - Hoist invariant hold-only USD computation out of the per-swap loop.
//!   - Accumulate fees-USD incrementally instead of re-converting accumulators.
//!   - Magic-constant precompute lives in `math/tick.rs`.

use num_bigint::{BigInt, BigUint};
use num_traits::{ToPrimitive, Zero};

use crate::math::fees::{bps_to_protocol_units, fee_share_token0, fee_share_token1};
use crate::math::liquidity::{amounts_for_liquidity, liquidity_for_amounts};
use crate::math::tick::tick_to_sqrt_price_x96;
use crate::storage::runs::{EquityCurvePoint, PositionRunSummary};
use crate::storage::swaps::SwapEventRow;
use crate::storage::Storage;

use super::error::BacktestError;
use super::gas::{cost_usd, mev_haircut_usd, MgmtGasOp};
use super::metrics::{
    annualise, block_equity_to_daily_returns, calmar_ratio, max_drawdown_pct, sharpe_ratio,
    sortino_ratio,
};
use super::position::PositionConfig;
use super::price::{position_usd_value, position_usd_value_explicit, sqrt_price_x96_to_human_price};
use super::rebalance::{RebalanceContext, RebalanceRule};

/// Simulation output: persisted summary + full equity curve.
#[derive(Debug, Clone)]
pub struct SimulationOutput {
    pub summary: PositionRunSummary,
    pub equity_curve: Vec<EquityCurvePoint>,
}

pub struct Engine<'a> {
    pub storage: &'a Storage,
}

/// Pre-parsed swap row. The storage layer keeps big integers as decimal
/// TEXT for full uint160 / int256 / uint128 precision; the engine pays
/// the parse cost exactly once per row by walking the loaded swap vec
/// up-front into this denser representation. Closes the per-loop
/// allocation finding in `code-health-audit/backtest.md`.
struct ParsedSwap {
    block_number: i64,
    block_timestamp: i64,
    block_gas_price_gwei: Option<f64>,
    sqrt_price_x96: BigUint,
    liquidity: u128,
    amount0_in: BigUint,
    amount1_in: BigUint,
    tick: i32,
}

fn parse_swaps(rows: Vec<SwapEventRow>) -> Result<Vec<ParsedSwap>, BacktestError> {
    rows.into_iter()
        .map(|s| {
            let sqrt_price_x96 = BigUint::parse_bytes(s.sqrt_price_x96.as_bytes(), 10)
                .ok_or_else(|| {
                    BacktestError::Parse(format!("sqrt_price_x96 from '{}'", s.sqrt_price_x96))
                })?;
            let liquidity = s
                .liquidity
                .parse::<u128>()
                .map_err(|_| BacktestError::Parse(format!("liquidity from '{}'", s.liquidity)))?;
            let amount0_signed = BigInt::parse_bytes(s.amount0.as_bytes(), 10)
                .ok_or_else(|| BacktestError::Parse(format!("amount0 from '{}'", s.amount0)))?;
            let amount1_signed = BigInt::parse_bytes(s.amount1.as_bytes(), 10)
                .ok_or_else(|| BacktestError::Parse(format!("amount1 from '{}'", s.amount1)))?;
            let zero = BigInt::from(0u8);
            let amount0_in = if amount0_signed > zero {
                amount0_signed.to_biguint().unwrap_or_default()
            } else {
                BigUint::from(0u8)
            };
            let zero = BigInt::from(0u8);
            let amount1_in = if amount1_signed > zero {
                amount1_signed.to_biguint().unwrap_or_default()
            } else {
                BigUint::from(0u8)
            };
            Ok(ParsedSwap {
                block_number: s.block_number,
                block_timestamp: s.block_timestamp,
                block_gas_price_gwei: s.block_gas_price_gwei,
                sqrt_price_x96,
                liquidity,
                amount0_in,
                amount1_in,
                tick: s.tick,
            })
        })
        .collect()
}

/// Hold-only USD evaluator. Initialised once per simulate call and
/// closed over the deposit token amounts (which are loop-invariant).
/// The `eval(cur_price)` call in the per-swap loop is then a constant-
/// time multiply-add — no `BigUint::to_f64`, no `value_usd` closure
/// dispatch, no `position_usd_value` re-conversion. Closes the audit
/// finding `backtest.md` §"Hoist invariant USD-conversion of hold-only
/// baseline".
struct HoldOnlyEvaluator {
    /// Pre-computed hold-only USD when token0_usd_price + token1_usd_price
    /// are configured. Constant across the run.
    explicit: Option<f64>,
    /// Pre-computed decimal-adjusted token0 amount, or 0.0 when explicit
    /// pricing is configured (path unused).
    a0_decimal: f64,
    /// Pre-computed decimal-adjusted token1 amount.
    a1_decimal: f64,
}

impl HoldOnlyEvaluator {
    fn new(deposit0: &BigUint, deposit1: &BigUint, config: &PositionConfig) -> Self {
        let a0_decimal = deposit0.to_f64().unwrap_or(0.0)
            / 10f64.powi(config.token0_decimals as i32);
        let a1_decimal = deposit1.to_f64().unwrap_or(0.0)
            / 10f64.powi(config.token1_decimals as i32);
        let explicit = match (config.token0_usd_price, config.token1_usd_price) {
            (Some(p0), Some(p1)) => Some(a0_decimal * p0 + a1_decimal * p1),
            _ => None,
        };
        Self {
            explicit,
            a0_decimal,
            a1_decimal,
        }
    }

    /// Evaluate hold-only USD at `cur_price` (token1-per-token0). When
    /// explicit pricing is configured the price is ignored and the
    /// pre-computed constant is returned.
    fn eval(&self, cur_price: f64) -> f64 {
        match self.explicit {
            Some(v) => v,
            None => self.a0_decimal * cur_price + self.a1_decimal,
        }
    }
}

impl<'a> Engine<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }

    /// Runs a single backtest end-to-end. Reads swap events from storage,
    /// replays them against the position, and returns the simulation
    /// output. Caller is responsible for persisting via
    /// `storage.persist_position_run` if they want the run cached.
    pub async fn simulate(
        &self,
        config: PositionConfig,
        rule: RebalanceRule,
    ) -> Result<SimulationOutput, BacktestError> {
        config.validate()?;

        let swaps_raw = self
            .storage
            .query_swaps_for_pool_range(
                config.pool_address.clone(),
                config.entry_block as i64,
                config.exit_block as i64,
            )
            .await?;
        if swaps_raw.is_empty() {
            return Err(BacktestError::EmptyData {
                pool: config.pool_address.clone(),
                from: config.entry_block,
                to: config.exit_block,
            });
        }

        // Pre-parse all swap rows into a denser in-memory representation.
        // Pays the parse cost once per row instead of N times in the
        // per-swap loop.
        let swaps = parse_swaps(swaps_raw)?;

        // Initialise position state from the first swap's price.
        let first_swap = &swaps[0];
        let entry_sqrt = first_swap.sqrt_price_x96.clone();
        let entry_price = sqrt_price_x96_to_human_price(
            &entry_sqrt,
            config.token0_decimals,
            config.token1_decimals,
        );

        let mut sqrt_lower = config.sqrt_lower()?;
        let mut sqrt_upper = config.sqrt_upper()?;
        let mut tick_lower = config.tick_lower;
        let mut tick_upper = config.tick_upper;

        let deposit0 = config.deposit_token0_big()?;
        let deposit1 = config.deposit_token1_big()?;

        // Compute initial liquidity from deposit + entry price.
        let mut liquidity = liquidity_for_amounts(
            &entry_sqrt,
            &sqrt_lower,
            &sqrt_upper,
            &deposit0,
            &deposit1,
        )?;

        // Hold-only baseline: pre-computed once, closure-free per-step
        // evaluation.
        let hold_eval = HoldOnlyEvaluator::new(&deposit0, &deposit1, &config);

        // Closure that picks between the pool-ratio-based USD valuation
        // and the explicit per-token USD valuation. Used for the *current*
        // amounts (which change per step); not used for hold-only or fees
        // any more — those have specialised paths below.
        let value_usd = |a0: &BigUint, a1: &BigUint, ratio: f64| -> f64 {
            match (config.token0_usd_price, config.token1_usd_price) {
                (Some(p0), Some(p1)) => position_usd_value_explicit(
                    a0,
                    a1,
                    p0,
                    p1,
                    config.token0_decimals,
                    config.token1_decimals,
                ),
                _ => position_usd_value(
                    a0,
                    a1,
                    ratio,
                    config.token0_decimals,
                    config.token1_decimals,
                ),
            }
        };
        let entry_position_usd = value_usd(&deposit0, &deposit1, entry_price);

        // Running aggregates.
        let mut fees_token0_acc = BigUint::from(0u8);
        let mut fees_token1_acc = BigUint::from(0u8);
        // Incremental fees-USD accumulator. Closes audit finding
        // `backtest.md` §"Accumulate fees-USD incrementally". Each
        // step adds value_usd(delta_f0, delta_f1, cur_price) instead
        // of re-converting the accumulators (which would walk the
        // growing BigUint digit vector every iteration).
        let mut fees_usd_acc = 0.0f64;
        let mut lvr_usd_acc = 0.0f64;
        let mut mgmt_gas_acc_usd = 0.0f64;
        let mut last_rebalance_block = config.entry_block;
        let mut blocks_oor_streak = 0u64;
        let mut rebalance_count = 0u64;
        let fee_units = bps_to_protocol_units(config.fee_tier_bps);

        // Pay mint cost at entry, priced at first swap's gas.
        let first_gas_gwei = first_swap.block_gas_price_gwei.unwrap_or(20.0);
        mgmt_gas_acc_usd += cost_usd(MgmtGasOp::Mint, first_gas_gwei, entry_price);

        let mut equity_points: Vec<EquityCurvePoint> = Vec::with_capacity(swaps.len());

        // Per-swap walk.
        let mut prev_sqrt = entry_sqrt.clone();
        for (idx, swap) in swaps.iter().enumerate() {
            let cur_sqrt = &swap.sqrt_price_x96;
            let cur_tick = swap.tick;
            let cur_price = sqrt_price_x96_to_human_price(
                cur_sqrt,
                config.token0_decimals,
                config.token1_decimals,
            );
            let active_liquidity = swap.liquidity;

            let in_range = cur_tick >= tick_lower && cur_tick < tick_upper;
            if in_range {
                blocks_oor_streak = 0;
            } else {
                blocks_oor_streak = blocks_oor_streak.saturating_add(1);
            }

            // Fees for this swap (zero if out of range or active_L is 0).
            let f0 = fee_share_token0(
                &swap.amount0_in,
                fee_units,
                liquidity,
                active_liquidity,
                in_range,
            )?;
            let f1 = fee_share_token1(
                &swap.amount1_in,
                fee_units,
                liquidity,
                active_liquidity,
                in_range,
            )?;
            // Incrementally add USD-converted delta fees so we never
            // re-walk the (monotonically growing) accumulator BigUints.
            let delta_fees_usd = value_usd(&f0, &f1, cur_price);
            fees_usd_acc += delta_fees_usd;
            fees_token0_acc += &f0;
            fees_token1_acc += &f1;

            // LVR: discrete approximation per Milionis-Moallemi-Roughgarden,
            // contributes (sqrtPrice_change)^2 * L / sqrtPrice when in
            // range. Convert to USD via current price.
            if in_range && !prev_sqrt.is_zero() {
                let cur_f = cur_sqrt.to_f64().unwrap_or(0.0);
                let prev_f = prev_sqrt.to_f64().unwrap_or(0.0);
                if prev_f > 0.0 && cur_f > 0.0 {
                    let delta = cur_f - prev_f;
                    let l_f = liquidity as f64;
                    // Position value scale factor — divide by sqrtPrice
                    // and Q96 to keep units consistent.
                    let q96 = (1u128 << 96) as f64;
                    let lvr_token1 = 0.5 * delta * delta * l_f / (cur_f * q96);
                    let lvr_usd = lvr_token1 / 10f64.powi(config.token1_decimals as i32);
                    lvr_usd_acc += lvr_usd.max(0.0);
                }
            }

            // Rebalance check.
            let blocks_since_rebalance =
                (swap.block_number as u64).saturating_sub(last_rebalance_block);
            let ctx = RebalanceContext {
                current_block: swap.block_number as u64,
                blocks_since_last_rebalance: blocks_since_rebalance,
                current_tick: cur_tick,
                tick_lower,
                tick_upper,
                blocks_out_of_range: blocks_oor_streak,
            };
            if rule.should_rebalance(&ctx) {
                // Pay rebalance gas.
                let rebalance_gas_gwei = swap.block_gas_price_gwei.unwrap_or(first_gas_gwei);
                mgmt_gas_acc_usd += cost_usd(MgmtGasOp::Rebalance, rebalance_gas_gwei, cur_price);
                // MEV haircut on the rebalance leg, if configured.
                let (a0_now, a1_now) =
                    amounts_for_liquidity(cur_sqrt, &sqrt_lower, &sqrt_upper, liquidity)?;
                let position_value_now = value_usd(&a0_now, &a1_now, cur_price);
                if config.mev_haircut_bps > 0.0 {
                    mgmt_gas_acc_usd +=
                        mev_haircut_usd(position_value_now, config.mev_haircut_bps);
                }
                // Re-centre the range around current tick. Width is
                // preserved (range_half * 2 ticks wide).
                let range_half = (tick_upper - tick_lower) / 2;
                tick_lower = cur_tick - range_half;
                tick_upper = cur_tick + range_half;
                sqrt_lower = tick_to_sqrt_price_x96(tick_lower)?;
                sqrt_upper = tick_to_sqrt_price_x96(tick_upper)?;
                liquidity = liquidity_for_amounts(
                    cur_sqrt,
                    &sqrt_lower,
                    &sqrt_upper,
                    &a0_now,
                    &a1_now,
                )?;
                last_rebalance_block = swap.block_number as u64;
                rebalance_count += 1;
                blocks_oor_streak = 0;
            }

            // Position USD value at this step.
            let (a0_cur, a1_cur) =
                amounts_for_liquidity(cur_sqrt, &sqrt_lower, &sqrt_upper, liquidity)?;
            let raw_position_value = value_usd(&a0_cur, &a1_cur, cur_price);
            let position_value_usd = raw_position_value + fees_usd_acc;

            // Hold-only revalued at this step (constant-time evaluator).
            let hold_only_usd = hold_eval.eval(cur_price);
            // Impermanent loss = LP token value (excluding fees earned)
            // minus the hold-only baseline. Negative when the LP is
            // worse off than holding both tokens 50/50 at the same
            // price. Positive only if the LP rebalanced into the
            // appreciating asset — uncommon for V3 LPs without fees.
            // Fees are accounted separately so the user can see the
            // gross fee earnings vs the gross IL drag, then net them.
            let il_usd = raw_position_value - hold_only_usd;
            let net_pnl_usd = position_value_usd - entry_position_usd - mgmt_gas_acc_usd;

            equity_points.push(EquityCurvePoint {
                sample_idx: idx as i64,
                block_number: swap.block_number,
                block_timestamp: swap.block_timestamp,
                position_value_usd,
                fees_accumulated_usd: fees_usd_acc,
                il_usd,
                lvr_usd: lvr_usd_acc,
                mgmt_gas_paid_usd: mgmt_gas_acc_usd,
                hold_only_value_usd: hold_only_usd,
                net_pnl_usd,
                in_range,
            });

            // Move the sqrt forward by reference-clone (cheap — BigUint
            // clone is a Vec clone, but the alternative is parsing
            // every row again).
            prev_sqrt = cur_sqrt.clone();
        }

        // Pay burn cost at exit, priced at last swap's gas.
        let last_swap = swaps.last().unwrap();
        let last_gas_gwei = last_swap.block_gas_price_gwei.unwrap_or(first_gas_gwei);
        let last_price = sqrt_price_x96_to_human_price(
            &last_swap.sqrt_price_x96,
            config.token0_decimals,
            config.token1_decimals,
        );
        mgmt_gas_acc_usd += cost_usd(MgmtGasOp::Burn, last_gas_gwei, last_price);

        // Update the final equity point's mgmt_gas to include the burn.
        if let Some(last_pt) = equity_points.last_mut() {
            last_pt.mgmt_gas_paid_usd = mgmt_gas_acc_usd;
            last_pt.net_pnl_usd =
                last_pt.position_value_usd - entry_position_usd - mgmt_gas_acc_usd;
        }

        // Aggregates for the run summary.
        let last_pt = equity_points.last().cloned().unwrap();
        let total_fees_usd = last_pt.fees_accumulated_usd;
        let total_il_usd = last_pt.il_usd;
        let total_lvr_usd = lvr_usd_acc;
        let total_mgmt_gas_usd = mgmt_gas_acc_usd;
        let final_value_usd = last_pt.position_value_usd;
        let hold_only_value_usd = last_pt.hold_only_value_usd;
        let net_pnl_usd = last_pt.net_pnl_usd;

        let in_range_count = equity_points.iter().filter(|p| p.in_range).count();
        let time_in_range_pct = 100.0 * in_range_count as f64 / equity_points.len() as f64;

        let equity_series: Vec<f64> = equity_points
            .iter()
            .map(|p| p.position_value_usd)
            .collect();
        let timestamps: Vec<i64> = equity_points
            .iter()
            .map(|p| p.block_timestamp)
            .collect();
        let max_dd = max_drawdown_pct(&equity_series);
        let daily_returns = block_equity_to_daily_returns(&timestamps, &equity_series);
        let sharpe = sharpe_ratio(&daily_returns, 0.0);
        let sortino = sortino_ratio(&daily_returns, 0.0);
        let annualised = annualise(&daily_returns);
        let calmar = calmar_ratio(annualised, max_dd);

        let summary = PositionRunSummary {
            config_hash: config.config_hash(),
            pool_address: config.pool_address.clone(),
            tick_lower: config.tick_lower,
            tick_upper: config.tick_upper,
            deposit_token0: config.deposit_token0.clone(),
            deposit_token1: config.deposit_token1.clone(),
            entry_block: config.entry_block as i64,
            exit_block: config.exit_block as i64,
            rebalance_rule: rule.label(),
            mev_haircut_bps: config.mev_haircut_bps,
            total_fees_usd,
            total_il_usd,
            total_lvr_usd,
            total_mgmt_gas_usd,
            final_value_usd,
            hold_only_value_usd,
            net_pnl_usd,
            time_in_range_pct,
            rebalance_count: rebalance_count as i64,
            max_drawdown_pct: max_dd * 100.0,
            sharpe,
            sortino,
            calmar,
            completed_at_unix_ms: chrono::Utc::now().timestamp_millis(),
        };

        // The fees BigUint accumulators are not part of the summary —
        // they exist for any future caller that wants the raw token-units
        // breakdown. Suppress unused-variable warning while keeping them
        // available for that future caller.
        let _ = (fees_token0_acc, fees_token1_acc);

        Ok(SimulationOutput {
            summary,
            equity_curve: equity_points,
        })
    }
}
