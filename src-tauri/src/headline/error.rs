use thiserror::Error;

use crate::storage::StorageError;
use crate::strategies::StrategyError;

#[derive(Debug, Error)]
pub enum HeadlineError {
    #[error(transparent)]
    Strategy(#[from] StrategyError),

    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error("insufficient data: need at least {required_months} months, have {actual_months}")]
    InsufficientData {
        required_months: i64,
        actual_months: i64,
    },

    #[error("regime classification failed: {0}")]
    RegimeError(String),
}
