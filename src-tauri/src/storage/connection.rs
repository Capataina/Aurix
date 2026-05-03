//! SQLite connection topology.
//!
//! Aurix uses the read/write split documented in
//! `context/references/sqlite-rust-production-patterns.md` §Question 1:
//!
//! * one writer — a single `tokio_rusqlite::Connection` lives on its own
//!   thread; every write goes through `Storage::write` which calls
//!   `writer.call(...)` and awaits the blocking work.
//! * N readers — an `r2d2_sqlite` pool with `max_size` = the CPU count.
//!   Each read acquires a connection, runs synchronously, releases.
//!
//! WAL mode + the production pragma set is applied on every connection
//! acquisition (writer init + every reader checkout) so the topology is
//! correct even if a connection is newly created mid-run.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tokio_rusqlite::Connection as TokioConnection;

use super::error::StorageError;

/// The pragma set every connection (reader or writer) configures on
/// acquisition. See `sqlite-rust-production-patterns.md` §Question 2 +
/// §Recommendation table for the per-pragma rationale.
///
/// `mmap_size` is set to 128 MiB — phiresky's blog suggests 30 GiB on Linux
/// but that is conservative for cross-platform Tauri (macOS / Windows file
/// systems behave differently around large mmap regions). 128 MiB is the
/// "safe wide" Aurix value.
const PRAGMA_BOOTSTRAP: &str = "
    PRAGMA journal_mode = WAL;
    PRAGMA synchronous = NORMAL;
    PRAGMA busy_timeout = 5000;
    PRAGMA temp_store = MEMORY;
    PRAGMA mmap_size = 134217728;
    PRAGMA cache_size = -65536;
    PRAGMA foreign_keys = ON;
    PRAGMA auto_vacuum = INCREMENTAL;
    PRAGMA wal_autocheckpoint = 1000;
";

/// SQLite open flags used when the location is a shared-cache in-memory
/// database (URI form `file:<name>?mode=memory&cache=shared`). The URI
/// flag is required for SQLite to recognise the URI syntax; without it
/// the path is treated as a literal filename.
fn shared_memory_flags() -> rusqlite::OpenFlags {
    rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
        | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
        | rusqlite::OpenFlags::SQLITE_OPEN_URI
        | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
}

/// Where to put the SQLite file.
///
/// `Path(p)` writes to a literal filesystem path — used by the production
/// app to put the DB inside the platform's app-data dir.
/// `SharedInMemory(name)` uses the shared-cache URI form so the writer and
/// every reader pool connection see the same in-memory database. Each call
/// to `DbLocation::in_memory()` generates a unique name so concurrent tests
/// remain isolated from each other.
#[derive(Debug, Clone)]
pub enum DbLocation {
    Path(PathBuf),
    SharedInMemory(String),
}

static IN_MEMORY_COUNTER: AtomicU64 = AtomicU64::new(0);

impl DbLocation {
    pub fn path(path: impl AsRef<Path>) -> Self {
        Self::Path(path.as_ref().to_path_buf())
    }

    /// Returns a unique-per-call shared-cache in-memory location. Used in
    /// tests; the unique name guarantees isolation between concurrent test
    /// fixtures even though every name resolves to a SQLite-internal mmap
    /// region rather than a real file.
    pub fn in_memory() -> Self {
        let n = IN_MEMORY_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self::SharedInMemory(format!("aurix_mem_{}_{}", std::process::id(), n))
    }

    fn manager(&self) -> SqliteConnectionManager {
        match self {
            Self::Path(p) => SqliteConnectionManager::file(p)
                .with_init(|c| c.execute_batch(PRAGMA_BOOTSTRAP)),
            Self::SharedInMemory(name) => {
                let uri = format!("file:{name}?mode=memory&cache=shared");
                SqliteConnectionManager::file(uri)
                    .with_flags(shared_memory_flags())
                    .with_init(|c| c.execute_batch(PRAGMA_BOOTSTRAP))
            }
        }
    }
}

/// Builds the reader pool. `max_size` defaults to the available CPU count
/// (with a floor of 2) per the read-pool pattern.
pub fn build_reader_pool(
    location: &DbLocation,
) -> Result<Pool<SqliteConnectionManager>, StorageError> {
    let cpu_count = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
        .max(2);

    let manager = location.manager();
    let pool = Pool::builder().max_size(cpu_count).build(manager)?;
    Ok(pool)
}

/// Opens an async writer connection (single-thread, owned by tokio-rusqlite)
/// and applies pragmas via a `call` on the dedicated thread.
pub async fn open_async_writer(
    location: &DbLocation,
) -> Result<TokioConnection, StorageError> {
    let conn = match location {
        DbLocation::Path(p) => TokioConnection::open(p).await?,
        DbLocation::SharedInMemory(name) => {
            let uri = format!("file:{name}?mode=memory&cache=shared");
            TokioConnection::open_with_flags(uri, shared_memory_flags()).await?
        }
    };
    conn.call(|c| {
        c.execute_batch(PRAGMA_BOOTSTRAP)?;
        Ok(())
    })
    .await?;
    Ok(conn)
}
