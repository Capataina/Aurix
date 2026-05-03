//! Archive log ingestion (M2.1).
//!
//! Read-only pipeline that pulls V3 Swap / Mint / Burn / Collect events
//! from an `ArchiveSource` (Alchemy live, or fixture mock for tests),
//! decodes them, and persists them idempotently into `storage`.
//!
//! Reference: `context/plans/vector-a-v3-lp-backtester.md` §M2.1 +
//! `context/references/ethereum-archive-log-ingestion.md`.

// Ingestion subsystem (M2.1). Public surface is the `Ingester` (high-
// level orchestrator) plus the trait-level abstractions; concrete
// adapters are wired by the IPC layer.
#![allow(dead_code)]

pub mod alchemy;
pub mod decoder;
pub mod error;
pub mod mock;
pub mod pipeline;
pub mod source;

pub use alchemy::AlchemyArchiveSource;
pub use error::IngestError;
pub use mock::MockArchiveSource;
pub use pipeline::{IngestionReport, Ingester};
pub use source::{ArchiveSource, EthLog};

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use num_bigint::BigUint;

    use crate::storage::pool_events::PoolEventKind;
    use crate::storage::{DbLocation, Storage};

    use super::decoder::{BURN_TOPIC0, MINT_TOPIC0, SWAP_TOPIC0};
    use super::source::EthLog;
    use super::*;

    fn encode_uint_word(value: &BigUint) -> String {
        format!("{:0>64}", value.to_str_radix(16))
    }

    fn encode_int24_word(value: i32) -> String {
        let raw = (value as u32) & 0x00FF_FFFF;
        if value < 0 {
            format!("{:0>58}{:06x}", "f".repeat(58), raw)
        } else {
            format!("{:0>64x}", raw)
        }
    }

    fn encode_int256_word(value: i64) -> String {
        if value >= 0 {
            format!("{:0>64x}", value as u64)
        } else {
            let two_to_256: BigUint = BigUint::from(1u8) << 256;
            let big: BigUint = two_to_256 - BigUint::from((-value) as u64);
            format!("{:0>64}", big.to_str_radix(16))
        }
    }

    fn make_swap_log(pool: &str, block: u64, log_idx: u64, tick: i32) -> EthLog {
        let topic0 = format!("0x{SWAP_TOPIC0}");
        let topic1 = format!("0x{:0>64}", "1");
        let topic2 = format!("0x{:0>64}", "2");
        let amount0 = encode_int256_word(1_000_000_000_000_000_000i64);
        let amount1 = encode_int256_word(-3_000_000_000i64);
        let sqrt = encode_uint_word(&BigUint::from(1_000_000u64));
        let liq = encode_uint_word(&BigUint::from(1_000_000_000u64));
        let tick_word = encode_int24_word(tick);
        let data = format!("0x{amount0}{amount1}{sqrt}{liq}{tick_word}");
        EthLog {
            address: pool.to_string(),
            block_number: block,
            log_index: log_idx,
            transaction_hash: format!("0x{:0>64}", "deadbeef"),
            block_timestamp: 1_700_000_000 + (block as i64) * 12,
            topics: vec![topic0, topic1, topic2],
            data,
        }
    }

    fn make_mint_log(pool: &str, block: u64, log_idx: u64, owner: &str) -> EthLog {
        let topic0 = format!("0x{MINT_TOPIC0}");
        let topic1 = format!("0x{:0>64}", owner);
        let topic2 = format!("0x{}", encode_int24_word(-100));
        let topic3 = format!("0x{}", encode_int24_word(100));
        let sender_word = format!("{:0>64}", "0");
        let amount = format!("{:0>64x}", 1_000_000u64);
        let amount0 = format!("{:0>64x}", 1_000_000_000_000_000_000u64);
        let amount1 = format!("{:0>64x}", 3_000_000_000u64);
        let data = format!("0x{sender_word}{amount}{amount0}{amount1}");
        EthLog {
            address: pool.to_string(),
            block_number: block,
            log_index: log_idx,
            transaction_hash: format!("0x{:0>64}", "feed"),
            block_timestamp: 1_700_000_000 + (block as i64) * 12,
            topics: vec![topic0, topic1, topic2, topic3],
            data,
        }
    }

    fn make_burn_log(pool: &str, block: u64, log_idx: u64, owner: &str) -> EthLog {
        let topic0 = format!("0x{BURN_TOPIC0}");
        let topic1 = format!("0x{:0>64}", owner);
        let topic2 = format!("0x{}", encode_int24_word(-100));
        let topic3 = format!("0x{}", encode_int24_word(100));
        let amount = format!("{:0>64x}", 1_000_000u64);
        let amount0 = format!("{:0>64x}", 5_000_000_000_000_000u64);
        let amount1 = format!("{:0>64x}", 15_000_000u64);
        let data = format!("0x{amount}{amount0}{amount1}");
        EthLog {
            address: pool.to_string(),
            block_number: block,
            log_index: log_idx,
            transaction_hash: format!("0x{:0>64}", "babe"),
            block_timestamp: 1_700_000_000 + (block as i64) * 12,
            topics: vec![topic0, topic1, topic2, topic3],
            data,
        }
    }

    #[tokio::test]
    async fn pipeline_ingests_swaps_and_pool_events() {
        let storage = Storage::open(DbLocation::in_memory()).await.unwrap();
        let mock = Arc::new(MockArchiveSource::new(1_000));
        for b in 100..105 {
            mock.add_log(make_swap_log("0xpool", b, 0, 200_000));
        }
        mock.add_log(make_mint_log("0xpool", 100, 1, "0xa"));
        mock.add_log(make_burn_log("0xpool", 104, 2, "0xa"));
        let ingester = Ingester::new(storage.clone(), mock);
        let report = ingester.backfill("0xpool", 100, 110).await.unwrap();
        assert_eq!(report.swap_rows_persisted, 5);
        assert_eq!(report.pool_event_rows_persisted, 2);

        let swaps = storage
            .query_swaps_for_pool_range("0xpool".into(), 100, 110)
            .await
            .unwrap();
        assert_eq!(swaps.len(), 5);
        assert_eq!(swaps[0].block_number, 100);
        assert_eq!(swaps[0].tick, 200_000);

        let mints = storage
            .query_pool_events_for_range("0xpool".into(), 100, 110, Some(PoolEventKind::Mint))
            .await
            .unwrap();
        assert_eq!(mints.len(), 1);
    }

    #[tokio::test]
    async fn pipeline_idempotent_on_re_run() {
        let storage = Storage::open(DbLocation::in_memory()).await.unwrap();
        let mock = Arc::new(MockArchiveSource::new(1_000));
        for b in 0..3 {
            mock.add_log(make_swap_log("0xpool", b, 0, 0));
        }
        let ingester = Ingester::new(storage.clone(), mock);
        let r1 = ingester.backfill("0xpool", 0, 5).await.unwrap();
        assert_eq!(r1.swap_rows_persisted, 3);
        let r2 = ingester.backfill("0xpool", 0, 5).await.unwrap();
        assert_eq!(r2.swap_rows_persisted, 0, "re-run must not duplicate");
        assert_eq!(storage.count_swap_events("0xpool".into()).await.unwrap(), 3);
    }

    #[tokio::test]
    async fn pipeline_advances_checkpoint() {
        let storage = Storage::open(DbLocation::in_memory()).await.unwrap();
        let mock = Arc::new(MockArchiveSource::new(1_000));
        let ingester = Ingester::new(storage.clone(), mock);

        assert_eq!(
            ingester
                .next_block_to_ingest("0xpool", 100)
                .await
                .unwrap(),
            100,
            "no checkpoint -> default"
        );

        ingester.backfill("0xpool", 100, 200).await.unwrap();
        let next = ingester.next_block_to_ingest("0xpool", 0).await.unwrap();
        assert_eq!(next, 201, "should resume one past last block");
    }

    #[tokio::test]
    async fn pipeline_rejects_inverted_range() {
        let storage = Storage::open(DbLocation::in_memory()).await.unwrap();
        let mock = Arc::new(MockArchiveSource::new(1_000));
        let ingester = Ingester::new(storage, mock);
        let r = ingester.backfill("0xpool", 200, 100).await;
        assert!(matches!(r, Err(IngestError::InvalidRange { .. })));
    }

    #[tokio::test]
    async fn pipeline_records_per_block_gas() {
        let storage = Storage::open(DbLocation::in_memory()).await.unwrap();
        let mock = Arc::new(MockArchiveSource::new(1_000));
        mock.set_block_gas(50, 25.5);
        mock.add_log(make_swap_log("0xpool", 50, 0, 0));
        let ingester = Ingester::new(storage.clone(), mock);
        ingester.backfill("0xpool", 50, 50).await.unwrap();
        let gas = storage.block_gas_at(50).await.unwrap().unwrap();
        assert_eq!(gas.median_gas_gwei, 25.5);
    }

    #[tokio::test]
    async fn pipeline_safe_to_block_uses_finalized() {
        let storage = Storage::open(DbLocation::in_memory()).await.unwrap();
        let mock = Arc::new(MockArchiveSource::new(123_456));
        let ingester = Ingester::new(storage, mock);
        assert_eq!(ingester.safe_to_block().await.unwrap(), 123_456);
    }
}
