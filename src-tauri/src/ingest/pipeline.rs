//! Ingestion orchestrator. Pulls a block range through the source,
//! decodes per-event-type, persists in idempotent batches, advances the
//! ingestion checkpoint.

use std::sync::Arc;

use crate::storage::gas::BlockGasRow;
use crate::storage::pool_events::PoolEventRow;
use crate::storage::state::IngestionState;
use crate::storage::swaps::SwapEventRow;
use crate::storage::Storage;

use super::decoder::{
    decode_burn, decode_collect, decode_mint, decode_swap, BURN_TOPIC0, COLLECT_TOPIC0,
    MINT_TOPIC0, SWAP_TOPIC0,
};
use super::error::IngestError;
use super::source::ArchiveSource;

/// Per-call ingestion result — the counts surface on the IPC boundary so
/// the GUI can show an "ingested N swaps in M seconds" toast.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestionReport {
    pub pool_address: String,
    pub from_block: u64,
    pub to_block: u64,
    pub swap_rows_persisted: usize,
    pub pool_event_rows_persisted: usize,
    pub gas_rows_persisted: usize,
}

pub struct Ingester {
    pub storage: Storage,
    pub source: Arc<dyn ArchiveSource>,
}

impl Ingester {
    pub fn new(storage: Storage, source: Arc<dyn ArchiveSource>) -> Self {
        Self { storage, source }
    }

    /// Backfills `[from_block, to_block]` for `pool_address`. The block
    /// range is sliced into 10k-block chunks (Alchemy PAYG-friendly; the
    /// underlying source enforces its own per-call cap).
    ///
    /// Idempotency: re-running the same range writes zero new rows
    /// because of the storage-level `INSERT OR IGNORE` contracts.
    pub async fn backfill(
        &self,
        pool_address: &str,
        from_block: u64,
        to_block: u64,
    ) -> Result<IngestionReport, IngestError> {
        if from_block > to_block {
            return Err(IngestError::InvalidRange {
                from: from_block,
                to: to_block,
            });
        }

        let mut total_swaps = 0usize;
        let mut total_pool_events = 0usize;
        let mut total_gas = 0usize;

        // The per-source cap enforces sub-chunking; we use 10k-block
        // outer chunks here for memory budgeting on big backfills.
        const OUTER_CHUNK: u64 = 10_000;
        let mut window_start = from_block;
        while window_start <= to_block {
            let window_end = (window_start + OUTER_CHUNK - 1).min(to_block);

            // Pull all four event types in one source call to amortise
            // RPC round-trips when the source is a real RPC.
            let topic0_filters: &[&str] =
                &[SWAP_TOPIC0, MINT_TOPIC0, BURN_TOPIC0, COLLECT_TOPIC0];
            let logs = self
                .source
                .get_pool_logs(pool_address, window_start, window_end, topic0_filters)
                .await?;

            let mut swap_rows: Vec<SwapEventRow> = Vec::new();
            let mut pool_rows: Vec<PoolEventRow> = Vec::new();
            let mut gas_rows: Vec<BlockGasRow> = Vec::new();
            let mut last_gas_block: Option<u64> = None;

            for log in logs {
                let topic0 = log
                    .topics
                    .first()
                    .map(|t| t.trim_start_matches("0x").to_lowercase())
                    .unwrap_or_default();

                match topic0.as_str() {
                    t if t == SWAP_TOPIC0 => {
                        // Capture the per-block gas price once per block.
                        let gas_gwei = if last_gas_block != Some(log.block_number) {
                            let g = self.source.block_median_gas_gwei(log.block_number).await.ok();
                            if let Some(gwei) = g {
                                gas_rows.push(BlockGasRow {
                                    block_number: log.block_number as i64,
                                    block_timestamp: log.block_timestamp,
                                    base_fee_gwei: Some(gwei),
                                    median_gas_gwei: gwei,
                                });
                            }
                            last_gas_block = Some(log.block_number);
                            g
                        } else {
                            None
                        };
                        let row = decode_swap(&log, gas_gwei)?;
                        swap_rows.push(row);
                    }
                    t if t == MINT_TOPIC0 => pool_rows.push(decode_mint(&log)?),
                    t if t == BURN_TOPIC0 => pool_rows.push(decode_burn(&log)?),
                    t if t == COLLECT_TOPIC0 => pool_rows.push(decode_collect(&log)?),
                    _ => continue, // unknown topic — skip silently
                }
            }

            let swap_persisted = if !swap_rows.is_empty() {
                self.storage.insert_swap_events_batch(swap_rows).await?
            } else {
                0
            };
            total_swaps += swap_persisted;

            let pool_persisted = if !pool_rows.is_empty() {
                self.storage.insert_pool_events_batch(pool_rows).await?
            } else {
                0
            };
            total_pool_events += pool_persisted;

            let gas_persisted = if !gas_rows.is_empty() {
                self.storage.upsert_block_gas_batch(gas_rows).await?
            } else {
                0
            };
            total_gas += gas_persisted;

            // Advance the checkpoint after every chunk so resume on
            // restart is granular.
            let now_ms = chrono::Utc::now().timestamp_millis();
            self.storage
                .upsert_ingestion_state(IngestionState {
                    pool_address: pool_address.to_string(),
                    last_swap_block: window_end as i64,
                    last_pool_event_block: window_end as i64,
                    last_run_at_unix_ms: now_ms,
                })
                .await?;

            window_start = window_end + 1;
        }

        Ok(IngestionReport {
            pool_address: pool_address.to_string(),
            from_block,
            to_block,
            swap_rows_persisted: total_swaps,
            pool_event_rows_persisted: total_pool_events,
            gas_rows_persisted: total_gas,
        })
    }

    /// Resume helper: if a checkpoint exists for `pool_address`, returns
    /// the next block to ingest from. Otherwise returns `default_from`.
    pub async fn next_block_to_ingest(
        &self,
        pool_address: &str,
        default_from: u64,
    ) -> Result<u64, IngestError> {
        let state = self
            .storage
            .ingestion_state(pool_address.to_string())
            .await?;
        Ok(match state {
            Some(s) => (s.last_swap_block as u64).saturating_add(1).max(default_from),
            None => default_from,
        })
    }

    /// Resolve the safe upper bound to ingest up to: the chain's
    /// `finalized` block. Per `context/references/ethereum-archive-log-
    /// ingestion.md`, finality is hard at ~64 blocks post-Merge;
    /// ingesting up to "finalized" eliminates rollback complexity that a
    /// "depth >= N" model carries.
    pub async fn safe_to_block(&self) -> Result<u64, IngestError> {
        self.source.latest_finalized_block().await
    }
}
