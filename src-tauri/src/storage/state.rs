//! Ingestion checkpoints. M2.1 reads these to resume from the last
//! successfully-persisted block when restarted.

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::error::StorageError;
use super::Storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionState {
    pub pool_address: String,
    pub last_swap_block: i64,
    pub last_pool_event_block: i64,
    pub last_run_at_unix_ms: i64,
}

impl Storage {
    pub async fn upsert_ingestion_state(
        &self,
        state: IngestionState,
    ) -> Result<(), StorageError> {
        self.write(move |conn| {
            conn.execute(
                "INSERT INTO ingestion_state
                 (pool_address, last_swap_block, last_pool_event_block, last_run_at_unix_ms)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(pool_address) DO UPDATE SET
                    last_swap_block = excluded.last_swap_block,
                    last_pool_event_block = excluded.last_pool_event_block,
                    last_run_at_unix_ms = excluded.last_run_at_unix_ms",
                params![
                    state.pool_address,
                    state.last_swap_block,
                    state.last_pool_event_block,
                    state.last_run_at_unix_ms,
                ],
            )?;
            Ok(())
        })
        .await
    }

    pub async fn ingestion_state(
        &self,
        pool_address: String,
    ) -> Result<Option<IngestionState>, StorageError> {
        let pool = self.reader_pool.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<Option<IngestionState>, StorageError> {
            let conn = pool.get()?;
            let row = conn
                .query_row(
                    "SELECT pool_address, last_swap_block, last_pool_event_block, last_run_at_unix_ms
                     FROM ingestion_state
                     WHERE pool_address = ?1",
                    params![pool_address],
                    |row| {
                        Ok(IngestionState {
                            pool_address: row.get(0)?,
                            last_swap_block: row.get(1)?,
                            last_pool_event_block: row.get(2)?,
                            last_run_at_unix_ms: row.get(3)?,
                        })
                    },
                )
                .optional()?;
            Ok(row)
        })
        .await
        .map_err(|e| StorageError::WriterUnavailable(e.to_string()))??;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::DbLocation;

    #[tokio::test]
    async fn upsert_round_trip() {
        let s = Storage::open(DbLocation::in_memory()).await.unwrap();
        s.upsert_ingestion_state(IngestionState {
            pool_address: "0xpool".into(),
            last_swap_block: 1000,
            last_pool_event_block: 999,
            last_run_at_unix_ms: 1_700_000_000_000,
        })
        .await
        .unwrap();
        let state = s
            .ingestion_state("0xpool".into())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(state.last_swap_block, 1000);

        s.upsert_ingestion_state(IngestionState {
            pool_address: "0xpool".into(),
            last_swap_block: 2000,
            last_pool_event_block: 1999,
            last_run_at_unix_ms: 1_700_000_001_000,
        })
        .await
        .unwrap();
        let state = s.ingestion_state("0xpool".into()).await.unwrap().unwrap();
        assert_eq!(state.last_swap_block, 2000);
    }
}
