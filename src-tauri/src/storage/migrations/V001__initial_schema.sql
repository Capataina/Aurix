-- V001 — Initial schema for the Aurix persistence layer.
--
-- Covers Tab 1 (price snapshots, opportunity log) and Vector A Tab 2
-- (swap events, mint/burn/collect events, position runs, strategy results,
-- benchmark series). Schema designed in one pass per the M2.0 plan to avoid
-- multiple migrations against the swap-event archive.
--
-- Conventions:
--   * All timestamps are unix-millis BIGINT. Block timestamps from the chain
--     are unix-seconds BIGINT (the Ethereum native unit).
--   * Money amounts at fixed precision are stored as TEXT decimal strings
--     when they exceed i64 range (uint256 token amounts, sqrtPriceX96, etc).
--     SQLite has no native uint128/uint256; storing as TEXT keeps arithmetic
--     accurate by deferring to BigUint on read.
--   * USD-denominated derived values (fees_usd, il_usd, etc) are REAL.
--   * Foreign keys rely on `PRAGMA foreign_keys = ON` (set in connection.rs).
--   * Indices are added at table creation; later migrations can extend.

-- ─── Tab 1 — price snapshots and opportunity log ─────────────────────────

CREATE TABLE IF NOT EXISTS price_snapshots (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    chain               TEXT    NOT NULL,
    pair_id             TEXT    NOT NULL,
    dex_name            TEXT    NOT NULL,
    pool_address        TEXT    NOT NULL,
    fee_tier_bps        INTEGER NOT NULL,
    price_usd           REAL    NOT NULL,
    fetched_at_unix_ms  INTEGER NOT NULL,
    gas_price_gwei      REAL,
    UNIQUE (pair_id, dex_name, fetched_at_unix_ms)
);

CREATE INDEX IF NOT EXISTS idx_price_snapshots_pair_time
    ON price_snapshots (pair_id, fetched_at_unix_ms);

CREATE INDEX IF NOT EXISTS idx_price_snapshots_dex_time
    ON price_snapshots (dex_name, fetched_at_unix_ms);

CREATE TABLE IF NOT EXISTS opportunity_log (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    pair_id             TEXT    NOT NULL,
    detected_at_unix_ms INTEGER NOT NULL,
    buy_dex             TEXT    NOT NULL,
    sell_dex            TEXT    NOT NULL,
    buy_price_usd       REAL    NOT NULL,
    sell_price_usd      REAL    NOT NULL,
    spread_bps          REAL    NOT NULL,
    gross_profit_usd    REAL    NOT NULL,
    estimated_gas_usd   REAL    NOT NULL,
    net_profit_usd      REAL    NOT NULL,
    notional_usd        REAL    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_opportunity_log_pair_time
    ON opportunity_log (pair_id, detected_at_unix_ms);

-- ─── Vector A Tab 2 — chain ingestion ────────────────────────────────────

CREATE TABLE IF NOT EXISTS swap_events (
    -- composite primary key: (pool_address, block_number, log_index) is
    -- globally unique on chain and idempotent across re-runs of ingestion.
    pool_address        TEXT    NOT NULL,
    block_number        INTEGER NOT NULL,
    log_index           INTEGER NOT NULL,
    transaction_hash    TEXT    NOT NULL,
    block_timestamp     INTEGER NOT NULL,           -- unix-seconds
    sender              TEXT    NOT NULL,
    recipient           TEXT    NOT NULL,
    -- amount0 / amount1 are signed int256 on chain (signed so negatives
    -- represent pool->user direction). Store as TEXT decimal strings to
    -- preserve full precision; parse to BigInt on read.
    amount0             TEXT    NOT NULL,
    amount1             TEXT    NOT NULL,
    sqrt_price_x96      TEXT    NOT NULL,           -- uint160 as decimal string
    liquidity           TEXT    NOT NULL,           -- uint128 as decimal string
    tick                INTEGER NOT NULL,
    -- gas price at this block in gwei (REAL — we tolerate float precision
    -- here because mgmt-gas modelling tolerates small drift)
    block_gas_price_gwei REAL,
    PRIMARY KEY (pool_address, block_number, log_index)
);

CREATE INDEX IF NOT EXISTS idx_swap_events_pool_block
    ON swap_events (pool_address, block_number);

CREATE INDEX IF NOT EXISTS idx_swap_events_pool_time
    ON swap_events (pool_address, block_timestamp);

-- Mint/Burn/Collect events. The single table with a `kind` column is
-- intentional: all three share the same shape (owner, tickLower, tickUpper,
-- amount, amount0, amount1) and this makes M2.3's reconstruct-liquidity-
-- surface query a single scan. See plan paper 1 §4.
CREATE TABLE IF NOT EXISTS pool_events (
    pool_address        TEXT    NOT NULL,
    block_number        INTEGER NOT NULL,
    log_index           INTEGER NOT NULL,
    transaction_hash    TEXT    NOT NULL,
    block_timestamp     INTEGER NOT NULL,
    kind                TEXT    NOT NULL CHECK (kind IN ('mint','burn','collect')),
    owner               TEXT    NOT NULL,
    tick_lower          INTEGER NOT NULL,
    tick_upper          INTEGER NOT NULL,
    -- liquidity delta for mint/burn (uint128); zero for collect
    liquidity           TEXT    NOT NULL,
    amount0             TEXT    NOT NULL,           -- uint256 / int256
    amount1             TEXT    NOT NULL,
    PRIMARY KEY (pool_address, block_number, log_index)
);

CREATE INDEX IF NOT EXISTS idx_pool_events_pool_block
    ON pool_events (pool_address, block_number);

CREATE INDEX IF NOT EXISTS idx_pool_events_owner_pool
    ON pool_events (owner, pool_address);

-- Per-block gas prices (median across the block) used for management-gas
-- costing in M2.3. Sparse — only blocks we explicitly touch are populated.
CREATE TABLE IF NOT EXISTS block_gas_prices (
    block_number        INTEGER PRIMARY KEY,
    block_timestamp     INTEGER NOT NULL,
    base_fee_gwei       REAL,                       -- post-EIP-1559
    median_gas_gwei     REAL    NOT NULL
);

-- Ingestion checkpoints — tracks the high-watermark block we have for each
-- pool's swap stream. M2.1 reads this to resume cleanly.
CREATE TABLE IF NOT EXISTS ingestion_state (
    pool_address        TEXT    PRIMARY KEY,
    last_swap_block     INTEGER NOT NULL,
    last_pool_event_block INTEGER NOT NULL,
    last_run_at_unix_ms INTEGER NOT NULL
);

-- ─── Vector A Tab 2 — backtest outputs ───────────────────────────────────

-- One row per simulated position run. Hash of inputs is the unique key so
-- re-running the same configuration is idempotent.
CREATE TABLE IF NOT EXISTS position_runs (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    config_hash         TEXT    NOT NULL UNIQUE,
    pool_address        TEXT    NOT NULL,
    tick_lower          INTEGER NOT NULL,
    tick_upper          INTEGER NOT NULL,
    deposit_token0      TEXT    NOT NULL,           -- uint256 raw
    deposit_token1      TEXT    NOT NULL,
    entry_block         INTEGER NOT NULL,
    exit_block          INTEGER NOT NULL,
    rebalance_rule      TEXT    NOT NULL,           -- json-serialised RebalanceRule
    mev_haircut_bps     REAL    NOT NULL DEFAULT 0,
    -- summary outputs (full equity curve in equity_curve_points below)
    total_fees_usd      REAL    NOT NULL,
    total_il_usd        REAL    NOT NULL,
    total_lvr_usd       REAL    NOT NULL,
    total_mgmt_gas_usd  REAL    NOT NULL,
    final_value_usd     REAL    NOT NULL,
    hold_only_value_usd REAL    NOT NULL,
    net_pnl_usd         REAL    NOT NULL,
    time_in_range_pct   REAL    NOT NULL,
    rebalance_count     INTEGER NOT NULL,
    max_drawdown_pct    REAL    NOT NULL,
    sharpe              REAL    NOT NULL,
    sortino             REAL    NOT NULL,
    calmar              REAL    NOT NULL,
    completed_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_position_runs_pool
    ON position_runs (pool_address);

CREATE INDEX IF NOT EXISTS idx_position_runs_sharpe
    ON position_runs (sharpe);

CREATE TABLE IF NOT EXISTS equity_curve_points (
    run_id              INTEGER NOT NULL,
    sample_idx          INTEGER NOT NULL,
    block_number        INTEGER NOT NULL,
    block_timestamp     INTEGER NOT NULL,
    position_value_usd  REAL    NOT NULL,
    fees_accumulated_usd REAL   NOT NULL,
    il_usd              REAL    NOT NULL,
    lvr_usd             REAL    NOT NULL,
    mgmt_gas_paid_usd   REAL    NOT NULL,
    hold_only_value_usd REAL    NOT NULL,
    net_pnl_usd         REAL    NOT NULL,
    in_range            INTEGER NOT NULL,           -- 0 / 1
    PRIMARY KEY (run_id, sample_idx),
    FOREIGN KEY (run_id) REFERENCES position_runs(id) ON DELETE CASCADE
);

-- Strategy comparison grid results — one row per cell of (range × rebalance
-- × deposit × period). Same metrics as position_runs but indexed for
-- heatmap queries.
CREATE TABLE IF NOT EXISTS strategy_results (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    grid_id             TEXT    NOT NULL,
    pool_address        TEXT    NOT NULL,
    range_width_pct     REAL    NOT NULL,
    rebalance_rule      TEXT    NOT NULL,
    deposit_usd         REAL    NOT NULL,
    period_days         INTEGER NOT NULL,
    period_start_unix_ms INTEGER NOT NULL,
    period_end_unix_ms  INTEGER NOT NULL,
    fees_usd            REAL    NOT NULL,
    il_usd              REAL    NOT NULL,
    lvr_usd             REAL    NOT NULL,
    mgmt_gas_usd        REAL    NOT NULL,
    net_return_usd      REAL    NOT NULL,
    net_return_vs_hold  REAL    NOT NULL,
    time_in_range_pct   REAL    NOT NULL,
    rebalance_count     INTEGER NOT NULL,
    max_drawdown_pct    REAL    NOT NULL,
    sharpe              REAL    NOT NULL,
    sortino             REAL    NOT NULL,
    calmar              REAL    NOT NULL,
    deflated_sharpe     REAL    NOT NULL,
    completed_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_strategy_results_grid
    ON strategy_results (grid_id);

CREATE INDEX IF NOT EXISTS idx_strategy_results_pool_sharpe
    ON strategy_results (pool_address, sharpe);

CREATE INDEX IF NOT EXISTS idx_strategy_results_period
    ON strategy_results (period_start_unix_ms, period_end_unix_ms);

-- ─── Vector A Tab 2 — benchmark series ───────────────────────────────────

-- Daily benchmark series: APYs from DefiLlama/Aave/Compound/Lido, ETH
-- staking yield, TradFi rates from FRED/Stooq/Yahoo. The `series_key`
-- distinguishes streams (e.g. "aave_v3_usdc_supply_apy", "lido_apr",
-- "fred_dgs3mo", "yahoo_sp500tr_pct_return", "v2lp_full_range_pnl_usd").
CREATE TABLE IF NOT EXISTS benchmark_series (
    series_key          TEXT    NOT NULL,
    sample_date         TEXT    NOT NULL,           -- ISO YYYY-MM-DD
    value               REAL    NOT NULL,
    source              TEXT    NOT NULL,           -- "defillama" | "fred" | "stooq" | "yahoo" | "beaconchain" | "synthetic"
    fetched_at_unix_ms  INTEGER NOT NULL,
    PRIMARY KEY (series_key, sample_date)
);

CREATE INDEX IF NOT EXISTS idx_benchmark_series_date
    ON benchmark_series (sample_date);

-- ─── Vector A Tab 2 — headline analysis (M2.8) ───────────────────────────

CREATE TABLE IF NOT EXISTS headline_runs (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    config_hash         TEXT    NOT NULL UNIQUE,
    pool_address        TEXT    NOT NULL,
    lookback_months     INTEGER NOT NULL,
    regime_method       TEXT    NOT NULL,           -- "adaptive_terciles" | "fixed"
    months_lp_beat_lending INTEGER NOT NULL,
    months_total        INTEGER NOT NULL,
    median_low_vol_spread  REAL,
    median_med_vol_spread  REAL,
    median_high_vol_spread REAL,
    verdict_text        TEXT    NOT NULL,
    completed_at_unix_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS headline_monthly (
    headline_run_id     INTEGER NOT NULL,
    year_month          TEXT    NOT NULL,           -- "YYYY-MM"
    vol_regime          TEXT    NOT NULL,           -- "low" | "medium" | "high"
    best_lp_return      REAL    NOT NULL,
    naive_lp_return     REAL    NOT NULL,
    median_lp_return    REAL    NOT NULL,
    aave_usdc_return    REAL    NOT NULL,
    lido_steth_return   REAL    NOT NULL,
    hodl_return         REAL    NOT NULL,
    eth_vol_30d         REAL    NOT NULL,
    PRIMARY KEY (headline_run_id, year_month),
    FOREIGN KEY (headline_run_id) REFERENCES headline_runs(id) ON DELETE CASCADE
);
