//! Position config + per-step state.

use num_bigint::BigUint;
use serde::{Deserialize, Serialize};

use crate::math::tick_to_sqrt_price_x96;

use super::error::BacktestError;

/// Configuration for one simulated LP position.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionConfig {
    pub pool_address: String,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub deposit_token0: String, // raw uint256 as decimal string
    pub deposit_token1: String, // raw uint256
    pub entry_block: u64,
    pub exit_block: u64,
    /// Pool fee tier in friendly bps (5, 30, 100, 10000).
    pub fee_tier_bps: u32,
    pub token0_decimals: u8,
    pub token1_decimals: u8,
    /// MEV haircut bps applied per rebalance leg (per plan paper 11).
    pub mev_haircut_bps: f64,
    /// Per-token USD prices (DefiLlama). When both are supplied, the
    /// engine values the position via `a0 * p0_usd + a1 * p1_usd`
    /// instead of assuming token1 is USD-pegged. Required for non-
    /// USD-quote pools (WBTC/ETH, LDO/ETH, etc.). Optional for
    /// backward compat — when missing, the engine falls back to the
    /// `cur_price` ratio with the token1=USD assumption.
    #[serde(default)]
    pub token0_usd_price: Option<f64>,
    #[serde(default)]
    pub token1_usd_price: Option<f64>,
}

impl PositionConfig {
    pub fn validate(&self) -> Result<(), BacktestError> {
        if self.tick_lower >= self.tick_upper {
            return Err(BacktestError::InvalidConfig(format!(
                "tick_lower ({}) must be < tick_upper ({})",
                self.tick_lower, self.tick_upper
            )));
        }
        if self.entry_block > self.exit_block {
            return Err(BacktestError::InvertedBlocks {
                entry: self.entry_block,
                exit: self.exit_block,
            });
        }
        Ok(())
    }

    pub fn deposit_token0_big(&self) -> Result<BigUint, BacktestError> {
        BigUint::parse_bytes(self.deposit_token0.as_bytes(), 10)
            .ok_or_else(|| BacktestError::Parse(format!("deposit_token0: {}", self.deposit_token0)))
    }
    pub fn deposit_token1_big(&self) -> Result<BigUint, BacktestError> {
        BigUint::parse_bytes(self.deposit_token1.as_bytes(), 10)
            .ok_or_else(|| BacktestError::Parse(format!("deposit_token1: {}", self.deposit_token1)))
    }

    pub fn sqrt_lower(&self) -> Result<BigUint, BacktestError> {
        Ok(tick_to_sqrt_price_x96(self.tick_lower)?)
    }
    pub fn sqrt_upper(&self) -> Result<BigUint, BacktestError> {
        Ok(tick_to_sqrt_price_x96(self.tick_upper)?)
    }

    /// Stable hash for idempotent persistence in storage::position_runs.
    pub fn config_hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.pool_address.hash(&mut hasher);
        self.tick_lower.hash(&mut hasher);
        self.tick_upper.hash(&mut hasher);
        self.deposit_token0.hash(&mut hasher);
        self.deposit_token1.hash(&mut hasher);
        self.entry_block.hash(&mut hasher);
        self.exit_block.hash(&mut hasher);
        self.fee_tier_bps.hash(&mut hasher);
        // mev_haircut_bps as bits to keep hash deterministic
        self.mev_haircut_bps.to_bits().hash(&mut hasher);
        format!("pos_{:016x}", hasher.finish())
    }
}
