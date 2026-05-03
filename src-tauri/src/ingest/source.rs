//! Archive RPC abstraction.
//!
//! Concrete sources (Alchemy, mock fixtures) implement this trait. The
//! ingestion pipeline reads logs through the trait so the live archive
//! and the test fixture set are interchangeable.

use async_trait::async_trait;

use super::error::IngestError;

/// Raw `eth_getLogs` row, before ABI decoding. Stays intentionally close
/// to the JSON-RPC shape so an Alchemy adapter is mostly a hex
/// pass-through.
#[derive(Debug, Clone)]
pub struct EthLog {
    pub address: String,        // 20-byte hex with 0x prefix
    pub block_number: u64,
    pub log_index: u64,
    pub transaction_hash: String,
    pub block_timestamp: i64,   // unix-seconds
    pub topics: Vec<String>,    // each 32-byte hex with 0x prefix
    pub data: String,           // 0x + N*64 hex chars
}

/// A bounded archive RPC source. Implementations may be backed by a live
/// JSON-RPC endpoint (Alchemy / Infura archive) or by an in-memory fixture
/// store (tests). The `topic0_filters` argument selects which event types
/// to fetch within the range.
#[async_trait]
pub trait ArchiveSource: Send + Sync {
    /// Returns logs for `pool_address` in `[from_block, to_block]` with
    /// any of the supplied topic0 filters. The caller is responsible for
    /// chunking large ranges if the source has a per-call cap (the
    /// 10-block cap on Alchemy free tier is enforced by the Alchemy
    /// adapter, not by callers).
    async fn get_pool_logs(
        &self,
        pool_address: &str,
        from_block: u64,
        to_block: u64,
        topic0_filters: &[&str],
    ) -> Result<Vec<EthLog>, IngestError>;

    /// Returns the chain head's `finalized` block number per
    /// `eth_getBlockByNumber("finalized")`. Post-Merge finality is hard
    /// at ~64 blocks; the backtester uses this as the safe upper bound to
    /// avoid reorg complexity (per
    /// `context/references/ethereum-archive-log-ingestion.md`).
    async fn latest_finalized_block(&self) -> Result<u64, IngestError>;

    /// Returns the per-block median gas price in gwei. Sparse — only
    /// invoked for blocks that the mgmt-gas pipeline persists.
    async fn block_median_gas_gwei(&self, block: u64) -> Result<f64, IngestError>;

    /// Returns the unix-second timestamp for a given block. Used when the
    /// log itself doesn't carry one (eth_getLogs returns block hash but
    /// not timestamp).
    async fn block_timestamp(&self, block: u64) -> Result<i64, IngestError>;
}
