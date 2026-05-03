//! M2.8 headline-run persistence.

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::error::StorageError;
use super::Storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadlineRunSummary {
    pub config_hash: String,
    pub pool_address: String,
    pub lookback_months: i64,
    pub regime_method: String,
    pub months_lp_beat_lending: i64,
    pub months_total: i64,
    pub median_low_vol_spread: Option<f64>,
    pub median_med_vol_spread: Option<f64>,
    pub median_high_vol_spread: Option<f64>,
    pub verdict_text: String,
    pub completed_at_unix_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadlineMonthlyRow {
    pub year_month: String,
    pub vol_regime: String,
    pub best_lp_return: f64,
    pub naive_lp_return: f64,
    pub median_lp_return: f64,
    pub aave_usdc_return: f64,
    pub lido_steth_return: f64,
    pub hodl_return: f64,
    pub eth_vol_30d: f64,
}

impl Storage {
    pub async fn persist_headline_run(
        &self,
        summary: HeadlineRunSummary,
        monthly: Vec<HeadlineMonthlyRow>,
    ) -> Result<i64, StorageError> {
        let result = self
            .write(move |conn| {
                let tx = conn.transaction()?;

                let existing: Option<i64> = tx
                    .query_row(
                        "SELECT id FROM headline_runs WHERE config_hash = ?1",
                        params![summary.config_hash],
                        |row| row.get(0),
                    )
                    .optional()?;

                let id = if let Some(id) = existing {
                    id
                } else {
                    tx.execute(
                        "INSERT INTO headline_runs
                         (config_hash, pool_address, lookback_months, regime_method,
                          months_lp_beat_lending, months_total,
                          median_low_vol_spread, median_med_vol_spread, median_high_vol_spread,
                          verdict_text, completed_at_unix_ms)
                         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                        params![
                            summary.config_hash,
                            summary.pool_address,
                            summary.lookback_months,
                            summary.regime_method,
                            summary.months_lp_beat_lending,
                            summary.months_total,
                            summary.median_low_vol_spread,
                            summary.median_med_vol_spread,
                            summary.median_high_vol_spread,
                            summary.verdict_text,
                            summary.completed_at_unix_ms,
                        ],
                    )?;
                    let id = tx.last_insert_rowid();

                    {
                        let mut stmt = tx.prepare(
                            "INSERT INTO headline_monthly
                             (headline_run_id, year_month, vol_regime,
                              best_lp_return, naive_lp_return, median_lp_return,
                              aave_usdc_return, lido_steth_return, hodl_return, eth_vol_30d)
                             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                        )?;
                        for m in monthly.iter() {
                            stmt.execute(params![
                                id,
                                m.year_month,
                                m.vol_regime,
                                m.best_lp_return,
                                m.naive_lp_return,
                                m.median_lp_return,
                                m.aave_usdc_return,
                                m.lido_steth_return,
                                m.hodl_return,
                                m.eth_vol_30d,
                            ])?;
                        }
                    }
                    id
                };

                tx.commit()?;
                Ok(id)
            })
            .await?;
        Ok(result)
    }

    pub async fn get_headline_run(
        &self,
        config_hash: String,
    ) -> Result<Option<HeadlineRunSummary>, StorageError> {
        let pool = self.reader_pool.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<Option<HeadlineRunSummary>, StorageError> {
            let conn = pool.get()?;
            let row = conn
                .query_row(
                    "SELECT config_hash, pool_address, lookback_months, regime_method,
                            months_lp_beat_lending, months_total,
                            median_low_vol_spread, median_med_vol_spread, median_high_vol_spread,
                            verdict_text, completed_at_unix_ms
                     FROM headline_runs WHERE config_hash = ?1",
                    params![config_hash],
                    |row| {
                        Ok(HeadlineRunSummary {
                            config_hash: row.get(0)?,
                            pool_address: row.get(1)?,
                            lookback_months: row.get(2)?,
                            regime_method: row.get(3)?,
                            months_lp_beat_lending: row.get(4)?,
                            months_total: row.get(5)?,
                            median_low_vol_spread: row.get(6)?,
                            median_med_vol_spread: row.get(7)?,
                            median_high_vol_spread: row.get(8)?,
                            verdict_text: row.get(9)?,
                            completed_at_unix_ms: row.get(10)?,
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

    pub async fn get_headline_monthly(
        &self,
        config_hash: String,
    ) -> Result<Vec<HeadlineMonthlyRow>, StorageError> {
        let pool = self.reader_pool.clone();
        let rows = tokio::task::spawn_blocking(move || -> Result<Vec<HeadlineMonthlyRow>, StorageError> {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(
                "SELECT year_month, vol_regime,
                        best_lp_return, naive_lp_return, median_lp_return,
                        aave_usdc_return, lido_steth_return, hodl_return, eth_vol_30d
                 FROM headline_monthly
                 WHERE headline_run_id = (SELECT id FROM headline_runs WHERE config_hash = ?1)
                 ORDER BY year_month ASC",
            )?;
            let iter = stmt.query_map(params![config_hash], |row| {
                Ok(HeadlineMonthlyRow {
                    year_month: row.get(0)?,
                    vol_regime: row.get(1)?,
                    best_lp_return: row.get(2)?,
                    naive_lp_return: row.get(3)?,
                    median_lp_return: row.get(4)?,
                    aave_usdc_return: row.get(5)?,
                    lido_steth_return: row.get(6)?,
                    hodl_return: row.get(7)?,
                    eth_vol_30d: row.get(8)?,
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
}
