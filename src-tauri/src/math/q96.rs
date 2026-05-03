//! Q64.96 fixed-point primitives.
//!
//! Aurix's V3 math stack rolls its own Q64.96 from `num_bigint::BigUint`
//! (per the resume bullet "no ethers-rs"). The functions here are the
//! shared building blocks for tick / liquidity / fee math.
//!
//! Reference: `context/references/v3-mathematics-deep-dive.md` §1.

use num_bigint::BigUint;
use num_traits::Zero;
use once_cell::sync::Lazy;

use super::error::V3MathError;

/// 2^96. The Q64.96 normalisation factor.
pub static Q96: Lazy<BigUint> = Lazy::new(|| BigUint::from(1u8) << 96);

/// 2^128. The Q64.128 / Q128.128 intermediate normalisation factor used by
/// `tick_to_sqrt_price_x96`'s magic-constant algorithm.
pub static Q128: Lazy<BigUint> = Lazy::new(|| BigUint::from(1u8) << 128);

/// 2^160. The uint160 ceiling — `sqrtPriceX96` must fit in this width.
pub static Q160: Lazy<BigUint> = Lazy::new(|| BigUint::from(1u8) << 160);

/// 2^192. Used in `sqrtPriceX96^2 / 2^192` price decoding.
pub static Q192: Lazy<BigUint> = Lazy::new(|| BigUint::from(1u8) << 192);

/// 2^256 - 1. Used in the `ratio = type(uint256).max / ratio` reciprocal
/// step of `getSqrtRatioAtTick` for positive ticks.
pub static U256_MAX: Lazy<BigUint> = Lazy::new(|| (BigUint::from(1u8) << 256) - 1u8);

/// `MIN_SQRT_RATIO` from `TickMath.sol` — `4295128739`. Lower bound of the
/// representable sqrtPriceX96 range; exclusive in the swap path.
pub static MIN_SQRT_RATIO: Lazy<BigUint> = Lazy::new(|| BigUint::from(4_295_128_739u64));

/// `MAX_SQRT_RATIO` from `TickMath.sol`.
pub static MAX_SQRT_RATIO: Lazy<BigUint> = Lazy::new(|| {
    BigUint::parse_bytes(b"1461446703485210103287273052203988822378723970342", 10)
        .expect("MAX_SQRT_RATIO literal is well-formed decimal")
});

/// `MIN_TICK` per `TickMath.sol`. The most-negative tick for which
/// `getSqrtRatioAtTick` is defined.
pub const MIN_TICK: i32 = -887_272;
/// `MAX_TICK` per `TickMath.sol`.
pub const MAX_TICK: i32 = 887_272;

/// Multiply-then-divide with full BigUint precision — never narrow before
/// dividing. Direct port of Solidity's `FullMath.mulDiv`.
///
/// Inputs: numerator factors `a`, `b` and denominator `denom`.
/// Outputs: `floor(a * b / denom)` as a BigUint.
/// Errors: returns `DivisionByZero` when `denom` is zero.
/// Side effects: none.
pub fn mul_div(a: &BigUint, b: &BigUint, denom: &BigUint) -> Result<BigUint, V3MathError> {
    if denom.is_zero() {
        return Err(V3MathError::DivisionByZero);
    }
    Ok((a * b) / denom)
}

/// Multiply-then-divide with rounding-up. Mirrors the `roundUp = true`
/// branch in `SqrtPriceMath` — when computing input amounts (token in to
/// the pool) we round up so the LP gets at least the modelled amount.
pub fn mul_div_round_up(
    a: &BigUint,
    b: &BigUint,
    denom: &BigUint,
) -> Result<BigUint, V3MathError> {
    if denom.is_zero() {
        return Err(V3MathError::DivisionByZero);
    }
    let prod = a * b;
    let quotient = &prod / denom;
    let remainder = &prod % denom;
    if remainder.is_zero() {
        Ok(quotient)
    } else {
        Ok(quotient + 1u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q96_constant_value() {
        let expected = BigUint::parse_bytes(b"79228162514264337593543950336", 10).unwrap();
        assert_eq!(*Q96, expected, "Q96 must equal 2^96 = 79228162514264337593543950336");
    }

    #[test]
    fn min_max_sqrt_ratio_match_tickmath_sol() {
        assert_eq!(*MIN_SQRT_RATIO, BigUint::from(4_295_128_739u64));
        let max_str = format!("{}", *MAX_SQRT_RATIO);
        assert_eq!(
            max_str,
            "1461446703485210103287273052203988822378723970342"
        );
    }

    #[test]
    fn mul_div_basic_arithmetic() {
        let r = mul_div(
            &BigUint::from(10u64),
            &BigUint::from(20u64),
            &BigUint::from(4u64),
        )
        .unwrap();
        assert_eq!(r, BigUint::from(50u64));
    }

    #[test]
    fn mul_div_truncates_toward_zero() {
        // 7 * 7 / 5 = 49 / 5 = 9 (floor)
        let r = mul_div(
            &BigUint::from(7u64),
            &BigUint::from(7u64),
            &BigUint::from(5u64),
        )
        .unwrap();
        assert_eq!(r, BigUint::from(9u64));
    }

    #[test]
    fn mul_div_round_up_rounds_when_remainder() {
        let r = mul_div_round_up(
            &BigUint::from(7u64),
            &BigUint::from(7u64),
            &BigUint::from(5u64),
        )
        .unwrap();
        assert_eq!(r, BigUint::from(10u64), "49/5 rounds up to 10");
    }

    #[test]
    fn mul_div_round_up_does_not_round_when_exact() {
        let r = mul_div_round_up(
            &BigUint::from(10u64),
            &BigUint::from(20u64),
            &BigUint::from(4u64),
        )
        .unwrap();
        assert_eq!(r, BigUint::from(50u64));
    }

    #[test]
    fn mul_div_zero_denom_errors() {
        let r = mul_div(
            &BigUint::from(1u64),
            &BigUint::from(1u64),
            &BigUint::from(0u64),
        );
        assert!(matches!(r, Err(V3MathError::DivisionByZero)));
    }
}
