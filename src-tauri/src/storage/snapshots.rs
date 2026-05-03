//! Tab 1 price-snapshot persistence.
//!
//! Persists every Tab 1 IPC tick. Idempotent on (pair_id, dex_name,
//! fetched_at_unix_ms) — re-inserting the same tick is a no-op via
//! `INSERT OR IGNORE`.

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::error::StorageError;
use super::Storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceSnapshotRow {
    pub chain: String,
    pub pair_id: String,
    pub dex_name: String,
    pub pool_address: String,
    pub fee_tier_bps: u16,
    pub price_usd: f64,
    pub fetched_at_unix_ms: i64,
    pub gas_price_gwei: Option<f64>,
}

impl Storage {
    /// Inserts a snapshot. Returns `true` if a new row was written, `false`
    /// if the (pair_id, dex_name, fetched_at_unix_ms) tuple already exists.
    pub async fn insert_price_snapshot(
        &self,
        snapshot: PriceSnapshotRow,
    ) -> Result<bool, StorageError> {
        let inserted = self
            .write(move |conn| {
                let rows = conn.execute(
                    "INSERT OR IGNORE INTO price_snapshots
                     (chain, pair_id, dex_name, pool_address, fee_tier_bps,
                      price_usd, fetched_at_unix_ms, gas_price_gwei)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        snapshot.chain,
                        snapshot.pair_id,
                        snapshot.dex_name,
                        snapshot.pool_address,
                        snapshot.fee_tier_bps,
                        snapshot.price_usd,
                        snapshot.fetched_at_unix_ms,
                        snapshot.gas_price_gwei,
                    ],
                )?;
                Ok(rows > 0)
            })
            .await?;
        Ok(inserted)
    }

    /// Bulk-inserts a batch of snapshots inside a single transaction (per
    /// `sqlite-rust-production-patterns.md` §Question 7 — never one-row-
    /// per-txn). Returns the count of rows actually inserted (skipping
    /// duplicates).
    pub async fn insert_price_snapshots_batch(
        &self,
        snapshots: Vec<PriceSnapshotRow>,
    ) -> Result<usize, StorageError> {
        let inserted = self
            .write(move |conn| {
                let tx = conn.transaction()?;
                let mut count = 0usize;
                {
                    let mut stmt = tx.prepare(
                        "INSERT OR IGNORE INTO price_snapshots
                         (chain, pair_id, dex_name, pool_address, fee_tier_bps,
                          price_usd, fetched_at_unix_ms, gas_price_gwei)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    )?;
                    for s in snapshots.iter() {
                        let rows = stmt.execute(params![
                            s.chain,
                            s.pair_id,
                            s.dex_name,
                            s.pool_address,
                            s.fee_tier_bps,
                            s.price_usd,
                            s.fetched_at_unix_ms,
                            s.gas_price_gwei,
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

    /// Returns snapshots for a pair within `[start_ms, end_ms]`, ordered by
    /// time ascending. Optionally filters to a single dex.
    pub async fn query_price_snapshots(
        &self,
        pair_id: String,
        start_ms: i64,
        end_ms: i64,
        dex_name: Option<String>,
    ) -> Result<Vec<PriceSnapshotRow>, StorageError> {
        let pool = self.reader_pool.clone();
        let rows = tokio::task::spawn_blocking(move || -> Result<Vec<PriceSnapshotRow>, StorageError> {
            let conn = pool.get()?;
            let mut rows = Vec::new();
            if let Some(dex) = dex_name {
                let mut stmt = conn.prepare(
                    "SELECT chain, pair_id, dex_name, pool_address, fee_tier_bps,
                            price_usd, fetched_at_unix_ms, gas_price_gwei
                     FROM price_snapshots
                     WHERE pair_id = ?1 AND dex_name = ?2
                       AND fetched_at_unix_ms BETWEEN ?3 AND ?4
                     ORDER BY fetched_at_unix_ms ASC",
                )?;
                let iter = stmt.query_map(params![pair_id, dex, start_ms, end_ms], |row| {
                    Ok(PriceSnapshotRow {
                        chain: row.get(0)?,
                        pair_id: row.get(1)?,
                        dex_name: row.get(2)?,
                        pool_address: row.get(3)?,
                        fee_tier_bps: row.get::<_, i64>(4)? as u16,
                        price_usd: row.get(5)?,
                        fetched_at_unix_ms: row.get(6)?,
                        gas_price_gwei: row.get(7)?,
                    })
                })?;
                for r in iter {
                    rows.push(r?);
                }
            } else {
                let mut stmt = conn.prepare(
                    "SELECT chain, pair_id, dex_name, pool_address, fee_tier_bps,
                            price_usd, fetched_at_unix_ms, gas_price_gwei
                     FROM price_snapshots
                     WHERE pair_id = ?1
                       AND fetched_at_unix_ms BETWEEN ?2 AND ?3
                     ORDER BY fetched_at_unix_ms ASC",
                )?;
                let iter = stmt.query_map(params![pair_id, start_ms, end_ms], |row| {
                    Ok(PriceSnapshotRow {
                        chain: row.get(0)?,
                        pair_id: row.get(1)?,
                        dex_name: row.get(2)?,
                        pool_address: row.get(3)?,
                        fee_tier_bps: row.get::<_, i64>(4)? as u16,
                        price_usd: row.get(5)?,
                        fetched_at_unix_ms: row.get(6)?,
                        gas_price_gwei: row.get(7)?,
                    })
                })?;
                for r in iter {
                    rows.push(r?);
                }
            }
            Ok(rows)
        })
        .await
        .map_err(|e| StorageError::WriterUnavailable(e.to_string()))??;
        Ok(rows)
    }

    /// Returns the most recent snapshot for `(pair_id, dex_name)`, if any.
    pub async fn latest_price_snapshot(
        &self,
        pair_id: String,
        dex_name: String,
    ) -> Result<Option<PriceSnapshotRow>, StorageError> {
        let pool = self.reader_pool.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<Option<PriceSnapshotRow>, StorageError> {
            let conn = pool.get()?;
            let row = conn
                .query_row(
                    "SELECT chain, pair_id, dex_name, pool_address, fee_tier_bps,
                            price_usd, fetched_at_unix_ms, gas_price_gwei
                     FROM price_snapshots
                     WHERE pair_id = ?1 AND dex_name = ?2
                     ORDER BY fetched_at_unix_ms DESC
                     LIMIT 1",
                    params![pair_id, dex_name],
                    |row| {
                        Ok(PriceSnapshotRow {
                            chain: row.get(0)?,
                            pair_id: row.get(1)?,
                            dex_name: row.get(2)?,
                            pool_address: row.get(3)?,
                            fee_tier_bps: row.get::<_, i64>(4)? as u16,
                            price_usd: row.get(5)?,
                            fetched_at_unix_ms: row.get(6)?,
                            gas_price_gwei: row.get(7)?,
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

    fn sample(ts_ms: i64, dex: &str, price: f64) -> PriceSnapshotRow {
        PriceSnapshotRow {
            chain: "Ethereum Mainnet".into(),
            pair_id: "weth-usdc".into(),
            dex_name: dex.into(),
            pool_address: "0xdeadbeef".into(),
            fee_tier_bps: 5,
            price_usd: price,
            fetched_at_unix_ms: ts_ms,
            gas_price_gwei: Some(20.0),
        }
    }

    #[tokio::test]
    async fn insert_and_query_round_trip() {
        let s = Storage::open(DbLocation::in_memory()).await.unwrap();
        s.insert_price_snapshot(sample(1, "v3", 3000.0)).await.unwrap();
        s.insert_price_snapshot(sample(2, "v3", 3001.0)).await.unwrap();
        s.insert_price_snapshot(sample(3, "v3", 3002.0)).await.unwrap();
        let rows = s
            .query_price_snapshots("weth-usdc".into(), 1, 3, None)
            .await
            .unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].fetched_at_unix_ms, 1);
        assert_eq!(rows[2].price_usd, 3002.0);
    }

    #[tokio::test]
    async fn insert_is_idempotent_on_duplicate_key() {
        let s = Storage::open(DbLocation::in_memory()).await.unwrap();
        let inserted_first = s
            .insert_price_snapshot(sample(10, "v3", 3000.0))
            .await
            .unwrap();
        assert!(inserted_first);
        let inserted_again = s
            .insert_price_snapshot(sample(10, "v3", 999.0))
            .await
            .unwrap();
        assert!(!inserted_again, "duplicate insert must return false");

        let rows = s
            .query_price_snapshots("weth-usdc".into(), 10, 10, None)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].price_usd, 3000.0, "first write wins, second ignored");
    }

    #[tokio::test]
    async fn batch_insert_runs_in_one_transaction() {
        let s = Storage::open(DbLocation::in_memory()).await.unwrap();
        let batch: Vec<_> = (0..100)
            .map(|i| sample(100 + i, "v3", 3000.0 + i as f64))
            .collect();
        let count = s.insert_price_snapshots_batch(batch).await.unwrap();
        assert_eq!(count, 100);

        let latest = s
            .latest_price_snapshot("weth-usdc".into(), "v3".into())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(latest.fetched_at_unix_ms, 199);
    }

    #[tokio::test]
    async fn dex_filter_works() {
        let s = Storage::open(DbLocation::in_memory()).await.unwrap();
        s.insert_price_snapshot(sample(1, "v3", 3000.0)).await.unwrap();
        s.insert_price_snapshot(sample(1, "v2", 2999.5)).await.unwrap();

        let v3 = s
            .query_price_snapshots("weth-usdc".into(), 1, 1, Some("v3".into()))
            .await
            .unwrap();
        assert_eq!(v3.len(), 1);
        assert_eq!(v3[0].dex_name, "v3");
    }
}
