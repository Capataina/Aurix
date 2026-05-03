//! V3 math error type. Per `context/notes/error-handling.md`, one
//! `thiserror::Error` enum per module.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum V3MathError {
    #[error("tick out of bounds (|tick| must be ≤ 887272, got {0})")]
    TickOutOfBounds(i32),

    #[error("sqrtPriceX96 out of bounds (must be in [MIN_SQRT_RATIO, MAX_SQRT_RATIO])")]
    SqrtRatioOutOfBounds,

    #[error("invalid range: tick_lower ({0}) must be < tick_upper ({1})")]
    InvalidTickRange(i32, i32),

    #[error("tick {tick} is not aligned to spacing {spacing}")]
    TickSpacingMismatch { tick: i32, spacing: i32 },

    #[error("liquidity overflow")]
    LiquidityOverflow,

    #[error("amount conversion failed: {0}")]
    AmountConversion(String),

    #[error("division by zero")]
    DivisionByZero,
}
