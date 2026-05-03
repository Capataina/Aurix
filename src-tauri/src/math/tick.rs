//! Tick ↔ sqrtPriceX96 conversion.
//!
//! Direct port of `getSqrtRatioAtTick` from Uniswap's `TickMath.sol`. The
//! 20 magic constants are transcribed exactly from the Solidity source —
//! they are pre-computed Q128.128 approximations of `1.0001^(2^k)` for
//! k = 0..19 (the `MAX_TICK` < 2^20 bound makes higher bits unnecessary).
//! Regenerating them via floating-point would introduce drift; do not.
//!
//! Reference: `context/references/v3-mathematics-deep-dive.md` §2.3 and
//! `TickMath.sol` quoted passage P-TM-4.

use num_bigint::BigUint;

use super::error::V3MathError;
use super::q96::{
    MAX_SQRT_RATIO, MAX_TICK, MIN_SQRT_RATIO, MIN_TICK, Q128, U256_MAX,
};

/// Magic constants, indexed by bit position (0..=19). Bit `k` corresponds
/// to the `1.0001^(-2^k)` factor in Q128.128, rounded up.
const MAGIC: [&str; 20] = [
    "0xfffcb933bd6fad37aa2d162d1a594001",
    "0xfff97272373d413259a46990580e213a",
    "0xfff2e50f5f656932ef12357cf3c7fdcc",
    "0xffe5caca7e10e4e61c3624eaa0941cd0",
    "0xffcb9843d60f6159c9db58835c926644",
    "0xff973b41fa98c081472e6896dfb254c0",
    "0xff2ea16466c96a3843ec78b326b52861",
    "0xfe5dee046a99a2a811c461f1969c3053",
    "0xfcbe86c7900a88aedcffc83b479aa3a4",
    "0xf987a7253ac413176f2b074cf7815e54",
    "0xf3392b0822b70005940c7a398e4b70f3",
    "0xe7159475a2c29b7443b29c7fa6e889d9",
    "0xd097f3bdfd2022b8845ad8f792aa5825",
    "0xa9f746462d870fdf8a65dc1f90e061e5",
    "0x70d869a156d2a1b890bb3df62baf32f7",
    "0x31be135f97d08fd981231505542fcfa6",
    "0x9aa508b5b7a84e1c677de54f3e99bc9",
    "0x5d6af8dedb81196699c329225ee604",
    "0x2216e584f5fa1ea926041bedfe98",
    "0x48a170391f7dc42444e8fa2",
];

fn magic(bit: usize) -> BigUint {
    let lit = MAGIC[bit];
    let stripped = lit.trim_start_matches("0x");
    BigUint::parse_bytes(stripped.as_bytes(), 16)
        .expect("magic constant literal is valid hex (transcribed from TickMath.sol)")
}

/// Computes `sqrtPriceX96 = floor(sqrt(1.0001^tick) · 2^96)` for any tick
/// in `[MIN_TICK, MAX_TICK]`. Bit-exact match for `getSqrtRatioAtTick`
/// from `TickMath.sol`.
///
/// Inputs: `tick` — signed 24-bit value in the protocol; `i32` here.
/// Outputs: the resulting `sqrtPriceX96` as a `BigUint` (always fits in
/// uint160).
/// Errors: returns `TickOutOfBounds` for `|tick| > MAX_TICK`.
/// Side effects: none.
pub fn tick_to_sqrt_price_x96(tick: i32) -> Result<BigUint, V3MathError> {
    let abs_tick = tick.unsigned_abs();
    if abs_tick > MAX_TICK as u32 {
        return Err(V3MathError::TickOutOfBounds(tick));
    }

    // Start ratio in Q128.128. If bit 0 set, start at 1.0001^(-1); else 1.
    let mut ratio: BigUint = if abs_tick & 0x1 != 0 {
        magic(0)
    } else {
        Q128.clone()
    };

    // For each remaining bit k of abs_tick, multiply ratio by 1.0001^(-2^k)
    // in Q128.128 and shift right 128 to renormalise.
    for k in 1..20usize {
        if abs_tick & (1u32 << k) != 0 {
            ratio = (&ratio * magic(k)) >> 128;
        }
    }

    // For positive ticks, take the reciprocal: ratio * inverse_ratio = 2^256 - 1.
    if tick > 0 {
        ratio = &*U256_MAX / ratio;
    }

    // Shift from Q128.128 down to Q64.96, rounding up the dropped bits.
    let dropped: BigUint = &ratio % (BigUint::from(1u8) << 32);
    let mut sqrt_price_x96 = ratio >> 32;
    if !dropped.bits().eq(&0) {
        // any non-zero dropped bits → round up
        if dropped > BigUint::from(0u8) {
            sqrt_price_x96 += 1u8;
        }
    }

    Ok(sqrt_price_x96)
}

/// Computes the greatest tick `t` such that `tick_to_sqrt_price_x96(t)` is
/// less than or equal to `sqrt_price_x96`. Inverse of
/// `tick_to_sqrt_price_x96`, satisfying the V3 invariant
/// `sqrt(t) <= sqrtPriceX96 < sqrt(t+1)`.
///
/// Implementation: f64 log-estimate then iterative refinement using the
/// exact `tick_to_sqrt_price_x96` to land on the unique `t` satisfying the
/// invariant. The refinement covers the off-by-one cases that hit when
/// the f64 log gives a tick on a boundary.
///
/// Inputs: a `sqrtPriceX96` in `[MIN_SQRT_RATIO, MAX_SQRT_RATIO]`.
/// Outputs: the integer tick.
/// Errors: returns `SqrtRatioOutOfBounds` when `sqrt_price_x96` is below
/// `MIN_SQRT_RATIO` or at/above `MAX_SQRT_RATIO`.
/// Side effects: none.
pub fn sqrt_price_x96_to_tick(sqrt_price_x96: &BigUint) -> Result<i32, V3MathError> {
    if sqrt_price_x96 < &*MIN_SQRT_RATIO || sqrt_price_x96 >= &*MAX_SQRT_RATIO {
        return Err(V3MathError::SqrtRatioOutOfBounds);
    }

    // ratio = sqrtPriceX96 / 2^96 (real number); tick = log_{1.0001}(ratio^2).
    // Use the bit length to keep the f64 conversion tractable for large
    // sqrt values. Equivalent: log_{1.0001}(sqrtPriceX96^2 / 2^192)
    // = 2 * (log2(sqrtPriceX96) - 96) / log2(1.0001).
    let log2_sqrt = bits_log2(sqrt_price_x96);
    // ln(1.0001) / ln(2) = log2(1.0001)
    let log2_1_0001 = 1.0001f64.log2();
    let estimate = (2.0 * (log2_sqrt - 96.0) / log2_1_0001).floor() as i32;

    // Refine by walking ±2 ticks around the estimate to find the unique t
    // satisfying tick_to_sqrt_price_x96(t) <= sqrtPriceX96
    //         < tick_to_sqrt_price_x96(t+1).
    let lo_search = (estimate - 2).max(MIN_TICK);
    let hi_search = (estimate + 2).min(MAX_TICK);
    let mut best = lo_search;
    for t in lo_search..=hi_search {
        let s = tick_to_sqrt_price_x96(t)?;
        if &s <= sqrt_price_x96 {
            best = t;
        } else {
            break;
        }
    }
    Ok(best)
}

/// Approximate log2 of a BigUint via bit length plus the high-mantissa
/// f64 reconstruction. Adequate for the log-estimate seed in
/// `sqrt_price_x96_to_tick`; the iterative refinement above corrects for
/// f64 rounding error.
fn bits_log2(value: &BigUint) -> f64 {
    let bits = value.bits();
    if bits <= 53 {
        // Tiny — convert directly.
        let v_f64: f64 = value
            .to_u64_digits()
            .first()
            .copied()
            .unwrap_or(0) as f64;
        return v_f64.log2();
    }
    // For large values: take the top 53 bits as a mantissa, log2 it, then
    // add (total_bits - 53) for the implicit shift.
    let shift = bits as i64 - 53;
    let high = value >> (shift as u64);
    let high_f64 = high.to_u64_digits().first().copied().unwrap_or(0) as f64;
    high_f64.log2() + shift as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_zero_is_q96() {
        // tick = 0 → sqrtPriceX96 = 1 * 2^96 = 79228162514264337593543950336.
        let s = tick_to_sqrt_price_x96(0).unwrap();
        let expected = BigUint::parse_bytes(b"79228162514264337593543950336", 10).unwrap();
        assert_eq!(s, expected);
    }

    #[test]
    fn tick_min_matches_min_sqrt_ratio() {
        // MIN_TICK = -887272 → sqrtPriceX96 = MIN_SQRT_RATIO = 4295128739.
        let s = tick_to_sqrt_price_x96(MIN_TICK).unwrap();
        assert_eq!(s, *MIN_SQRT_RATIO);
    }

    #[test]
    fn tick_max_matches_max_sqrt_ratio() {
        // MAX_TICK = 887272 → sqrtPriceX96 = MAX_SQRT_RATIO.
        let s = tick_to_sqrt_price_x96(MAX_TICK).unwrap();
        assert_eq!(s, *MAX_SQRT_RATIO);
    }

    #[test]
    fn tick_one_increases_above_q96() {
        // sqrt(1.0001^1) > 1 → sqrtPriceX96 must exceed Q96 = 2^96.
        let s = tick_to_sqrt_price_x96(1).unwrap();
        let q96 = BigUint::parse_bytes(b"79228162514264337593543950336", 10).unwrap();
        assert!(s > q96);
        // The per-tick factor is 0.5 bp on sqrt → roughly 4e24 absolute
        // increase at this scale; check magnitude bracket.
        let diff = &s - &q96;
        let lo = BigUint::parse_bytes(b"3000000000000000000000000", 10).unwrap();
        let hi = BigUint::parse_bytes(b"5000000000000000000000000", 10).unwrap();
        assert!(diff > lo && diff < hi, "tick=1 step magnitude wrong: diff={diff}");
    }

    #[test]
    fn tick_minus_one_decreases_below_q96() {
        let s = tick_to_sqrt_price_x96(-1).unwrap();
        let q96 = BigUint::parse_bytes(b"79228162514264337593543950336", 10).unwrap();
        assert!(s < q96);
    }

    #[test]
    fn out_of_bounds_returns_err() {
        let r = tick_to_sqrt_price_x96(MAX_TICK + 1);
        assert!(matches!(r, Err(V3MathError::TickOutOfBounds(_))));
        let r = tick_to_sqrt_price_x96(MIN_TICK - 1);
        assert!(matches!(r, Err(V3MathError::TickOutOfBounds(_))));
    }

    #[test]
    fn round_trip_at_zero() {
        let s = tick_to_sqrt_price_x96(0).unwrap();
        let t = sqrt_price_x96_to_tick(&s).unwrap();
        assert_eq!(t, 0);
    }

    #[test]
    fn round_trip_at_min_returns_min_tick() {
        let t = sqrt_price_x96_to_tick(&MIN_SQRT_RATIO).unwrap();
        assert_eq!(t, MIN_TICK);
    }

    #[test]
    fn round_trip_at_various_ticks() {
        // Sample several ticks; round-trip via to_sqrt → to_tick must match.
        for &t in &[
            -200_000, -100_000, -50_000, -10_000, -1_000, -100, -1, 0, 1, 100, 1_000, 10_000,
            50_000, 100_000, 200_000,
        ] {
            let s = tick_to_sqrt_price_x96(t).unwrap();
            let recovered = sqrt_price_x96_to_tick(&s).unwrap();
            assert_eq!(
                recovered, t,
                "round-trip failed at tick={t}: sqrt={s}, recovered={recovered}"
            );
        }
    }

    #[test]
    fn invariant_sqrt_at_tick_le_sqrt_at_next() {
        // For every tick t, sqrt(t) <= sqrt(t+1) and the recovered tick
        // for any value in [sqrt(t), sqrt(t+1)) is t.
        for &t in &[-10_000, -100, 0, 100, 10_000] {
            let s_t = tick_to_sqrt_price_x96(t).unwrap();
            let s_next = tick_to_sqrt_price_x96(t + 1).unwrap();
            assert!(s_t < s_next, "sqrt should increase with tick");
            // A point one less than s_next must round-trip to t.
            let mid = &s_next - 1u8;
            assert_eq!(sqrt_price_x96_to_tick(&mid).unwrap(), t);
        }
    }

    #[test]
    fn sqrt_below_min_errors() {
        let s = &*MIN_SQRT_RATIO - 1u8;
        let r = sqrt_price_x96_to_tick(&s);
        assert!(matches!(r, Err(V3MathError::SqrtRatioOutOfBounds)));
    }

    #[test]
    fn sqrt_at_max_errors() {
        // The protocol's invariant excludes MAX_SQRT_RATIO itself.
        let r = sqrt_price_x96_to_tick(&*MAX_SQRT_RATIO);
        assert!(matches!(r, Err(V3MathError::SqrtRatioOutOfBounds)));
    }
}
