use thiserror::Error;

use crate::backtest::error::BacktestError;
use crate::ingest::IngestError;
use crate::storage::StorageError;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error(transparent)]
    Backtest(#[from] BacktestError),

    #[error(transparent)]
    Ingest(#[from] IngestError),

    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error("validation harness requires {0} but it is not configured (KeyRequired)")]
    KeyRequired(&'static str),

    #[error("position fixture is malformed: {0}")]
    MalformedFixture(String),

    #[error("acceptance criterion failed: {detail}")]
    AcceptanceFailed { detail: String },
}
