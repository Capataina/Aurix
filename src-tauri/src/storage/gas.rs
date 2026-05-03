//! Per-block gas price persistence. Sparse — only blocks the ingestion or
//! mgmt-gas costing path explicitly queries are populated.

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::error::StorageError;
use super::Storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockGasRow {
    pub block_number: i64,
    pub block_timestamp: i64,
    pub base_fee_gwei: Option<f64>,
    pub median_gas_gwei: f64,
}

impl Storage {
    pub async fn upsert_block_gas(&self, row: BlockGasRow) -> Result<(), StorageError> {
        self.write(move |conn| {
            conn.execute(
                "INSERT INTO block_gas_prices
                 (block_number, block_timestamp, base_fee_gwei, median_gas_gwei)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(block_number) DO UPDATE SET
                    block_timestamp = excluded.block_timestamp,
                    base_fee_gwei = excluded.base_fee_gwei,
                    median_gas_gwei = excluded.median_gas_gwei",
                params![
                    row.block_number,
                    row.block_timestamp,
                    row.base_fee_gwei,
                    row.median_gas_gwei,
                ],
            )?;
            Ok(())
        })
        .await
    }

    pub async fn upsert_block_gas_batch(
        &self,
        rows: Vec<BlockGasRow>,
    ) -> Result<usize, StorageError> {
        let count = self
            .write(move |conn| {
                let tx = conn.transaction()?;
                let mut count = 0usize;
                {
                    let mut stmt = tx.prepare(
                        "INSERT INTO block_gas_prices
                         (block_number, block_timestamp, base_fee_gwei, median_gas_gwei)
                         VALUES (?1, ?2, ?3, ?4)
                         ON CONFLICT(block_number) DO UPDATE SET
                            block_timestamp = excluded.block_timestamp,
                            base_fee_gwei = excluded.base_fee_gwei,
                            median_gas_gwei = excluded.median_gas_gwei",
                    )?;
                    for r in rows.iter() {
                        let written = stmt.execute(params![
                            r.block_number,
                            r.block_timestamp,
                            r.base_fee_gwei,
                            r.median_gas_gwei,
                        ])?;
                        count += written;
                    }
                }
                tx.commit()?;
                Ok(count)
            })
            .await?;
        Ok(count)
    }

    pub async fn block_gas_at(&self, block: i64) -> Result<Option<BlockGasRow>, StorageError> {
        let pool = self.reader_pool.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<Option<BlockGasRow>, StorageError> {
            let conn = pool.get()?;
            let row = conn
                .query_row(
                    "SELECT block_number, block_timestamp, base_fee_gwei, median_gas_gwei
                     FROM block_gas_prices
                     WHERE block_number = ?1",
                    params![block],
                    |row| {
                        Ok(BlockGasRow {
                            block_number: row.get(0)?,
                            block_timestamp: row.get(1)?,
                            base_fee_gwei: row.get(2)?,
                            median_gas_gwei: row.get(3)?,
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
    async fn upsert_overwrites_on_conflict() {
        let s = Storage::open(DbLocation::in_memory()).await.unwrap();
        s.upsert_block_gas(BlockGasRow {
            block_number: 1000,
            block_timestamp: 1_700_000_000,
            base_fee_gwei: Some(15.0),
            median_gas_gwei: 18.0,
        })
        .await
        .unwrap();
        s.upsert_block_gas(BlockGasRow {
            block_number: 1000,
            block_timestamp: 1_700_000_000,
            base_fee_gwei: Some(20.0),
            median_gas_gwei: 25.0,
        })
        .await
        .unwrap();

        let row = s.block_gas_at(1000).await.unwrap().unwrap();
        assert_eq!(row.median_gas_gwei, 25.0);
        assert_eq!(row.base_fee_gwei, Some(20.0));
    }
}
