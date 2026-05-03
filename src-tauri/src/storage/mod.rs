//! Persistence layer (M2.0).
//!
//! Topology — one writer connection on its own thread (via tokio-rusqlite),
//! one reader pool (via r2d2_sqlite). Public API exposes async write
//! methods and sync (or `spawn_blocking`-able) read methods. WAL mode
//! plus the production pragma set are applied on every connection
//! acquisition. Migration is forward-only via embedded refinery.
//!
//! Reference: `context/references/sqlite-rust-production-patterns.md`,
//! `context/plans/vector-a-v3-lp-backtester.md` §M2.0.
//!
//! Public surface organised by domain:
//!
//! | Submodule        | Responsibility                                      |
//! | ---------------- | --------------------------------------------------- |
//! | `connection`     | Pragmas, reader pool, async writer, db location.    |
//! | `migrations`     | Refinery embed + run.                               |
//! | `error`          | One `StorageError` enum.                            |
//! | `snapshots`      | Tab 1 price snapshot CRUD.                          |
//! | `swaps`          | V3 Swap event CRUD + idempotent batch insert.       |
//! | `pool_events`    | V3 Mint/Burn/Collect event CRUD.                    |
//! | `gas`            | Per-block gas-price persistence.                    |
//! | `runs`           | Position simulation runs + equity curve points.     |
//! | `strategy`       | Strategy comparison grid results.                   |
//! | `benchmarks`     | Daily benchmark series cache.                       |
//! | `headline`       | Headline run + per-month outputs (M2.8).            |
//! | `state`          | Ingestion checkpoints.                              |

mod connection;
mod error;
mod migrations;

pub mod benchmarks;
pub mod gas;
pub mod headline;
pub mod pool_events;
pub mod runs;
pub mod snapshots;
pub mod state;
pub mod strategy;
pub mod swaps;

pub use connection::DbLocation;
pub use error::StorageError;

use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use tokio_rusqlite::Connection as TokioConnection;

/// Top-level storage handle. Wraps the writer connection and the reader
/// pool. Cheap to clone (both halves are `Arc`-like internally).
#[derive(Clone)]
pub struct Storage {
    writer: TokioConnection,
    reader_pool: Pool<SqliteConnectionManager>,
}

impl Storage {
    /// Opens the storage layer at `location`, runs migrations, returns the
    /// configured handle.
    ///
    /// Inputs: a `DbLocation` — production code uses `DbLocation::path`,
    /// tests use `DbLocation::in_memory`.
    /// Outputs: a `Storage` ready for reads and writes.
    /// Errors: returned when the connection cannot be opened, pragmas
    /// fail, or migration fails.
    /// Side effects: creates the database file (if a path was supplied
    /// and the file did not exist) and writes migration history.
    pub async fn open(location: DbLocation) -> Result<Self, StorageError> {
        let writer = connection::open_async_writer(&location).await?;
        // Run migrations through a blocking call on the writer's own
        // thread (refinery uses sync rusqlite).
        writer
            .call(|conn| {
                migrations::run(conn).map_err(|e| {
                    tokio_rusqlite::Error::Other(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string(),
                    )))
                })?;
                Ok(())
            })
            .await?;

        let reader_pool = connection::build_reader_pool(&location)?;
        Ok(Self {
            writer,
            reader_pool,
        })
    }

    /// Borrows the async writer for one-shot mutations. Callers pass a
    /// blocking closure that takes `&mut rusqlite::Connection`; the
    /// closure runs on the writer's dedicated thread.
    pub async fn write<R, F>(&self, f: F) -> Result<R, StorageError>
    where
        R: Send + 'static,
        F: FnOnce(&mut rusqlite::Connection) -> rusqlite::Result<R> + Send + 'static,
    {
        let result = self.writer.call(move |conn| Ok(f(conn))).await?;
        Ok(result?)
    }

    /// Acquires a reader connection from the pool. The handle is bound to
    /// the calling task; release happens on drop. Reader connections must
    /// not perform writes — the WAL topology + pool sizing assumes the
    /// invariant.
    pub fn read(&self) -> Result<PooledConnection<SqliteConnectionManager>, StorageError> {
        Ok(self.reader_pool.get()?)
    }

    /// Issues a `wal_checkpoint(TRUNCATE)` on the writer. Should be called
    /// periodically (e.g. every N writes or every M seconds) to bound WAL
    /// growth — see `sqlite-rust-production-patterns.md` §Question 8 on
    /// checkpoint starvation.
    pub async fn checkpoint(&self) -> Result<(), StorageError> {
        self.writer
            .call(|conn| {
                // PRAGMA returns rows; iterate to drive the call but discard.
                let _: i64 = conn
                    .query_row("PRAGMA wal_checkpoint(TRUNCATE);", [], |row| row.get(0))
                    .unwrap_or(0);
                Ok(())
            })
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn opens_in_memory_and_runs_migrations() {
        let storage = Storage::open(DbLocation::in_memory())
            .await
            .expect("open in-memory storage");

        // Inspect refinery's bookkeeping table to confirm V001 ran.
        let conn = storage.read().expect("acquire reader");
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM refinery_schema_history",
                [],
                |row| row.get(0),
            )
            .expect("schema history table exists");
        assert!(count >= 1, "expected at least one migration applied");
    }

    #[tokio::test]
    async fn applies_wal_pragmas_on_writer() {
        let storage = Storage::open(DbLocation::in_memory())
            .await
            .expect("open in-memory storage");

        let mode: String = storage
            .write(|conn| conn.query_row("PRAGMA journal_mode", [], |row| row.get(0)))
            .await
            .expect("query journal_mode");
        // In-memory dbs report "memory" (not WAL) but the pragma is still
        // exercised; on disk this would be "wal".
        assert!(
            mode == "memory" || mode == "wal",
            "unexpected journal_mode: {mode}"
        );

        let foreign_keys: i64 = storage
            .write(|conn| conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0)))
            .await
            .expect("query foreign_keys");
        assert_eq!(foreign_keys, 1, "foreign_keys must be ON");
    }

    #[tokio::test]
    async fn migrations_are_idempotent() {
        let storage = Storage::open(DbLocation::in_memory())
            .await
            .expect("open in-memory storage");
        // Re-running migrations through the writer should be a no-op.
        let count = storage
            .write(|conn| {
                migrations::run(conn).map_err(|e| {
                    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string(),
                    )))
                })
            })
            .await
            .expect("re-run migrations");
        assert_eq!(count, 0, "no migrations should be applied on the second run");
    }
}
