//! Position simulation runs + equity curve points.
//!
//! `position_runs` is keyed by `config_hash` — a deterministic hash of the
//! input config (pool, ticks, deposit, blocks, rule, MEV haircut). Re-running
//! the same config returns the cached result (idempotency contract).
//!
//! `equity_curve_points` is the per-sample timeseries; cascade-deletes when
//! the parent run is removed.

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::error::StorageError;
use super::Storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionRunSummary {
    pub config_hash: String,
    pub pool_address: String,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub deposit_token0: String,
    pub deposit_token1: String,
    pub entry_block: i64,
    pub exit_block: i64,
    pub rebalance_rule: String, // serialised
    pub mev_haircut_bps: f64,
    pub total_fees_usd: f64,
    pub total_il_usd: f64,
    pub total_lvr_usd: f64,
    pub total_mgmt_gas_usd: f64,
    pub final_value_usd: f64,
    pub hold_only_value_usd: f64,
    pub net_pnl_usd: f64,
    pub time_in_range_pct: f64,
    pub rebalance_count: i64,
    pub max_drawdown_pct: f64,
    pub sharpe: f64,
    pub sortino: f64,
    pub calmar: f64,
    pub completed_at_unix_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquityCurvePoint {
    pub sample_idx: i64,
    pub block_number: i64,
    pub block_timestamp: i64,
    pub position_value_usd: f64,
    pub fees_accumulated_usd: f64,
    pub il_usd: f64,
    pub lvr_usd: f64,
    pub mgmt_gas_paid_usd: f64,
    pub hold_only_value_usd: f64,
    pub net_pnl_usd: f64,
    pub in_range: bool,
}

impl Storage {
    /// Persists a run summary plus its equity curve in one transaction.
    /// Returns the inserted run's `id` (or the existing id if the
    /// `config_hash` was already present and the call therefore skipped).
    pub async fn persist_position_run(
        &self,
        summary: PositionRunSummary,
        curve: Vec<EquityCurvePoint>,
    ) -> Result<i64, StorageError> {
        let result = self
            .write(move |conn| {
                let tx = conn.transaction()?;

                // First check if this config_hash exists already.
                let existing: Option<i64> = tx
                    .query_row(
                        "SELECT id FROM position_runs WHERE config_hash = ?1",
                        params![summary.config_hash],
                        |row| row.get(0),
                    )
                    .optional()?;

                let run_id = if let Some(id) = existing {
                    id
                } else {
                    tx.execute(
                        "INSERT INTO position_runs
                         (config_hash, pool_address, tick_lower, tick_upper,
                          deposit_token0, deposit_token1, entry_block, exit_block,
                          rebalance_rule, mev_haircut_bps,
                          total_fees_usd, total_il_usd, total_lvr_usd, total_mgmt_gas_usd,
                          final_value_usd, hold_only_value_usd, net_pnl_usd,
                          time_in_range_pct, rebalance_count, max_drawdown_pct,
                          sharpe, sortino, calmar, completed_at_unix_ms)
                         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,
                                 ?15,?16,?17,?18,?19,?20,?21,?22,?23,?24)",
                        params![
                            summary.config_hash,
                            summary.pool_address,
                            summary.tick_lower,
                            summary.tick_upper,
                            summary.deposit_token0,
                            summary.deposit_token1,
                            summary.entry_block,
                            summary.exit_block,
                            summary.rebalance_rule,
                            summary.mev_haircut_bps,
                            summary.total_fees_usd,
                            summary.total_il_usd,
                            summary.total_lvr_usd,
                            summary.total_mgmt_gas_usd,
                            summary.final_value_usd,
                            summary.hold_only_value_usd,
                            summary.net_pnl_usd,
                            summary.time_in_range_pct,
                            summary.rebalance_count,
                            summary.max_drawdown_pct,
                            summary.sharpe,
                            summary.sortino,
                            summary.calmar,
                            summary.completed_at_unix_ms,
                        ],
                    )?;
                    let id = tx.last_insert_rowid();

                    {
                        let mut stmt = tx.prepare(
                            "INSERT INTO equity_curve_points
                             (run_id, sample_idx, block_number, block_timestamp,
                              position_value_usd, fees_accumulated_usd, il_usd, lvr_usd,
                              mgmt_gas_paid_usd, hold_only_value_usd, net_pnl_usd, in_range)
                             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                        )?;
                        for p in curve.iter() {
                            stmt.execute(params![
                                id,
                                p.sample_idx,
                                p.block_number,
                                p.block_timestamp,
                                p.position_value_usd,
                                p.fees_accumulated_usd,
                                p.il_usd,
                                p.lvr_usd,
                                p.mgmt_gas_paid_usd,
                                p.hold_only_value_usd,
                                p.net_pnl_usd,
                                if p.in_range { 1i64 } else { 0i64 },
                            ])?;
                        }
                    }
                    id
                };

                tx.commit()?;
                Ok(run_id)
            })
            .await?;
        Ok(result)
    }

    pub async fn get_position_run(
        &self,
        config_hash: String,
    ) -> Result<Option<PositionRunSummary>, StorageError> {
        let pool = self.reader_pool.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<Option<PositionRunSummary>, StorageError> {
            let conn = pool.get()?;
            let row = conn
                .query_row(
                    "SELECT config_hash, pool_address, tick_lower, tick_upper,
                            deposit_token0, deposit_token1, entry_block, exit_block,
                            rebalance_rule, mev_haircut_bps,
                            total_fees_usd, total_il_usd, total_lvr_usd, total_mgmt_gas_usd,
                            final_value_usd, hold_only_value_usd, net_pnl_usd,
                            time_in_range_pct, rebalance_count, max_drawdown_pct,
                            sharpe, sortino, calmar, completed_at_unix_ms
                     FROM position_runs WHERE config_hash = ?1",
                    params![config_hash],
                    |row| {
                        Ok(PositionRunSummary {
                            config_hash: row.get(0)?,
                            pool_address: row.get(1)?,
                            tick_lower: row.get::<_, i64>(2)? as i32,
                            tick_upper: row.get::<_, i64>(3)? as i32,
                            deposit_token0: row.get(4)?,
                            deposit_token1: row.get(5)?,
                            entry_block: row.get(6)?,
                            exit_block: row.get(7)?,
                            rebalance_rule: row.get(8)?,
                            mev_haircut_bps: row.get(9)?,
                            total_fees_usd: row.get(10)?,
                            total_il_usd: row.get(11)?,
                            total_lvr_usd: row.get(12)?,
                            total_mgmt_gas_usd: row.get(13)?,
                            final_value_usd: row.get(14)?,
                            hold_only_value_usd: row.get(15)?,
                            net_pnl_usd: row.get(16)?,
                            time_in_range_pct: row.get(17)?,
                            rebalance_count: row.get(18)?,
                            max_drawdown_pct: row.get(19)?,
                            sharpe: row.get(20)?,
                            sortino: row.get(21)?,
                            calmar: row.get(22)?,
                            completed_at_unix_ms: row.get(23)?,
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

    pub async fn get_equity_curve(
        &self,
        config_hash: String,
    ) -> Result<Vec<EquityCurvePoint>, StorageError> {
        let pool = self.reader_pool.clone();
        let rows = tokio::task::spawn_blocking(move || -> Result<Vec<EquityCurvePoint>, StorageError> {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(
                "SELECT sample_idx, block_number, block_timestamp,
                        position_value_usd, fees_accumulated_usd, il_usd, lvr_usd,
                        mgmt_gas_paid_usd, hold_only_value_usd, net_pnl_usd, in_range
                 FROM equity_curve_points
                 WHERE run_id = (SELECT id FROM position_runs WHERE config_hash = ?1)
                 ORDER BY sample_idx ASC",
            )?;
            let iter = stmt.query_map(params![config_hash], |row| {
                Ok(EquityCurvePoint {
                    sample_idx: row.get(0)?,
                    block_number: row.get(1)?,
                    block_timestamp: row.get(2)?,
                    position_value_usd: row.get(3)?,
                    fees_accumulated_usd: row.get(4)?,
                    il_usd: row.get(5)?,
                    lvr_usd: row.get(6)?,
                    mgmt_gas_paid_usd: row.get(7)?,
                    hold_only_value_usd: row.get(8)?,
                    net_pnl_usd: row.get(9)?,
                    in_range: row.get::<_, i64>(10)? == 1,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::DbLocation;

    fn sample_summary(hash: &str) -> PositionRunSummary {
        PositionRunSummary {
            config_hash: hash.into(),
            pool_address: "0xpool".into(),
            tick_lower: 200000,
            tick_upper: 201000,
            deposit_token0: "1000000000000000000".into(),
            deposit_token1: "3000000000".into(),
            entry_block: 100,
            exit_block: 200,
            rebalance_rule: r#"{"kind":"static"}"#.into(),
            mev_haircut_bps: 5.0,
            total_fees_usd: 12.34,
            total_il_usd: 8.0,
            total_lvr_usd: 4.0,
            total_mgmt_gas_usd: 2.0,
            final_value_usd: 1010.0,
            hold_only_value_usd: 1005.0,
            net_pnl_usd: 5.0,
            time_in_range_pct: 80.0,
            rebalance_count: 0,
            max_drawdown_pct: 1.0,
            sharpe: 0.5,
            sortino: 0.7,
            calmar: 0.8,
            completed_at_unix_ms: 1_700_000_000_000,
        }
    }

    fn sample_curve(n: usize) -> Vec<EquityCurvePoint> {
        (0..n)
            .map(|i| EquityCurvePoint {
                sample_idx: i as i64,
                block_number: 100 + i as i64,
                block_timestamp: 1_700_000_000 + (i as i64) * 12,
                position_value_usd: 1000.0 + i as f64,
                fees_accumulated_usd: i as f64 * 0.1,
                il_usd: 0.0,
                lvr_usd: 0.0,
                mgmt_gas_paid_usd: 0.0,
                hold_only_value_usd: 1000.0,
                net_pnl_usd: i as f64,
                in_range: i % 2 == 0,
            })
            .collect()
    }

    #[tokio::test]
    async fn persist_and_read_round_trip() {
        let s = Storage::open(DbLocation::in_memory()).await.unwrap();
        s.persist_position_run(sample_summary("h1"), sample_curve(10))
            .await
            .unwrap();

        let summary = s.get_position_run("h1".into()).await.unwrap().unwrap();
        assert_eq!(summary.entry_block, 100);

        let curve = s.get_equity_curve("h1".into()).await.unwrap();
        assert_eq!(curve.len(), 10);
        assert!(curve[0].in_range);
        assert!(!curve[1].in_range);
    }

    #[tokio::test]
    async fn re_persisting_same_hash_is_idempotent() {
        let s = Storage::open(DbLocation::in_memory()).await.unwrap();
        let id1 = s
            .persist_position_run(sample_summary("h1"), sample_curve(5))
            .await
            .unwrap();
        let id2 = s
            .persist_position_run(sample_summary("h1"), sample_curve(5))
            .await
            .unwrap();
        assert_eq!(id1, id2, "same config_hash must reuse the same run id");

        let curve = s.get_equity_curve("h1".into()).await.unwrap();
        assert_eq!(curve.len(), 5, "no curve duplication on idempotent re-persist");
    }
}
