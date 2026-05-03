use thiserror::Error;

use crate::math::V3MathError;
use crate::storage::StorageError;

#[derive(Debug, Error)]
pub enum BacktestError {
    #[error(transparent)]
    Math(#[from] V3MathError),

    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error("invalid position config: {0}")]
    InvalidConfig(String),

    #[error("no swaps available for {pool} between blocks {from} and {to}")]
    EmptyData {
        pool: String,
        from: u64,
        to: u64,
    },

    #[error("decoded value parse error: {0}")]
    Parse(String),

    #[error("entry block {entry} > exit block {exit}")]
    InvertedBlocks { entry: u64, exit: u64 },
}
