//! Liquidity ↔ amounts conversion.
//!
//! Implements `getLiquidityForAmounts`, `getAmountsForLiquidity`, and the
//! per-step `getAmount0Delta` / `getAmount1Delta` primitives from
//! `LiquidityAmounts.sol` and `SqrtPriceMath.sol`.
//!
//! Reference: `context/references/v3-mathematics-deep-dive.md` §3.

use num_bigint::BigUint;

use super::error::V3MathError;
use super::q96::{mul_div, mul_div_round_up, Q96};

/// Computes the liquidity for an unbalanced position.
///
/// Given `sqrt_lower < sqrt_upper` (range bounds) and a current
/// `sqrt_current`, plus available token0/token1 amounts, returns the
/// maximum constructible liquidity.
///
/// Three cases (V3 §getLiquidityForAmounts):
/// 1. `sqrt_current <= sqrt_lower` — entirely token0; use amount0 only.
/// 2. `sqrt_current >= sqrt_upper` — entirely token1; use amount1 only.
/// 3. otherwise — use min(L_from_amount0(current..upper), L_from_amount1(lower..current)).
///
/// Inputs: sqrtPriceX96 bounds + amounts in raw token units.
/// Outputs: liquidity as `u128` (the protocol's chosen type).
/// Errors: liquidity overflow if the result exceeds `u128::MAX`.
/// Side effects: none.
pub fn liquidity_for_amounts(
    sqrt_current: &BigUint,
    sqrt_lower: &BigUint,
    sqrt_upper: &BigUint,
    amount0: &BigUint,
    amount1: &BigUint,
) -> Result<u128, V3MathError> {
    let (lo, hi) = if sqrt_lower < sqrt_upper {
        (sqrt_lower, sqrt_upper)
    } else {
        (sqrt_upper, sqrt_lower)
    };

    let result = if sqrt_current <= lo {
        liquidity_from_amount0(lo, hi, amount0)?
    } else if sqrt_current >= hi {
        liquidity_from_amount1(lo, hi, amount1)?
    } else {
        let l0 = liquidity_from_amount0(sqrt_current, hi, amount0)?;
        let l1 = liquidity_from_amount1(lo, sqrt_current, amount1)?;
        l0.min(l1)
    };

    Ok(result)
}

/// `liquidity = amount0 * (sqrt_a * sqrt_b / Q96) / (sqrt_b - sqrt_a)`
fn liquidity_from_amount0(
    sqrt_a: &BigUint,
    sqrt_b: &BigUint,
    amount0: &BigUint,
) -> Result<u128, V3MathError> {
    if sqrt_a >= sqrt_b {
        return Ok(0);
    }
    let intermediate = mul_div(sqrt_a, sqrt_b, &Q96)?;
    let l = mul_div(amount0, &intermediate, &(sqrt_b - sqrt_a))?;
    bigint_to_u128(&l)
}

/// `liquidity = amount1 * Q96 / (sqrt_b - sqrt_a)`
fn liquidity_from_amount1(
    sqrt_a: &BigUint,
    sqrt_b: &BigUint,
    amount1: &BigUint,
) -> Result<u128, V3MathError> {
    if sqrt_a >= sqrt_b {
        return Ok(0);
    }
    let l = mul_div(amount1, &Q96, &(sqrt_b - sqrt_a))?;
    bigint_to_u128(&l)
}

/// Computes the token0/token1 amounts a position with `liquidity` over
/// `[sqrt_lower, sqrt_upper]` represents at price `sqrt_current`.
///
/// Three cases (mirror of `liquidity_for_amounts`):
/// 1. current <= lower → all token0.
/// 2. current >= upper → all token1.
/// 3. otherwise → both, split at current.
pub fn amounts_for_liquidity(
    sqrt_current: &BigUint,
    sqrt_lower: &BigUint,
    sqrt_upper: &BigUint,
    liquidity: u128,
) -> Result<(BigUint, BigUint), V3MathError> {
    let (lo, hi) = if sqrt_lower < sqrt_upper {
        (sqrt_lower, sqrt_upper)
    } else {
        (sqrt_upper, sqrt_lower)
    };
    let liq_big = BigUint::from(liquidity);

    let (a0, a1) = if sqrt_current <= lo {
        (amount0_delta(lo, hi, &liq_big, false)?, BigUint::from(0u8))
    } else if sqrt_current >= hi {
        (BigUint::from(0u8), amount1_delta(lo, hi, &liq_big, false)?)
    } else {
        let a0 = amount0_delta(sqrt_current, hi, &liq_big, false)?;
        let a1 = amount1_delta(lo, sqrt_current, &liq_big, false)?;
        (a0, a1)
    };
    Ok((a0, a1))
}

/// Per-step `getAmount0Delta`. Token0 is X = liquidity * (sqrt_b - sqrt_a)
/// / (sqrt_a * sqrt_b) — derived in V3 whitepaper §6.5. The Q96 cancels:
///
/// ```text
///   amount0 = (L * 2^96 * (sqrt_b - sqrt_a)) / (sqrt_b * sqrt_a)
/// ```
///
/// `round_up` controls SqrtPriceMath's per-direction rounding.
pub fn amount0_delta(
    sqrt_a: &BigUint,
    sqrt_b: &BigUint,
    liquidity: &BigUint,
    round_up: bool,
) -> Result<BigUint, V3MathError> {
    if sqrt_a >= sqrt_b {
        return Ok(BigUint::from(0u8));
    }
    let numerator1 = liquidity << 96;
    let numerator2 = sqrt_b - sqrt_a;
    if round_up {
        let intermediate = mul_div_round_up(&numerator1, &numerator2, sqrt_b)?;
        // (a / sqrt_a) round-up
        let q = &intermediate / sqrt_a;
        let r = &intermediate % sqrt_a;
        if r.bits() != 0 {
            Ok(q + 1u8)
        } else {
            Ok(q)
        }
    } else {
        let intermediate = mul_div(&numerator1, &numerator2, sqrt_b)?;
        Ok(intermediate / sqrt_a)
    }
}

/// Per-step `getAmount1Delta`. Token1 is Y = liquidity * (sqrt_b - sqrt_a)
/// — straight from the AMM constant.
///
/// ```text
///   amount1 = L * (sqrt_b - sqrt_a) / 2^96
/// ```
pub fn amount1_delta(
    sqrt_a: &BigUint,
    sqrt_b: &BigUint,
    liquidity: &BigUint,
    round_up: bool,
) -> Result<BigUint, V3MathError> {
    if sqrt_a >= sqrt_b {
        return Ok(BigUint::from(0u8));
    }
    let diff = sqrt_b - sqrt_a;
    if round_up {
        mul_div_round_up(liquidity, &diff, &Q96)
    } else {
        mul_div(liquidity, &diff, &Q96)
    }
}

fn bigint_to_u128(v: &BigUint) -> Result<u128, V3MathError> {
    let digits = v.to_u64_digits();
    match digits.len() {
        0 => Ok(0u128),
        1 => Ok(digits[0] as u128),
        2 => Ok((digits[1] as u128) << 64 | (digits[0] as u128)),
        _ => Err(V3MathError::LiquidityOverflow),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::tick::tick_to_sqrt_price_x96;

    #[test]
    fn round_trip_in_range_position() {
        // Reference scenario: tick range [-100, 100] around tick 0, supply
        // 1 token0 (1e18) and 3000 token1 (3e9 with 6 decimals = $3000),
        // current price at tick 0.
        let sqrt_lower = tick_to_sqrt_price_x96(-100).unwrap();
        let sqrt_upper = tick_to_sqrt_price_x96(100).unwrap();
        let sqrt_current = tick_to_sqrt_price_x96(0).unwrap();
        let amount0 = BigUint::parse_bytes(b"1000000000000000000", 10).unwrap();
        let amount1 = BigUint::parse_bytes(b"3000000000", 10).unwrap();

        let l = liquidity_for_amounts(
            &sqrt_current,
            &sqrt_lower,
            &sqrt_upper,
            &amount0,
            &amount1,
        )
        .unwrap();
        assert!(l > 0);

        // Recompute amounts from L and verify ≤ inputs (rounding-down)
        let (a0, a1) = amounts_for_liquidity(&sqrt_current, &sqrt_lower, &sqrt_upper, l).unwrap();
        assert!(a0 <= amount0);
        assert!(a1 <= amount1);
    }

    #[test]
    fn current_below_range_uses_only_amount0() {
        let sqrt_lower = tick_to_sqrt_price_x96(100).unwrap();
        let sqrt_upper = tick_to_sqrt_price_x96(200).unwrap();
        let sqrt_current = tick_to_sqrt_price_x96(50).unwrap();
        let amount0 = BigUint::parse_bytes(b"1000000000000000000", 10).unwrap();
        let amount1 = BigUint::parse_bytes(b"3000000000", 10).unwrap();

        let l = liquidity_for_amounts(
            &sqrt_current,
            &sqrt_lower,
            &sqrt_upper,
            &amount0,
            &amount1,
        )
        .unwrap();

        let (a0, a1) = amounts_for_liquidity(&sqrt_current, &sqrt_lower, &sqrt_upper, l).unwrap();
        assert!(a0 > BigUint::from(0u8));
        assert_eq!(a1, BigUint::from(0u8), "below range — no token1");
    }

    #[test]
    fn current_above_range_uses_only_amount1() {
        let sqrt_lower = tick_to_sqrt_price_x96(-200).unwrap();
        let sqrt_upper = tick_to_sqrt_price_x96(-100).unwrap();
        let sqrt_current = tick_to_sqrt_price_x96(0).unwrap();
        let amount0 = BigUint::parse_bytes(b"1000000000000000000", 10).unwrap();
        let amount1 = BigUint::parse_bytes(b"3000000000", 10).unwrap();

        let l = liquidity_for_amounts(
            &sqrt_current,
            &sqrt_lower,
            &sqrt_upper,
            &amount0,
            &amount1,
        )
        .unwrap();

        let (a0, a1) = amounts_for_liquidity(&sqrt_current, &sqrt_lower, &sqrt_upper, l).unwrap();
        assert_eq!(a0, BigUint::from(0u8), "above range — no token0");
        assert!(a1 > BigUint::from(0u8));
    }

    #[test]
    fn amount_deltas_sum_to_position_value() {
        // L from a known position should reproduce the input amounts via
        // amount0_delta + amount1_delta when current is inside the range.
        let sqrt_lower = tick_to_sqrt_price_x96(-100).unwrap();
        let sqrt_upper = tick_to_sqrt_price_x96(100).unwrap();
        let sqrt_current = tick_to_sqrt_price_x96(0).unwrap();

        let liquidity = 1_000_000_000_000u128;
        let (a0, a1) =
            amounts_for_liquidity(&sqrt_current, &sqrt_lower, &sqrt_upper, liquidity).unwrap();
        // Both > 0 because we're inside the range.
        assert!(a0 > BigUint::from(0u8));
        assert!(a1 > BigUint::from(0u8));

        // Recompute liquidity from amounts → with both directions using
        // round-down, recovered should be ≤ liquidity. The drift bound is
        // proportional to liquidity / Q96 (rounding cost per division);
        // for L=1e12 the drift is well under the relative-1ppm threshold.
        let recovered = liquidity_for_amounts(
            &sqrt_current,
            &sqrt_lower,
            &sqrt_upper,
            &a0,
            &a1,
        )
        .unwrap();
        assert!(
            recovered <= liquidity,
            "round-down round-trip must not exceed input"
        );
        let diff = liquidity - recovered;
        // Allow up to 1 ppm drift (1e6 absolute drift on L=1e12).
        let bound = liquidity / 1_000_000;
        assert!(
            diff <= bound,
            "round-trip drift {diff} exceeds ppm bound {bound}"
        );
    }

    #[test]
    fn round_up_branches_yield_at_least_round_down() {
        // Round-up amount must be ≥ round-down amount.
        let sqrt_a = tick_to_sqrt_price_x96(-100).unwrap();
        let sqrt_b = tick_to_sqrt_price_x96(100).unwrap();
        let liq = BigUint::from(1_000_000_000_000u64);

        let down0 = amount0_delta(&sqrt_a, &sqrt_b, &liq, false).unwrap();
        let up0 = amount0_delta(&sqrt_a, &sqrt_b, &liq, true).unwrap();
        assert!(up0 >= down0);

        let down1 = amount1_delta(&sqrt_a, &sqrt_b, &liq, false).unwrap();
        let up1 = amount1_delta(&sqrt_a, &sqrt_b, &liq, true).unwrap();
        assert!(up1 >= down1);
    }

    #[test]
    fn collapsed_range_returns_zero_liquidity_or_amounts() {
        // sqrt_a == sqrt_b → no width, no liquidity, no amounts.
        let s = tick_to_sqrt_price_x96(100).unwrap();
        let liq = BigUint::from(1u64);
        assert_eq!(amount0_delta(&s, &s, &liq, false).unwrap(), BigUint::from(0u8));
        assert_eq!(amount1_delta(&s, &s, &liq, false).unwrap(), BigUint::from(0u8));
    }
}
