//! In-memory fixture-backed archive source. Used by the test suite and
//! by the validation harness when no Alchemy key is configured.

use std::collections::BTreeMap;
use std::sync::Mutex;

use async_trait::async_trait;

use super::error::IngestError;
use super::source::{ArchiveSource, EthLog};

/// `MockArchiveSource` holds a per-block log fixture set. Tests build the
/// fixture upfront via the `add_*` helpers and then run the ingestion
/// pipeline against it as if it were a real RPC. Concurrency-safe via a
/// single `Mutex` — fixture sets are tiny.
pub struct MockArchiveSource {
    inner: Mutex<MockState>,
}

struct MockState {
    logs: BTreeMap<u64, Vec<EthLog>>,
    block_gas_gwei: BTreeMap<u64, f64>,
    block_timestamps: BTreeMap<u64, i64>,
    finalized_block: u64,
}

impl MockArchiveSource {
    pub fn new(finalized_block: u64) -> Self {
        Self {
            inner: Mutex::new(MockState {
                logs: BTreeMap::new(),
                block_gas_gwei: BTreeMap::new(),
                block_timestamps: BTreeMap::new(),
                finalized_block,
            }),
        }
    }

    pub fn add_log(&self, log: EthLog) {
        let mut s = self.inner.lock().unwrap();
        s.block_timestamps
            .entry(log.block_number)
            .or_insert(log.block_timestamp);
        s.logs
            .entry(log.block_number)
            .or_default()
            .push(log);
    }

    pub fn set_block_gas(&self, block: u64, gwei: f64) {
        let mut s = self.inner.lock().unwrap();
        s.block_gas_gwei.insert(block, gwei);
    }

    pub fn set_block_timestamp(&self, block: u64, ts: i64) {
        let mut s = self.inner.lock().unwrap();
        s.block_timestamps.insert(block, ts);
    }

    pub fn advance_finalized_to(&self, block: u64) {
        let mut s = self.inner.lock().unwrap();
        s.finalized_block = block;
    }
}

#[async_trait]
impl ArchiveSource for MockArchiveSource {
    async fn get_pool_logs(
        &self,
        pool_address: &str,
        from_block: u64,
        to_block: u64,
        topic0_filters: &[&str],
    ) -> Result<Vec<EthLog>, IngestError> {
        if from_block > to_block {
            return Err(IngestError::InvalidRange {
                from: from_block,
                to: to_block,
            });
        }
        let s = self.inner.lock().unwrap();
        let pool_lower = pool_address.to_lowercase();
        let mut out = Vec::new();
        for (&_b, logs) in s.logs.range(from_block..=to_block) {
            for log in logs {
                if log.address.to_lowercase() != pool_lower {
                    continue;
                }
                if !topic0_filters.is_empty() {
                    let log_t0 = log
                        .topics
                        .first()
                        .map(|t| t.trim_start_matches("0x").to_lowercase())
                        .unwrap_or_default();
                    let matches = topic0_filters
                        .iter()
                        .any(|f| f.trim_start_matches("0x").to_lowercase() == log_t0);
                    if !matches {
                        continue;
                    }
                }
                out.push(log.clone());
            }
        }
        Ok(out)
    }

    async fn latest_finalized_block(&self) -> Result<u64, IngestError> {
        Ok(self.inner.lock().unwrap().finalized_block)
    }

    async fn block_median_gas_gwei(&self, block: u64) -> Result<f64, IngestError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .block_gas_gwei
            .get(&block)
            .copied()
            .unwrap_or(20.0))
    }

    async fn block_timestamp(&self, block: u64) -> Result<i64, IngestError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .block_timestamps
            .get(&block)
            .copied()
            .unwrap_or(1_700_000_000 + (block as i64) * 12))
    }
}
