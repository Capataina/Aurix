//! V3 Mint / Burn / Collect event persistence.
//!
//! All three event kinds share the same shape (owner, tickLower, tickUpper,
//! liquidity, amount0, amount1) so they live in one table with a `kind`
//! column. The reconstruct-time-varying-liquidity-surface query (M2.3)
//! becomes a single scan. Plan paper 1 §4 + plan paper 3.

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::error::StorageError;
use super::Storage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PoolEventKind {
    Mint,
    Burn,
    Collect,
}

impl PoolEventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mint => "mint",
            Self::Burn => "burn",
            Self::Collect => "collect",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "mint" => Some(Self::Mint),
            "burn" => Some(Self::Burn),
            "collect" => Some(Self::Collect),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolEventRow {
    pub pool_address: String,
    pub block_number: i64,
    pub log_index: i64,
    pub transaction_hash: String,
    pub block_timestamp: i64,
    pub kind: PoolEventKind,
    pub owner: String,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: String,
    pub amount0: String,
    pub amount1: String,
}

impl Storage {
    /// Bulk-inserts pool events idempotently.
    pub async fn insert_pool_events_batch(
        &self,
        events: Vec<PoolEventRow>,
    ) -> Result<usize, StorageError> {
        let inserted = self
            .write(move |conn| {
                let tx = conn.transaction()?;
                let mut count = 0usize;
                {
                    let mut stmt = tx.prepare(
                        "INSERT OR IGNORE INTO pool_events
                         (pool_address, block_number, log_index, transaction_hash,
                          block_timestamp, kind, owner,
                          tick_lower, tick_upper, liquidity, amount0, amount1)
                         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                    )?;
                    for e in events.iter() {
                        let rows = stmt.execute(params![
                            e.pool_address,
                            e.block_number,
                            e.log_index,
                            e.transaction_hash,
                            e.block_timestamp,
                            e.kind.as_str(),
                            e.owner,
                            e.tick_lower,
                            e.tick_upper,
                            e.liquidity,
                            e.amount0,
                            e.amount1,
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

    /// Returns all pool events for `pool` within `[start_block, end_block]`,
    /// ordered chronologically. Optionally filters by kind.
    pub async fn query_pool_events_for_range(
        &self,
        pool_address: String,
        start_block: i64,
        end_block: i64,
        kind: Option<PoolEventKind>,
    ) -> Result<Vec<PoolEventRow>, StorageError> {
        let pool = self.reader_pool.clone();
        let rows = tokio::task::spawn_blocking(move || -> Result<Vec<PoolEventRow>, StorageError> {
            let conn = pool.get()?;
            let mut rows = Vec::new();
            if let Some(k) = kind {
                let mut stmt = conn.prepare(
                    "SELECT pool_address, block_number, log_index, transaction_hash,
                            block_timestamp, kind, owner,
                            tick_lower, tick_upper, liquidity, amount0, amount1
                     FROM pool_events
                     WHERE pool_address = ?1 AND kind = ?2
                       AND block_number BETWEEN ?3 AND ?4
                     ORDER BY block_number ASC, log_index ASC",
                )?;
                let iter = stmt.query_map(
                    params![pool_address, k.as_str(), start_block, end_block],
                    map_pool_event_row,
                )?;
                for r in iter {
                    rows.push(r?);
                }
            } else {
                let mut stmt = conn.prepare(
                    "SELECT pool_address, block_number, log_index, transaction_hash,
                            block_timestamp, kind, owner,
                            tick_lower, tick_upper, liquidity, amount0, amount1
                     FROM pool_events
                     WHERE pool_address = ?1
                       AND block_number BETWEEN ?2 AND ?3
                     ORDER BY block_number ASC, log_index ASC",
                )?;
                let iter = stmt.query_map(
                    params![pool_address, start_block, end_block],
                    map_pool_event_row,
                )?;
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

    /// Returns active mint events for `owner` within the pool — useful for
    /// the validation harness which reconstructs known LP positions.
    pub async fn query_owner_mints(
        &self,
        pool_address: String,
        owner: String,
    ) -> Result<Vec<PoolEventRow>, StorageError> {
        let pool = self.reader_pool.clone();
        let rows = tokio::task::spawn_blocking(move || -> Result<Vec<PoolEventRow>, StorageError> {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(
                "SELECT pool_address, block_number, log_index, transaction_hash,
                        block_timestamp, kind, owner,
                        tick_lower, tick_upper, liquidity, amount0, amount1
                 FROM pool_events
                 WHERE pool_address = ?1 AND owner = ?2 AND kind = 'mint'
                 ORDER BY block_number ASC, log_index ASC",
            )?;
            let iter = stmt.query_map(params![pool_address, owner], map_pool_event_row)?;
            let mut rows = Vec::new();
            for r in iter {
                rows.push(r?);
            }
            Ok(rows)
        })
        .await
        .map_err(|e| StorageError::WriterUnavailable(e.to_string()))??;
        Ok(rows)
    }
}

fn map_pool_event_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PoolEventRow> {
    let kind_str: String = row.get(5)?;
    let kind = PoolEventKind::from_str(&kind_str).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            5,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unknown pool_events.kind value: {kind_str}"),
            )),
        )
    })?;
    Ok(PoolEventRow {
        pool_address: row.get(0)?,
        block_number: row.get(1)?,
        log_index: row.get(2)?,
        transaction_hash: row.get(3)?,
        block_timestamp: row.get(4)?,
        kind,
        owner: row.get(6)?,
        tick_lower: row.get::<_, i64>(7)? as i32,
        tick_upper: row.get::<_, i64>(8)? as i32,
        liquidity: row.get(9)?,
        amount0: row.get(10)?,
        amount1: row.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::DbLocation;

    fn sample(block: i64, log: i64, kind: PoolEventKind) -> PoolEventRow {
        PoolEventRow {
            pool_address: "0xpool".into(),
            block_number: block,
            log_index: log,
            transaction_hash: format!("0xtx{block}_{log}"),
            block_timestamp: 1_700_000_000 + block * 12,
            kind,
            owner: "0xowner".into(),
            tick_lower: 200000,
            tick_upper: 201000,
            liquidity: "1000000000000".into(),
            amount0: "1000000000000000000".into(),
            amount1: "3000000000".into(),
        }
    }

    #[tokio::test]
    async fn round_trip_mints_and_burns() {
        let s = Storage::open(DbLocation::in_memory()).await.unwrap();
        s.insert_pool_events_batch(vec![
            sample(10, 0, PoolEventKind::Mint),
            sample(20, 0, PoolEventKind::Burn),
            sample(20, 1, PoolEventKind::Collect),
        ])
        .await
        .unwrap();

        let all = s
            .query_pool_events_for_range("0xpool".into(), 0, 100, None)
            .await
            .unwrap();
        assert_eq!(all.len(), 3);

        let mints = s
            .query_pool_events_for_range("0xpool".into(), 0, 100, Some(PoolEventKind::Mint))
            .await
            .unwrap();
        assert_eq!(mints.len(), 1);
        assert_eq!(mints[0].kind, PoolEventKind::Mint);
    }

    #[tokio::test]
    async fn check_constraint_rejects_unknown_kind() {
        let s = Storage::open(DbLocation::in_memory()).await.unwrap();
        let result = s
            .write(|conn| {
                conn.execute(
                    "INSERT INTO pool_events
                     (pool_address, block_number, log_index, transaction_hash,
                      block_timestamp, kind, owner,
                      tick_lower, tick_upper, liquidity, amount0, amount1)
                     VALUES ('p',1,1,'tx',1,'rugpull','o',0,0,'0','0','0')",
                    [],
                )
            })
            .await;
        assert!(result.is_err(), "CHECK constraint should reject unknown kind");
    }
}
