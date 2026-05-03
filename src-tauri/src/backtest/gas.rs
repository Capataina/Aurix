//! Management gas costing.
//!
//! Per `vector-a-v3-lp-backtester.md` §M2.3, mgmt gas costs are deducted
//! at the block they occurred, priced at that block's median gas price.
//!
//! Standard cost estimates:
//!
//! | Operation | Gas units | Notes                            |
//! |-----------|-----------|----------------------------------|
//! | Mint      | 350_000   | Plan correction per Gamma 2021    |
//! | Burn      | 150_000   |                                  |
//! | Collect   | 120_000   |                                  |
//! | Rebalance | 500_000   | mint + burn + small overhead     |
//!
//! Conversion to USD: `gas_units * gas_price_gwei * 1e-9 * eth_usd`.
//! When `gas_price_gwei` is missing, falls back to a config default.

#[derive(Debug, Clone, Copy)]
pub enum MgmtGasOp {
    Mint,
    Burn,
    Collect,
    Rebalance,
}

impl MgmtGasOp {
    pub fn gas_units(&self) -> u64 {
        match self {
            Self::Mint => 350_000,
            Self::Burn => 150_000,
            Self::Collect => 120_000,
            Self::Rebalance => 500_000,
        }
    }
}

/// USD cost of `op` at `gas_price_gwei` and `eth_usd_price`.
pub fn cost_usd(op: MgmtGasOp, gas_price_gwei: f64, eth_usd_price: f64) -> f64 {
    let gas_units = op.gas_units() as f64;
    gas_units * gas_price_gwei * 1e-9 * eth_usd_price
}

/// Fee-haircut applied to a swap leg from MEV (sandwich tax). `bps` is
/// the per-leg basis-point haircut (e.g. 5 ⇒ 0.05% loss). Returns the
/// USD loss for a swap of `notional_usd`.
pub fn mev_haircut_usd(notional_usd: f64, bps: f64) -> f64 {
    notional_usd * bps / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mint_at_20_gwei_3000_eth_costs_about_21() {
        // 350_000 * 20 * 1e-9 * 3000 = 21.0 USD
        let c = cost_usd(MgmtGasOp::Mint, 20.0, 3000.0);
        assert!((c - 21.0).abs() < 1e-6);
    }

    #[test]
    fn rebalance_dwarfs_collect_at_same_gas_price() {
        let r = cost_usd(MgmtGasOp::Rebalance, 20.0, 3000.0);
        let c = cost_usd(MgmtGasOp::Collect, 20.0, 3000.0);
        assert!(r > 4.0 * c);
    }

    #[test]
    fn mev_haircut_5bps_on_10k_is_5usd() {
        assert!((mev_haircut_usd(10_000.0, 5.0) - 5.0).abs() < 1e-9);
    }
}
