//! Strategy comparison grid (M2.5).

#![allow(dead_code)]

pub mod error;
pub mod grid;

pub use error::StrategyError;
pub use grid::{GridConfig, GridRunner};

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Arc;

    use num_bigint::BigUint;

    use crate::backtest::RebalanceRule;
    use crate::ingest::{decoder::SWAP_TOPIC0, EthLog, Ingester, MockArchiveSource};
    use crate::storage::{DbLocation, Storage};
    use crate::math::tick_to_sqrt_price_x96;

    use super::grid::{GridConfig, GridRunner};

    pub(crate) fn dummy_link() {}

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

    async fn build_swaps(storage: &Storage, pool: &str, from: u64, to: u64) {
        let mock = Arc::new(MockArchiveSource::new(to + 100));
        for b in from..=to {
            let tick = ((b - from) as i32 % 200) - 100;
            let sqrt = tick_to_sqrt_price_x96(tick).unwrap();
            let topic0 = format!("0x{SWAP_TOPIC0}");
            let topic1 = format!("0x{:0>64}", "1");
            let topic2 = format!("0x{:0>64}", "2");
            let a0 = encode_int256_word(1_000_000_000_000_000_000);
            let a1 = encode_int256_word(-3_000_000_000);
            let sq = encode_uint_word(&sqrt);
            let li = encode_uint_word(&BigUint::from(1_000_000_000_000u128));
            let tk = encode_int24_word(tick);
            let log = EthLog {
                address: pool.to_string(),
                block_number: b,
                log_index: 0,
                transaction_hash: format!("0x{:0>64}", "deadbeef"),
                block_timestamp: 1_700_000_000 + (b as i64) * 12,
                topics: vec![topic0, topic1, topic2],
                data: format!("0x{a0}{a1}{sq}{li}{tk}"),
            };
            mock.add_log(log);
            mock.set_block_gas(b, 20.0);
        }
        let ingester = Ingester::new(storage.clone(), mock);
        ingester.backfill(pool, from, to).await.unwrap();
    }

    #[tokio::test]
    async fn run_grid_executes_every_cell() {
        let storage = Storage::open(DbLocation::in_memory()).await.unwrap();
        build_swaps(&storage, "0xpool", 1000, 1500).await;
        let config = GridConfig {
            grid_id: "test_grid".into(),
            pool_address: "0xpool".into(),
            range_widths_pct: vec![1.0, 5.0],
            rebalance_rules: vec![
                RebalanceRule::Static,
                RebalanceRule::Schedule { every_n_blocks: 100 },
            ],
            deposits_usd: vec![10_000.0],
            periods_days: vec![30],
            fee_tier_bps: 5,
            token0_decimals: 18,
            token1_decimals: 6,
            mev_haircut_bps: 0.0,
            period_end_block: 1500,
            // small blocks_per_day so 30-day window fits in our synthetic span
            blocks_per_day: 10,
        };
        let runner = GridRunner::new(&storage);
        let rows = runner.run_grid(config).await.unwrap();
        assert_eq!(rows.len(), 4, "2 widths × 2 rules × 1 deposit × 1 period");

        // Persisted to storage
        let stored = storage
            .query_strategy_results("test_grid".into())
            .await
            .unwrap();
        assert_eq!(stored.len(), 4);
    }

    #[tokio::test]
    async fn run_grid_rejects_empty_axes() {
        let storage = Storage::open(DbLocation::in_memory()).await.unwrap();
        let config = GridConfig {
            grid_id: "g".into(),
            pool_address: "0xp".into(),
            range_widths_pct: vec![],
            rebalance_rules: vec![RebalanceRule::Static],
            deposits_usd: vec![1.0],
            periods_days: vec![1],
            fee_tier_bps: 5,
            token0_decimals: 18,
            token1_decimals: 6,
            mev_haircut_bps: 0.0,
            period_end_block: 100,
            blocks_per_day: 7200,
        };
        let runner = GridRunner::new(&storage);
        assert!(runner.run_grid(config).await.is_err());
    }
}
