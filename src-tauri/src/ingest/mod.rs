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
pub mod subgraph;

pub use alchemy::AlchemyArchiveSource;
pub use error::IngestError;
pub use mock::MockArchiveSource;
pub use pipeline::{AttemptedSource, IngestionReport, Ingester};
pub use source::{ArchiveSource, EthLog};
pub use subgraph::{PoolMetadata, UniswapV3SubgraphSource};

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

    // ─── Live smoke tests against Alchemy ────────────────────────────
    //
    // These hit the real chain via whatever key is in `.env`. Run with:
    //
    //   cargo test --lib ingest::tests::live_ -- --ignored --nocapture
    //
    // They are `#[ignore]` so the normal `cargo test` stays offline + free.

    #[tokio::test]
    #[ignore]
    async fn live_alchemy_finalized_block() {
        use crate::ingest::AlchemyArchiveSource;

        let source = AlchemyArchiveSource::from_environment()
            .expect("Alchemy key not configured — set ALCHEMY_API_KEY in .env");
        let block = source.latest_finalized_block().await.expect("RPC call");
        println!("✓ finalized block: {block}");
        assert!(
            block > 19_000_000,
            "finalized block looks unrealistically low: {block}"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn live_alchemy_ingest_10_block_range() {
        use crate::ingest::AlchemyArchiveSource;

        let source = AlchemyArchiveSource::from_environment()
            .expect("Alchemy key not configured")
            .with_payg_unbounded(false); // 10-block chunks on free tier

        let storage = Storage::open(DbLocation::in_memory()).await.unwrap();

        // V3 5bps WETH/USDC pool. 100-block window — wide enough to catch
        // at least one Mint/Burn/Collect event and exercise the full
        // decoder vocabulary.
        let pool = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640";
        let from_block = 19_000_000u64;
        let to_block = 19_000_099u64;

        let ingester = Ingester::new(storage.clone(), Arc::new(source));
        let report = ingester
            .backfill(pool, from_block, to_block)
            .await
            .expect("backfill");

        println!(
            "✓ live ingest: {} swaps + {} pool events + {} gas rows over blocks {}-{}",
            report.swap_rows_persisted,
            report.pool_event_rows_persisted,
            report.gas_rows_persisted,
            from_block,
            to_block,
        );

        let count = storage
            .count_swap_events(pool.to_string())
            .await
            .unwrap();
        assert!(count > 0, "no swaps persisted in live block range");

        let swaps = storage
            .query_swaps_for_pool_range(pool.to_string(), from_block as i64, to_block as i64)
            .await
            .unwrap();
        let first = &swaps[0];
        println!(
            "✓ first swap: blk={} log={} tx=0x{}… tick={} sqrt=0x{:x}… gas={:?}gwei",
            first.block_number,
            first.log_index,
            &first.transaction_hash[2..10],
            first.tick,
            num_bigint::BigUint::parse_bytes(first.sqrt_price_x96.as_bytes(), 10)
                .unwrap_or_default()
                .iter_u64_digits()
                .last()
                .unwrap_or(0),
            first.block_gas_price_gwei,
        );
        assert!(first.tick > -887_272 && first.tick < 887_272);
    }

    #[tokio::test]
    #[ignore]
    async fn live_alchemy_ingest_then_backtest() {
        use crate::backtest::{Engine, PositionConfig, RebalanceRule};
        use crate::ingest::AlchemyArchiveSource;

        let source = AlchemyArchiveSource::from_environment()
            .expect("Alchemy key not configured")
            .with_payg_unbounded(false);

        let storage = Storage::open(DbLocation::in_memory()).await.unwrap();

        let pool = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640";
        let from_block = 19_000_000u64;
        let to_block = 19_000_099u64;

        let ingester = Ingester::new(storage.clone(), Arc::new(source));
        ingester
            .backfill(pool, from_block, to_block)
            .await
            .expect("backfill");

        let swaps = storage
            .query_swaps_for_pool_range(pool.to_string(), from_block as i64, to_block as i64)
            .await
            .unwrap();
        assert!(!swaps.is_empty(), "no swaps to replay");
        let entry_tick = swaps[0].tick;

        // V3 5bps WETH/USDC pool: USDC (0xA0b8…) < WETH (0xC02a…) so
        // token0 = USDC (6 dec), token1 = WETH (18 dec).
        let config = PositionConfig {
            pool_address: pool.to_string(),
            tick_lower: entry_tick - 500,
            tick_upper: entry_tick + 500,
            deposit_token0: "3000000000".into(),     // 3000 USDC
            deposit_token1: "1000000000000000000".into(), // 1 WETH
            entry_block: from_block,
            exit_block: to_block,
            fee_tier_bps: 5,
            token0_decimals: 6,
            token1_decimals: 18,
            mev_haircut_bps: 0.0,
            token0_usd_price: None,
            token1_usd_price: None,

        };

        let engine = Engine::new(&storage);
        let out = engine
            .simulate(config, RebalanceRule::Static)
            .await
            .expect("simulate");

        println!(
            "✓ backtest on real chain data:\n\
             \tsamples: {}\n\
             \ttime in range: {:.1}%\n\
             \tfees:    ${:.6}\n\
             \tIL:      ${:.6}\n\
             \tLVR:     ${:.6}\n\
             \tmgmt gas:${:.6}\n\
             \tfinal:   ${:.4}\n\
             \thold:    ${:.4}\n\
             \tnet PnL: ${:.6}\n\
             \tSharpe:  {:.3}\n\
             \tSortino: {:.3}\n\
             \tmax DD:  {:.3}%",
            out.equity_curve.len(),
            out.summary.time_in_range_pct,
            out.summary.total_fees_usd,
            out.summary.total_il_usd,
            out.summary.total_lvr_usd,
            out.summary.total_mgmt_gas_usd,
            out.summary.final_value_usd,
            out.summary.hold_only_value_usd,
            out.summary.net_pnl_usd,
            out.summary.sharpe,
            out.summary.sortino,
            out.summary.max_drawdown_pct,
        );

        assert_eq!(out.equity_curve.len(), swaps.len());
        assert!(
            out.summary.total_mgmt_gas_usd > 0.0,
            "mgmt gas should have been paid"
        );
    }
}
