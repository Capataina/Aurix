use thiserror::Error;

use crate::backtest::error::BacktestError;
use crate::storage::StorageError;

#[derive(Debug, Error)]
pub enum StrategyError {
    #[error(transparent)]
    Backtest(#[from] BacktestError),

    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error("invalid grid config: {0}")]
    InvalidConfig(String),

    #[error("no swaps available for the requested period")]
    NoData,
}
