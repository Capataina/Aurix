//! Impermanent loss closed forms.
//!
//! Two regimes:
//!
//! * **V2 / full-range V3** — IL has a clean closed form depending only on
//!   the price ratio `r = p_now / p_entry`:
//!
//!   ```text
//!     IL_v2(r) = 2*sqrt(r) / (1 + r) - 1
//!   ```
//!
//!   At r=1 → 0 (no move). At r=0.5 or r=2 → ≈ -5.7%.
//!
//! * **Concentrated V3** — IL depends on the position range
//!   `[p_lower, p_upper]` and the current price; the formula reduces to
//!   the V2 formula scaled by the range factor. We compute it
//!   numerically from the current `(amount0, amount1)` of the position
//!   versus the hold-only baseline.
//!
//! Both regimes use `f64` for the closed forms — the inputs are already
//! USD-scale and IL is a dimensionless fraction.
//!
//! Reference: `context/references/v3-mathematics-deep-dive.md` §6.

use super::error::V3MathError;

/// V2-style IL (or full-range V3 IL). Returns the impermanent-loss
/// fraction at `r = p_now / p_entry` — a value in `(-1, 0]`.
///
/// Inputs: `r` strictly positive.
/// Outputs: IL as a fraction (-0.057 = 5.7% loss).
/// Errors: `DivisionByZero` when r is zero.
/// Side effects: none.
pub fn il_v2(r: f64) -> Result<f64, V3MathError> {
    if r <= 0.0 {
        return Err(V3MathError::DivisionByZero);
    }
    Ok(2.0 * r.sqrt() / (1.0 + r) - 1.0)
}

/// Concentrated-V3 IL. Computes the position's current USD value vs the
/// hold-only USD value at the same prices, returning the IL fraction.
///
/// Used in M2.3 to populate the per-sample `il_usd` field of the equity
/// curve — the absolute USD figure is `position_value_usd -
/// hold_only_value_usd` (note: this is the realised IL contribution, not
/// the multiplier).
///
/// Inputs:
/// - `position_value_usd` — the current USD value of the position's
///   token0+token1 holdings at the current price.
/// - `hold_only_value_usd` — the USD value the entry composition would
///   have at the current price (no LP, just holding).
/// Outputs: IL fraction in `(-1, 0]`. Returns 0 when hold value is zero.
/// Errors: never.
/// Side effects: none.
pub fn il_concentrated(position_value_usd: f64, hold_only_value_usd: f64) -> f64 {
    if hold_only_value_usd <= 0.0 {
        return 0.0;
    }
    (position_value_usd - hold_only_value_usd) / hold_only_value_usd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn il_v2_zero_at_r_one() {
        let il = il_v2(1.0).unwrap();
        assert!(il.abs() < 1e-12);
    }

    #[test]
    fn il_v2_known_two_x() {
        // Reference: r=2 → 2*sqrt(2)/3 - 1 = -0.057190958417936644...
        let il = il_v2(2.0).unwrap();
        let expected = 2.0 * 2.0_f64.sqrt() / 3.0 - 1.0;
        assert!((il - expected).abs() < 1e-12);
        assert!((il - (-0.057_190_958_417_936_644)).abs() < 1e-12);
    }

    #[test]
    fn il_v2_known_half() {
        // Symmetry: r=0.5 should match r=2 in magnitude.
        let il_half = il_v2(0.5).unwrap();
        let il_two = il_v2(2.0).unwrap();
        assert!((il_half - il_two).abs() < 1e-12, "IL must be symmetric");
    }

    #[test]
    fn il_v2_increases_in_magnitude_for_larger_moves() {
        let r2 = il_v2(2.0).unwrap().abs();
        let r4 = il_v2(4.0).unwrap().abs();
        let r10 = il_v2(10.0).unwrap().abs();
        assert!(r2 < r4);
        assert!(r4 < r10);
    }

    #[test]
    fn il_concentrated_zero_hold_returns_zero() {
        assert_eq!(il_concentrated(100.0, 0.0), 0.0);
        assert_eq!(il_concentrated(100.0, -1.0), 0.0);
    }

    #[test]
    fn il_concentrated_typical() {
        // Position worth $95, hold-only worth $100 → IL = -5%.
        let il = il_concentrated(95.0, 100.0);
        assert!((il - (-0.05)).abs() < 1e-12);
    }
}
