//! Position simulation engine (M2.3) and per-strategy metrics (M2.5
//! support).
//!
//! Reference: `context/plans/vector-a-v3-lp-backtester.md` §M2.3 + §M2.5.

#![allow(dead_code)]

pub mod engine;
pub mod error;
pub mod gas;
pub mod metrics;
pub mod position;
pub mod price;
pub mod rebalance;

pub use engine::{Engine, SimulationOutput};
pub use error::BacktestError;
pub use position::PositionConfig;
pub use rebalance::RebalanceRule;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use num_bigint::BigUint;

    use crate::ingest::{Ingester, MockArchiveSource};
    use crate::storage::{DbLocation, Storage};

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
    fn make_swap_log(
        pool: &str,
        block: u64,
        log_idx: u64,
        amount0: i64,
        amount1: i64,
        sqrt_price_x96: &BigUint,
        liquidity: u128,
        tick: i32,
    ) -> crate::ingest::EthLog {
        let topic0 = format!("0x{}", crate::ingest::decoder::SWAP_TOPIC0);
        let topic1 = format!("0x{:0>64}", "1");
        let topic2 = format!("0x{:0>64}", "2");
        let a0 = encode_int256_word(amount0);
        let a1 = encode_int256_word(amount1);
        let sq = encode_uint_word(sqrt_price_x96);
        let li = encode_uint_word(&BigUint::from(liquidity));
        let tk = encode_int24_word(tick);
        let data = format!("0x{a0}{a1}{sq}{li}{tk}");
        crate::ingest::EthLog {
            address: pool.to_string(),
            block_number: block,
            log_index: log_idx,
            transaction_hash: format!("0x{:0>64}", "deadbeef"),
            block_timestamp: 1_700_000_000 + (block as i64) * 12,
            topics: vec![topic0, topic1, topic2],
            data,
        }
    }

    /// Helper: build a synthetic deterministic swap stream and ingest it
    /// so the simulator has data to replay.
    async fn build_synthetic_swaps(
        storage: &Storage,
        pool: &str,
        from_block: u64,
        to_block: u64,
        center_tick: i32,
    ) {
        let mock = Arc::new(MockArchiveSource::new(to_block + 100));
        for b in from_block..=to_block {
            // Sinusoidal-ish tick walk around `center_tick`.
            let tick = center_tick
                + (((b - from_block) % 200) as i32 - 100);
            let sqrt = crate::math::tick_to_sqrt_price_x96(tick).unwrap();
            let log = make_swap_log(
                pool,
                b,
                0,
                1_000_000_000_000_000_000i64,
                -3_000_000_000i64,
                &sqrt,
                1_000_000_000_000u128,
                tick,
            );
            mock.add_log(log);
            mock.set_block_gas(b, 20.0);
        }
        let ingester = Ingester::new(storage.clone(), mock);
        ingester
            .backfill(pool, from_block, to_block)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn simulate_static_position_produces_curve() {
        let storage = Storage::open(DbLocation::in_memory()).await.unwrap();
        build_synthetic_swaps(&storage, "0xpool", 100, 200, 0).await;

        let config = PositionConfig {
            pool_address: "0xpool".into(),
            tick_lower: -200,
            tick_upper: 200,
            deposit_token0: "1000000000000000000".into(),
            deposit_token1: "3000000000".into(),
            entry_block: 100,
            exit_block: 200,
            fee_tier_bps: 5,
            token0_decimals: 18,
            token1_decimals: 6,
            mev_haircut_bps: 0.0,
        };
        let engine = Engine::new(&storage);
        let out = engine
            .simulate(config, RebalanceRule::Static)
            .await
            .unwrap();
        assert_eq!(out.equity_curve.len(), 101);
        // All swaps are within tick range [-200, 200].
        assert!(out.summary.time_in_range_pct > 99.0);
        // Fees accrued because in-range and active liquidity.
        assert!(out.summary.total_fees_usd >= 0.0);
        // Mgmt gas: mint + burn at the configured prices, both > 0.
        assert!(out.summary.total_mgmt_gas_usd > 0.0);
        // Rebalance count is zero for a static rule.
        assert_eq!(out.summary.rebalance_count, 0);
    }

    #[tokio::test]
    async fn simulate_persists_to_storage_and_is_idempotent() {
        let storage = Storage::open(DbLocation::in_memory()).await.unwrap();
        build_synthetic_swaps(&storage, "0xpool", 100, 150, 0).await;
        let config = PositionConfig {
            pool_address: "0xpool".into(),
            tick_lower: -100,
            tick_upper: 100,
            deposit_token0: "1000000000000000000".into(),
            deposit_token1: "3000000000".into(),
            entry_block: 100,
            exit_block: 150,
            fee_tier_bps: 30,
            token0_decimals: 18,
            token1_decimals: 6,
            mev_haircut_bps: 5.0,
        };
        let engine = Engine::new(&storage);
        let out1 = engine
            .simulate(config.clone(), RebalanceRule::Static)
            .await
            .unwrap();
        let id1 = storage
            .persist_position_run(out1.summary.clone(), out1.equity_curve.clone())
            .await
            .unwrap();
        let id2 = storage
            .persist_position_run(out1.summary.clone(), out1.equity_curve.clone())
            .await
            .unwrap();
        assert_eq!(id1, id2, "same config_hash must reuse run id");
    }

    #[tokio::test]
    async fn empty_swap_data_returns_error() {
        let storage = Storage::open(DbLocation::in_memory()).await.unwrap();
        let config = PositionConfig {
            pool_address: "0xpool".into(),
            tick_lower: -100,
            tick_upper: 100,
            deposit_token0: "1000000000000000000".into(),
            deposit_token1: "3000000000".into(),
            entry_block: 100,
            exit_block: 200,
            fee_tier_bps: 5,
            token0_decimals: 18,
            token1_decimals: 6,
            mev_haircut_bps: 0.0,
        };
        let engine = Engine::new(&storage);
        let r = engine.simulate(config, RebalanceRule::Static).await;
        assert!(matches!(r, Err(BacktestError::EmptyData { .. })));
    }

    #[tokio::test]
    async fn invalid_tick_range_errors() {
        let storage = Storage::open(DbLocation::in_memory()).await.unwrap();
        let config = PositionConfig {
            pool_address: "0xpool".into(),
            tick_lower: 100,
            tick_upper: 100,
            deposit_token0: "1".into(),
            deposit_token1: "1".into(),
            entry_block: 100,
            exit_block: 200,
            fee_tier_bps: 5,
            token0_decimals: 18,
            token1_decimals: 6,
            mev_haircut_bps: 0.0,
        };
        let engine = Engine::new(&storage);
        let r = engine.simulate(config, RebalanceRule::Static).await;
        assert!(matches!(r, Err(BacktestError::InvalidConfig(_))));
    }

    #[tokio::test]
    async fn schedule_rebalance_increments_count() {
        let storage = Storage::open(DbLocation::in_memory()).await.unwrap();
        build_synthetic_swaps(&storage, "0xpool", 100, 300, 0).await;
        let config = PositionConfig {
            pool_address: "0xpool".into(),
            tick_lower: -200,
            tick_upper: 200,
            deposit_token0: "1000000000000000000".into(),
            deposit_token1: "3000000000".into(),
            entry_block: 100,
            exit_block: 300,
            fee_tier_bps: 5,
            token0_decimals: 18,
            token1_decimals: 6,
            mev_haircut_bps: 0.0,
        };
        let engine = Engine::new(&storage);
        let out = engine
            .simulate(
                config,
                RebalanceRule::Schedule {
                    every_n_blocks: 50,
                },
            )
            .await
            .unwrap();
        assert!(
            out.summary.rebalance_count >= 3,
            "scheduled-50 over 200 blocks should rebalance at least 3 times, got {}",
            out.summary.rebalance_count
        );
        // Rebalances cost mgmt gas.
        assert!(out.summary.total_mgmt_gas_usd > 50.0);
    }

    #[tokio::test]
    async fn out_of_range_position_earns_no_fees() {
        let storage = Storage::open(DbLocation::in_memory()).await.unwrap();
        // Swaps occur around tick 0; position ranges far above.
        build_synthetic_swaps(&storage, "0xpool", 100, 150, 0).await;
        let config = PositionConfig {
            pool_address: "0xpool".into(),
            tick_lower: 50000,
            tick_upper: 60000,
            deposit_token0: "1000000000000000000".into(),
            deposit_token1: "3000000000".into(),
            entry_block: 100,
            exit_block: 150,
            fee_tier_bps: 5,
            token0_decimals: 18,
            token1_decimals: 6,
            mev_haircut_bps: 0.0,
        };
        let engine = Engine::new(&storage);
        let out = engine
            .simulate(config, RebalanceRule::Static)
            .await
            .unwrap();
        assert_eq!(out.summary.time_in_range_pct, 0.0);
        assert_eq!(out.summary.total_fees_usd, 0.0);
    }
}
