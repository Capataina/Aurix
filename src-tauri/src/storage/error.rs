//! Storage-layer error type. One `thiserror::Error` enum per module per the
//! `context/notes/error-handling.md` convention.

use thiserror::Error;

/// Errors emitted by the storage layer.
///
/// Variants wrap underlying driver / connection / migration errors with
/// `#[error(transparent)]` so callers can downcast or print without losing
/// the original cause. Domain-level errors (missing rows, idempotency
/// violations) carry contextual strings rather than dynamic `Box<dyn Error>`.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("rusqlite error: {0}")]
    Rusqlite(#[from] rusqlite::Error),

    #[error("tokio-rusqlite error: {0}")]
    TokioRusqlite(#[from] tokio_rusqlite::Error),

    #[error("r2d2 connection-pool error: {0}")]
    Pool(#[from] r2d2::Error),

    #[error("refinery migration error: {0}")]
    Migration(#[from] refinery::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to acquire writer lock: {0}")]
    WriterUnavailable(String),

    #[error("query returned no row (table={table}, key={key})")]
    NotFound { table: &'static str, key: String },

    #[error("idempotency violation: {0}")]
    IdempotencyViolation(String),

    #[error("conversion error: {0}")]
    Conversion(String),
}

impl StorageError {
    pub fn conversion<E: std::fmt::Display>(detail: E) -> Self {
        Self::Conversion(detail.to_string())
    }
}
