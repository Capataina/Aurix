//! Strategy comparison grid results — one row per (range × rebalance ×
//! deposit × period) cell of M2.5's grid search.

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::error::StorageError;
use super::Storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyResultRow {
    pub grid_id: String,
    pub pool_address: String,
    pub range_width_pct: f64,
    pub rebalance_rule: String,
    pub deposit_usd: f64,
    pub period_days: i64,
    pub period_start_unix_ms: i64,
    pub period_end_unix_ms: i64,
    pub fees_usd: f64,
    pub il_usd: f64,
    pub lvr_usd: f64,
    pub mgmt_gas_usd: f64,
    pub net_return_usd: f64,
    pub net_return_vs_hold: f64,
    pub time_in_range_pct: f64,
    pub rebalance_count: i64,
    pub max_drawdown_pct: f64,
    pub sharpe: f64,
    pub sortino: f64,
    pub calmar: f64,
    pub deflated_sharpe: f64,
    pub completed_at_unix_ms: i64,
}

impl Storage {
    pub async fn insert_strategy_results_batch(
        &self,
        rows: Vec<StrategyResultRow>,
    ) -> Result<usize, StorageError> {
        let count = self
            .write(move |conn| {
                let tx = conn.transaction()?;
                let mut count = 0usize;
                {
                    let mut stmt = tx.prepare(
                        "INSERT INTO strategy_results
                         (grid_id, pool_address, range_width_pct, rebalance_rule,
                          deposit_usd, period_days, period_start_unix_ms, period_end_unix_ms,
                          fees_usd, il_usd, lvr_usd, mgmt_gas_usd,
                          net_return_usd, net_return_vs_hold, time_in_range_pct,
                          rebalance_count, max_drawdown_pct,
                          sharpe, sortino, calmar, deflated_sharpe, completed_at_unix_ms)
                         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,
                                 ?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)",
                    )?;
                    for r in rows.iter() {
                        let written = stmt.execute(params![
                            r.grid_id,
                            r.pool_address,
                            r.range_width_pct,
                            r.rebalance_rule,
                            r.deposit_usd,
                            r.period_days,
                            r.period_start_unix_ms,
                            r.period_end_unix_ms,
                            r.fees_usd,
                            r.il_usd,
                            r.lvr_usd,
                            r.mgmt_gas_usd,
                            r.net_return_usd,
                            r.net_return_vs_hold,
                            r.time_in_range_pct,
                            r.rebalance_count,
                            r.max_drawdown_pct,
                            r.sharpe,
                            r.sortino,
                            r.calmar,
                            r.deflated_sharpe,
                            r.completed_at_unix_ms,
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

    pub async fn query_strategy_results(
        &self,
        grid_id: String,
    ) -> Result<Vec<StrategyResultRow>, StorageError> {
        let pool = self.reader_pool.clone();
        let rows = tokio::task::spawn_blocking(move || -> Result<Vec<StrategyResultRow>, StorageError> {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(
                "SELECT grid_id, pool_address, range_width_pct, rebalance_rule,
                        deposit_usd, period_days, period_start_unix_ms, period_end_unix_ms,
                        fees_usd, il_usd, lvr_usd, mgmt_gas_usd,
                        net_return_usd, net_return_vs_hold, time_in_range_pct,
                        rebalance_count, max_drawdown_pct,
                        sharpe, sortino, calmar, deflated_sharpe, completed_at_unix_ms
                 FROM strategy_results
                 WHERE grid_id = ?1
                 ORDER BY sharpe DESC",
            )?;
            let iter = stmt.query_map(params![grid_id], |row| {
                Ok(StrategyResultRow {
                    grid_id: row.get(0)?,
                    pool_address: row.get(1)?,
                    range_width_pct: row.get(2)?,
                    rebalance_rule: row.get(3)?,
                    deposit_usd: row.get(4)?,
                    period_days: row.get(5)?,
                    period_start_unix_ms: row.get(6)?,
                    period_end_unix_ms: row.get(7)?,
                    fees_usd: row.get(8)?,
                    il_usd: row.get(9)?,
                    lvr_usd: row.get(10)?,
                    mgmt_gas_usd: row.get(11)?,
                    net_return_usd: row.get(12)?,
                    net_return_vs_hold: row.get(13)?,
                    time_in_range_pct: row.get(14)?,
                    rebalance_count: row.get(15)?,
                    max_drawdown_pct: row.get(16)?,
                    sharpe: row.get(17)?,
                    sortino: row.get(18)?,
                    calmar: row.get(19)?,
                    deflated_sharpe: row.get(20)?,
                    completed_at_unix_ms: row.get(21)?,
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
