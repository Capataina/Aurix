//! V3 Swap event persistence.
//!
//! Each swap is keyed by (pool_address, block_number, log_index) — chain-
//! globally unique. Inserts use `INSERT OR IGNORE` so re-running ingestion
//! against the same block range is a no-op (idempotency contract per
//! `vector-a-v3-lp-backtester.md` §M2.1).
//!
//! `amount0`, `amount1`, `sqrt_price_x96`, and `liquidity` are stored as
//! TEXT decimal strings to preserve full uint160 / int256 / uint128
//! precision; callers parse to `BigInt` / `BigUint` on read.

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::error::StorageError;
use super::Storage;

/// One V3 Swap log entry, after ABI decode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapEventRow {
    pub pool_address: String,
    pub block_number: i64,
    pub log_index: i64,
    pub transaction_hash: String,
    pub block_timestamp: i64,
    pub sender: String,
    pub recipient: String,
    pub amount0: String,
    pub amount1: String,
    pub sqrt_price_x96: String,
    pub liquidity: String,
    pub tick: i32,
    pub block_gas_price_gwei: Option<f64>,
}

impl Storage {
    /// Bulk-inserts swap events inside a single transaction. Returns the
    /// number of rows actually written (skipping duplicates).
    pub async fn insert_swap_events_batch(
        &self,
        events: Vec<SwapEventRow>,
    ) -> Result<usize, StorageError> {
        let inserted = self
            .write(move |conn| {
                let tx = conn.transaction()?;
                let mut count = 0usize;
                {
                    let mut stmt = tx.prepare(
                        "INSERT OR IGNORE INTO swap_events
                         (pool_address, block_number, log_index, transaction_hash,
                          block_timestamp, sender, recipient,
                          amount0, amount1, sqrt_price_x96, liquidity, tick,
                          block_gas_price_gwei)
                         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
                    )?;
                    for e in events.iter() {
                        let rows = stmt.execute(params![
                            e.pool_address,
                            e.block_number,
                            e.log_index,
                            e.transaction_hash,
                            e.block_timestamp,
                            e.sender,
                            e.recipient,
                            e.amount0,
                            e.amount1,
                            e.sqrt_price_x96,
                            e.liquidity,
                            e.tick,
                            e.block_gas_price_gwei,
                        ])?;
                        count += rows;
                    }
                }
                tx.commit()?;
                Ok(count)
            })
            .await?;
        Ok(inserted)
    }

    /// Returns swap events for a pool within `[start_block, end_block]`,
    /// ordered by (block_number, log_index) ascending — the canonical
    /// per-pool replay order.
    pub async fn query_swaps_for_pool_range(
        &self,
        pool_address: String,
        start_block: i64,
        end_block: i64,
    ) -> Result<Vec<SwapEventRow>, StorageError> {
        // On-chain log.address is always lowercase; user-supplied pool
        // addresses (EIP-55 checksummed) need to match.
        let pool_address = pool_address.to_lowercase();
        let pool = self.reader_pool.clone();
        let rows = tokio::task::spawn_blocking(move || -> Result<Vec<SwapEventRow>, StorageError> {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(
                "SELECT pool_address, block_number, log_index, transaction_hash,
                        block_timestamp, sender, recipient,
                        amount0, amount1, sqrt_price_x96, liquidity, tick,
                        block_gas_price_gwei
                 FROM swap_events
                 WHERE pool_address = ?1
                   AND block_number BETWEEN ?2 AND ?3
                 ORDER BY block_number ASC, log_index ASC",
            )?;
            let iter = stmt.query_map(params![pool_address, start_block, end_block], |row| {
                Ok(SwapEventRow {
                    pool_address: row.get(0)?,
                    block_number: row.get(1)?,
                    log_index: row.get(2)?,
                    transaction_hash: row.get(3)?,
                    block_timestamp: row.get(4)?,
                    sender: row.get(5)?,
                    recipient: row.get(6)?,
                    amount0: row.get(7)?,
                    amount1: row.get(8)?,
                    sqrt_price_x96: row.get(9)?,
                    liquidity: row.get(10)?,
                    tick: row.get::<_, i64>(11)? as i32,
                    block_gas_price_gwei: row.get(12)?,
                })
            })?;
            let mut out = Vec::new();
            for r in iter {
                out.push(r?);
            }
            Ok(out)
        })
        .await
        .map_err(|e| StorageError::WriterUnavailable(e.to_string()))??;
        Ok(rows)
    }

    /// Returns the highest block number for which we have swaps in `pool`.
    /// `None` when no swaps exist for the pool.
    pub async fn max_swap_block_for_pool(
        &self,
        pool_address: String,
    ) -> Result<Option<i64>, StorageError> {
        let pool_address = pool_address.to_lowercase();
        let pool = self.reader_pool.clone();
        let max_block = tokio::task::spawn_blocking(move || -> Result<Option<i64>, StorageError> {
            let conn = pool.get()?;
            let result: Option<i64> = conn
                .query_row(
                    "SELECT MAX(block_number) FROM swap_events WHERE pool_address = ?1",
                    params![pool_address],
                    |row| row.get(0),
                )
                .optional()?
                .flatten();
            Ok(result)
        })
        .await
        .map_err(|e| StorageError::WriterUnavailable(e.to_string()))??;
        Ok(max_block)
    }

    /// Returns the count of swap rows in the table — useful for tests +
    /// dashboards.
    pub async fn count_swap_events(&self, pool_address: String) -> Result<i64, StorageError> {
        let pool_address = pool_address.to_lowercase();
        let pool = self.reader_pool.clone();
        let count = tokio::task::spawn_blocking(move || -> Result<i64, StorageError> {
            let conn = pool.get()?;
            let n: i64 = conn.query_row(
                "SELECT COUNT(*) FROM swap_events WHERE pool_address = ?1",
                params![pool_address],
                |row| row.get(0),
            )?;
            Ok(n)
        })
        .await
        .map_err(|e| StorageError::WriterUnavailable(e.to_string()))??;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::DbLocation;

    fn sample(block: i64, log: i64) -> SwapEventRow {
        SwapEventRow {
            pool_address: "0xpool".into(),
            block_number: block,
            log_index: log,
            transaction_hash: format!("0xtx{block}_{log}"),
            block_timestamp: 1_700_000_000 + block * 12,
            sender: "0xsender".into(),
            recipient: "0xrecipient".into(),
            amount0: "-1000000000000000000".into(),
            amount1: "3000000000".into(),
            sqrt_price_x96: "1382037470929380185091293796".into(),
            liquidity: "1000000000000000000".into(),
            tick: 200500,
            block_gas_price_gwei: Some(20.0),
        }
    }

    #[tokio::test]
    async fn batch_insert_and_range_query() {
        let s = Storage::open(DbLocation::in_memory()).await.unwrap();
        let batch = vec![sample(100, 0), sample(100, 1), sample(101, 0), sample(102, 0)];
        let n = s.insert_swap_events_batch(batch).await.unwrap();
        assert_eq!(n, 4);

        let range = s
            .query_swaps_for_pool_range("0xpool".into(), 100, 101)
            .await
            .unwrap();
        assert_eq!(range.len(), 3);
        // ordering check
        assert_eq!(range[0].block_number, 100);
        assert_eq!(range[0].log_index, 0);
        assert_eq!(range[1].log_index, 1);
        assert_eq!(range[2].block_number, 101);
    }

    #[tokio::test]
    async fn batch_insert_is_idempotent() {
        let s = Storage::open(DbLocation::in_memory()).await.unwrap();
        let batch = vec![sample(10, 0), sample(10, 1)];
        let first = s.insert_swap_events_batch(batch.clone()).await.unwrap();
        assert_eq!(first, 2);
        let second = s.insert_swap_events_batch(batch).await.unwrap();
        assert_eq!(second, 0, "duplicate batch must not insert");
        assert_eq!(s.count_swap_events("0xpool".into()).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn max_block_returns_none_for_empty_pool() {
        let s = Storage::open(DbLocation::in_memory()).await.unwrap();
        assert_eq!(s.max_swap_block_for_pool("0xpool".into()).await.unwrap(), None);
        s.insert_swap_events_batch(vec![sample(50, 0), sample(70, 0), sample(60, 0)])
            .await
            .unwrap();
        assert_eq!(
            s.max_swap_block_for_pool("0xpool".into()).await.unwrap(),
            Some(70)
        );
    }
}
