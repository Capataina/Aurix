//! Embedded migration runner. Refinery's `embed_migrations!` macro picks up
//! every `V<NNN>__<name>.sql` in the `migrations/` directory at build time
//! and bakes them into the binary, so the deployed app needs no external
//! SQL files at runtime.
//!
//! Forward-only by design (per `context/references/sqlite-rust-production-
//! patterns.md` §Question 4 and `context/plans/vector-a-v3-lp-backtester.md`
//! §M2.0). To undo a migration, write a new one that reverses the change.

use rusqlite::Connection;

use super::error::StorageError;

mod embedded {
    use refinery::embed_migrations;

    embed_migrations!("./src/storage/migrations");
}

/// Applies every pending migration to the supplied connection, in version
/// order. Idempotent — already-applied migrations are skipped.
///
/// Inputs: a mutable reference to a rusqlite `Connection` (the writer
/// connection — migrations must run single-threaded).
/// Outputs: the count of migrations applied during this call.
/// Errors: returned when refinery rejects a migration (checksum mismatch,
/// SQL error, or version gap).
/// Side effects: creates / extends the schema; writes a row to the
/// `refinery_schema_history` bookkeeping table.
pub fn run(conn: &mut Connection) -> Result<usize, StorageError> {
    let report = embedded::migrations::runner().run(conn)?;
    Ok(report.applied_migrations().len())
}
