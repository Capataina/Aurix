//! Daily benchmark series cache. `series_key` distinguishes streams:
//! "aave_v3_usdc_supply_apy", "compound_v3_usdc_supply_apy",
//! "lido_steth_apy", "beaconchain_eth_store",
//! "v2lp_full_range_pnl_usd", "fred_dgs3mo", "fred_dgs1",
//! "yahoo_sp500tr_pct_return", "stooq_voo_close", "fred_gold_lbma".

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::error::StorageError;
use super::Storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkPoint {
    pub series_key: String,
    pub sample_date: String, // YYYY-MM-DD
    pub value: f64,
    pub source: String,
    pub fetched_at_unix_ms: i64,
}

impl Storage {
    pub async fn insert_benchmark_points_batch(
        &self,
        rows: Vec<BenchmarkPoint>,
    ) -> Result<usize, StorageError> {
        let count = self
            .write(move |conn| {
                let tx = conn.transaction()?;
                let mut count = 0usize;
                {
                    let mut stmt = tx.prepare(
                        "INSERT OR REPLACE INTO benchmark_series
                         (series_key, sample_date, value, source, fetched_at_unix_ms)
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                    )?;
                    for r in rows.iter() {
                        let n = stmt.execute(params![
                            r.series_key,
                            r.sample_date,
                            r.value,
                            r.source,
                            r.fetched_at_unix_ms,
                        ])?;
                        count += n;
                    }
                }
                tx.commit()?;
                Ok(count)
            })
            .await?;
        Ok(count)
    }

    pub async fn query_benchmark_range(
        &self,
        series_key: String,
        start_date: String,
        end_date: String,
    ) -> Result<Vec<BenchmarkPoint>, StorageError> {
        let pool = self.reader_pool.clone();
        let rows = tokio::task::spawn_blocking(move || -> Result<Vec<BenchmarkPoint>, StorageError> {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(
                "SELECT series_key, sample_date, value, source, fetched_at_unix_ms
                 FROM benchmark_series
                 WHERE series_key = ?1
                   AND sample_date BETWEEN ?2 AND ?3
                 ORDER BY sample_date ASC",
            )?;
            let iter = stmt.query_map(params![series_key, start_date, end_date], |row| {
                Ok(BenchmarkPoint {
                    series_key: row.get(0)?,
                    sample_date: row.get(1)?,
                    value: row.get(2)?,
                    source: row.get(3)?,
                    fetched_at_unix_ms: row.get(4)?,
                })
            })?;
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

    pub async fn list_benchmark_series_keys(&self) -> Result<Vec<String>, StorageError> {
        let pool = self.reader_pool.clone();
        let rows = tokio::task::spawn_blocking(move || -> Result<Vec<String>, StorageError> {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(
                "SELECT DISTINCT series_key FROM benchmark_series ORDER BY series_key ASC",
            )?;
            let iter = stmt.query_map([], |row| row.get::<_, String>(0))?;
            let mut keys = Vec::new();
            for r in iter {
                keys.push(r?);
            }
            Ok(keys)
        })
        .await
        .map_err(|e| StorageError::WriterUnavailable(e.to_string()))??;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::DbLocation;

    fn pt(key: &str, date: &str, v: f64) -> BenchmarkPoint {
        BenchmarkPoint {
            series_key: key.into(),
            sample_date: date.into(),
            value: v,
            source: "test".into(),
            fetched_at_unix_ms: 1_700_000_000_000,
        }
    }

    #[tokio::test]
    async fn batch_insert_and_range_query() {
        let s = Storage::open(DbLocation::in_memory()).await.unwrap();
        s.insert_benchmark_points_batch(vec![
            pt("aave_v3_usdc_supply_apy", "2026-01-01", 4.5),
            pt("aave_v3_usdc_supply_apy", "2026-01-02", 4.6),
            pt("aave_v3_usdc_supply_apy", "2026-02-01", 4.4),
            pt("lido_steth_apy", "2026-01-01", 3.2),
        ])
        .await
        .unwrap();

        let aave = s
            .query_benchmark_range(
                "aave_v3_usdc_supply_apy".into(),
                "2026-01-01".into(),
                "2026-01-31".into(),
            )
            .await
            .unwrap();
        assert_eq!(aave.len(), 2);

        let keys = s.list_benchmark_series_keys().await.unwrap();
        assert_eq!(keys, vec!["aave_v3_usdc_supply_apy", "lido_steth_apy"]);
    }

    #[tokio::test]
    async fn replace_on_duplicate_key() {
        let s = Storage::open(DbLocation::in_memory()).await.unwrap();
        s.insert_benchmark_points_batch(vec![pt("k", "2026-01-01", 1.0)])
            .await
            .unwrap();
        s.insert_benchmark_points_batch(vec![pt("k", "2026-01-01", 2.0)])
            .await
            .unwrap();
        let v = s
            .query_benchmark_range("k".into(), "2026-01-01".into(), "2026-01-01".into())
            .await
            .unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].value, 2.0, "INSERT OR REPLACE must overwrite");
    }
}
