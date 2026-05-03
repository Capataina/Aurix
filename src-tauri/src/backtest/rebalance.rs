//! Rebalance rules — first-class strategy axis per M2.5.
//!
//! Each rule decides, given the current position state, whether to
//! rebalance at this step. The four canonical rules from
//! `vector-a-v3-lp-backtester.md` §M2.5:
//!
//! 1. `Static`            — set range at entry, never rebalance.
//! 2. `Schedule`          — rebalance every N blocks.
//! 3. `PriceExitThreshold`— rebalance when price exits the central X%
//!    of the active range (e.g. 0.5 ⇒ outer 25% on either side).
//! 4. `OutOfRangeDuration`— rebalance after Y consecutive
//!    out-of-range blocks.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RebalanceRule {
    Static,
    Schedule {
        every_n_blocks: u64,
    },
    PriceExitThreshold {
        /// Central fraction of the range (0.5 = central 50%; rebalance
        /// when price hits the outer 25% on either side).
        central_pct: f64,
    },
    OutOfRangeDuration {
        /// Minimum consecutive out-of-range blocks before a rebalance
        /// fires.
        min_oor_blocks: u64,
    },
}

impl RebalanceRule {
    pub fn label(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "unknown".to_string())
    }
}

/// Per-step rebalance decision context.
#[derive(Debug, Clone)]
pub struct RebalanceContext {
    pub current_block: u64,
    pub blocks_since_last_rebalance: u64,
    pub current_tick: i32,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub blocks_out_of_range: u64,
}

impl RebalanceRule {
    /// Returns true when the rule fires a rebalance for `ctx`.
    pub fn should_rebalance(&self, ctx: &RebalanceContext) -> bool {
        match self {
            Self::Static => false,
            Self::Schedule { every_n_blocks } => {
                *every_n_blocks > 0 && ctx.blocks_since_last_rebalance >= *every_n_blocks
            }
            Self::PriceExitThreshold { central_pct } => {
                let central = central_pct.clamp(0.0, 1.0);
                // Width of the central band, in tick units.
                let half_outer = (1.0 - central) / 2.0;
                let span = (ctx.tick_upper - ctx.tick_lower) as f64;
                let center = (ctx.tick_lower + ctx.tick_upper) as f64 / 2.0;
                let half_central = span * (1.0 - half_outer * 2.0) / 2.0;
                let inner_lo = center - half_central;
                let inner_hi = center + half_central;
                let t = ctx.current_tick as f64;
                t < inner_lo || t > inner_hi
            }
            Self::OutOfRangeDuration { min_oor_blocks } => {
                *min_oor_blocks > 0 && ctx.blocks_out_of_range >= *min_oor_blocks
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(current_tick: i32, blocks_since: u64, oor: u64) -> RebalanceContext {
        RebalanceContext {
            current_block: 100,
            blocks_since_last_rebalance: blocks_since,
            current_tick,
            tick_lower: -100,
            tick_upper: 100,
            blocks_out_of_range: oor,
        }
    }

    #[test]
    fn static_never_rebalances() {
        let r = RebalanceRule::Static;
        assert!(!r.should_rebalance(&ctx(0, 1_000_000, 1_000_000)));
    }

    #[test]
    fn schedule_fires_at_threshold() {
        let r = RebalanceRule::Schedule { every_n_blocks: 100 };
        assert!(!r.should_rebalance(&ctx(0, 99, 0)));
        assert!(r.should_rebalance(&ctx(0, 100, 0)));
        assert!(r.should_rebalance(&ctx(0, 200, 0)));
    }

    #[test]
    fn schedule_zero_never_fires() {
        let r = RebalanceRule::Schedule { every_n_blocks: 0 };
        assert!(!r.should_rebalance(&ctx(0, 1_000_000, 0)));
    }

    #[test]
    fn price_threshold_central_50_fires_when_price_in_outer_25() {
        let r = RebalanceRule::PriceExitThreshold { central_pct: 0.5 };
        // Range [-100, 100], center=0, central 50% = [-50, 50], outer 25%
        // is [-100, -50] ∪ [50, 100]. Price at tick 60 is in outer 25%.
        assert!(r.should_rebalance(&ctx(60, 0, 0)));
        // Tick 25 is inside central 50% — no rebalance.
        assert!(!r.should_rebalance(&ctx(25, 0, 0)));
        // Tick -75 in outer 25% (lower side).
        assert!(r.should_rebalance(&ctx(-75, 0, 0)));
    }

    #[test]
    fn out_of_range_duration_fires_after_threshold() {
        let r = RebalanceRule::OutOfRangeDuration {
            min_oor_blocks: 10,
        };
        assert!(!r.should_rebalance(&ctx(0, 0, 9)));
        assert!(r.should_rebalance(&ctx(0, 0, 10)));
    }

    #[test]
    fn label_round_trips() {
        let r = RebalanceRule::Schedule { every_n_blocks: 100 };
        let lbl = r.label();
        let parsed: RebalanceRule = serde_json::from_str(&lbl).unwrap();
        assert!(matches!(parsed, RebalanceRule::Schedule { every_n_blocks: 100 }));
    }
}
