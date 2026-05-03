//! V3 math primitives (M2.2).
//!
//! Pure-function module — no I/O, no async. Every primitive is bit-exact
//! against `TickMath.sol` / `LiquidityAmounts.sol` / `SqrtPriceMath.sol`
//! within rounding direction (callers supply `round_up` where applicable).
//!
//! Reference: `context/references/v3-mathematics-deep-dive.md`.

// The math primitives form an internal library consumed by M2.3 (sim
// engine), M2.5 (strategies), and M2.7 (V2 LP benchmark). Some primitives
// (Q160/Q192, mul_div_round_up, fee_share_token1) are part of the
// canonical V3 surface even when not yet referenced in callsites.
#![allow(dead_code)]

pub mod error;
pub mod fees;
pub mod il;
pub mod liquidity;
pub mod q96;
pub mod tick;

pub use error::V3MathError;
pub use tick::{sqrt_price_x96_to_tick, tick_to_sqrt_price_x96};
