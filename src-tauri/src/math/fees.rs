//! Per-swap fee accounting.
//!
//! V3 distributes swap fees to LPs proportionally to their share of the
//! active in-range liquidity at the moment of the swap. Per
//! `context/references/v3-mathematics-deep-dive.md` §4 + §5, the
//! per-swap distribution is what differentiates an exact backtester from
//! one that aggregates at the block level.
//!
//! Aurix computes per-swap fees with the simple share rule:
//!
//! ```text
//!   fee_amount_for_position
//!     = swap_fee * (position_liquidity / active_liquidity_at_swap)
//! ```
//!
//! where `swap_fee = swap_amount * fee_tier_bps / 10_000`.
//!
//! The active liquidity at swap time is the pool's `liquidity` field at
//! the start of the swap. We compute the LP's share against that value
//! when the position is in range; otherwise the LP earns nothing.

use num_bigint::{BigInt, BigUint};
use num_traits::Zero;

use super::error::V3MathError;

/// Returns the position's fee share of a single swap, in token0 raw units.
///
/// `swap_amount0_in_abs` is the absolute value of the swap's `amount0`
/// when token0 is being paid in (when `amount0 > 0` per V3 sign
/// conventions). Otherwise pass zero — the position only earns the
/// token0-fee on token0-in legs.
pub fn fee_share_token0(
    swap_amount0_in_abs: &BigUint,
    fee_tier_bps: u32,
    position_liquidity: u128,
    active_liquidity: u128,
    in_range: bool,
) -> Result<BigUint, V3MathError> {
    fee_share(
        swap_amount0_in_abs,
        fee_tier_bps,
        position_liquidity,
        active_liquidity,
        in_range,
    )
}

/// Mirror for token1.
pub fn fee_share_token1(
    swap_amount1_in_abs: &BigUint,
    fee_tier_bps: u32,
    position_liquidity: u128,
    active_liquidity: u128,
    in_range: bool,
) -> Result<BigUint, V3MathError> {
    fee_share(
        swap_amount1_in_abs,
        fee_tier_bps,
        position_liquidity,
        active_liquidity,
        in_range,
    )
}

fn fee_share(
    swap_amount: &BigUint,
    fee_tier_bps: u32,
    position_liquidity: u128,
    active_liquidity: u128,
    in_range: bool,
) -> Result<BigUint, V3MathError> {
    if !in_range || active_liquidity == 0 {
        return Ok(BigUint::from(0u8));
    }
    let total_fee_token = swap_amount * BigUint::from(fee_tier_bps) / BigUint::from(1_000_000u32);
    // Note 1_000_000 because Solidity scales fee_tier as `feeAmount * fee /
    // 1e6` where fee tiers are stored in "hundredths-of-bps" — i.e. 5bps
    // is stored as 500. We expect callers to pass fee_tier_bps in the
    // "hundredths of bps" convention (500 for 5bps, 3000 for 30bps).
    let pos_l = BigUint::from(position_liquidity);
    let active_l = BigUint::from(active_liquidity);
    if active_l.is_zero() {
        return Ok(BigUint::from(0u8));
    }
    Ok(total_fee_token * pos_l / active_l)
}

/// For a swap at signed `amount0` / `amount1` per V3 sign convention,
/// returns `(amount0_in_abs, amount1_in_abs)` — exactly one of the pair
/// is non-zero in a normal swap. Used as the fee-leg input to
/// `fee_share_*`.
pub fn extract_in_amounts(amount0: &BigInt, amount1: &BigInt) -> (BigUint, BigUint) {
    let z = BigInt::from(0u8);
    let a0_in = if amount0 > &z {
        amount0.to_biguint().unwrap_or_default()
    } else {
        BigUint::from(0u8)
    };
    let a1_in = if amount1 > &z {
        amount1.to_biguint().unwrap_or_default()
    } else {
        BigUint::from(0u8)
    };
    (a0_in, a1_in)
}

/// Convenience: convert a fee-tier in basis points (5, 30, 100, 10000)
/// into the protocol's "hundredths-of-bps" unit (500, 3000, 10000,
/// 1000000). Aurix stores bps in the friendlier unit and converts here
/// before fee math.
pub fn bps_to_protocol_units(fee_tier_bps: u32) -> u32 {
    fee_tier_bps * 100
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;

    #[test]
    fn out_of_range_position_earns_no_fees() {
        let r = fee_share_token0(
            &BigUint::from(1_000_000u64),
            500,
            1_000u128,
            1_000_000u128,
            false,
        )
        .unwrap();
        assert_eq!(r, BigUint::from(0u8));
    }

    #[test]
    fn share_is_proportional_to_liquidity_ratio() {
        // 5bps fee = 500 protocol units. swap_amount = 1000.
        // total_fee = 1000 * 500 / 1_000_000 = 0.5 → 0 with integer div.
        // Use larger swap so fees are non-trivial.
        let swap = BigUint::from(1_000_000_000u64);
        let total_fee = &swap * BigUint::from(500u32) / BigUint::from(1_000_000u32);
        // = 500_000.

        // Position is 10% of active liquidity.
        let r = fee_share_token0(&swap, 500, 100u128, 1_000u128, true).unwrap();
        assert_eq!(r, &total_fee * 100u32 / 1_000u32);
        assert_eq!(r, BigUint::from(50_000u32));
    }

    #[test]
    fn extract_in_amounts_pulls_only_positive_legs() {
        // amount0 < 0 (out), amount1 > 0 (in) — typical swap shape.
        let a0 = BigInt::parse_bytes(b"-1000000000000000000", 10).unwrap();
        let a1 = BigInt::parse_bytes(b"3000000000", 10).unwrap();
        let (a0_in, a1_in) = extract_in_amounts(&a0, &a1);
        assert_eq!(a0_in, BigUint::from(0u8));
        assert_eq!(a1_in, BigUint::from(3_000_000_000u64));
    }

    #[test]
    fn bps_to_protocol_units_is_x100() {
        assert_eq!(bps_to_protocol_units(5), 500);
        assert_eq!(bps_to_protocol_units(30), 3000);
        assert_eq!(bps_to_protocol_units(100), 10_000);
        assert_eq!(bps_to_protocol_units(10_000), 1_000_000);
    }
}
