use thiserror::Error;

use crate::storage::StorageError;

#[derive(Debug, Error)]
pub enum BenchmarkError {
    #[error("http transport: {0}")]
    Http(String),

    #[error("upstream returned status {0}")]
    Status(u16),

    #[error("response parse failed: {0}")]
    Parse(String),

    #[error("date parse failed: {0}")]
    Date(String),

    #[error("api key required for {0}")]
    KeyRequired(&'static str),

    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error("unsupported series: {0}")]
    UnsupportedSeries(String),
}
