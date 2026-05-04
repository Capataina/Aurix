# Idempotent Runs

## Current Understanding

Every Vector A backend operation that the frontend's auto-run pipeline triggers is structurally idempotent — running the same `(config) → result` mapping twice with identical inputs produces identical outputs and zero net DB churn beyond reading the cached row.

Idempotency is enforced at the **storage** layer (per [storage-conventions](storage-conventions.md)) and used at the **engine + grid + headline** layers to short-circuit cached results. The combination is what lets `LpBacktestPage`'s React StrictMode double-mount produce one observable result instead of two duplicate runs.

## How Each Layer Achieves Idempotency

### `Engine::simulate(config, rule)`

- Storage's `position_runs` table is keyed on `config_hash = SHA(config)`; `INSERT OR IGNORE` makes a second `persist_position_run` call with the same hash a no-op.
- `commands::lp::run_lp_backtest` first calls `simulate`, then calls `persist_position_run`. The second call from a duplicate IPC invocation hits the existing row, returns the same `id`, and the frontend renders the cached `equity_curve` + `summary`.

### `GridRunner::run_grid(grid_config)`

- Each cell's `PositionConfig` flows through `Engine::simulate` → cached as above.
- The grid id is deterministic from `grid_config`; `strategy_results` rows are keyed on `(grid_id, cell_index)` with `INSERT OR IGNORE`.
- Re-running a grid with overlapping cells (same range × same rule × same deposit but different period) hits the per-cell cache for shared cells.

### `HeadlineRunner::run(headline_config)`

- Per-month sub-backtests flow through `Engine::simulate` → cached.
- `headline_runs` rows are keyed on `config_hash`; `INSERT OR IGNORE` makes the duplicate call a no-op.

### `Ingester::backfill(pool, from, to)`

- `swap_events` and `pool_events` rows are keyed on `(pool_address, block_number, log_index)` — chain-globally unique.
- Re-running ingest over the same block range inserts zero new rows; the source-side fetch still happens (network call) but the persistence is a no-op.
- The synthetic-vs-live separation (`SYNTHETIC_TX_HASH`) keeps the two streams from interfering.

## Why This Matters

**StrictMode robustness.** React 18 StrictMode (active in dev mode) intentionally double-mounts components. The auto-run `useEffect` fires twice on cold start. Without idempotency, the second mount would re-run the whole pipeline against the same inputs, doubling ingest network calls and DB writes. With idempotency, the second mount's pipeline sees cached data and short-circuits.

**Re-run UX.** The user can click Re-run repeatedly without growing the database. The frontend's `rerunNonce` mechanism intentionally re-fires the pipeline; idempotency means the cost is one cache-hit per call, not one re-execution.

**Failure recovery.** A pipeline failure mid-step does not corrupt state. The next run picks up from cached rows + retries the failed step.

## Guiding Principles

- New backend functions invoked from Tauri commands follow the same pattern: deterministic config hash + `INSERT OR IGNORE` keyed on the hash. Adding a new IPC that mutates state without idempotency is a regression.
- The idempotency contract relies on **all** of the storage conventions in [storage-conventions](storage-conventions.md). Address-case normalisation is part of the contract — without it, querying an idempotent insert with a different case produces zero-row results, defeating the cache.
- `config_hash` should hash **only** the inputs that semantically determine the output. Including `chrono::Utc::now()` or any wall-clock value defeats the cache.

## Verification Pattern

Every idempotency-claiming function has at least one test that:
1. Calls the function with a fixed config.
2. Calls it again with the same config.
3. Asserts the second call returns the same id / result without duplicate rows.

Examples: `storage::swaps::tests::batch_insert_is_idempotent`, `storage::runs::tests::re_persisting_same_hash_is_idempotent`, `backtest::tests::simulate_persists_to_storage_and_is_idempotent`.

## Cross-references

- Storage layer: [storage-conventions](storage-conventions.md), [storage system](../systems/storage.md).
- Frontend consequence: [lp-backtest-gui system](../systems/lp-backtest-gui.md) StrictMode discipline section.
- Related: commit `43599ba` (the StrictMode bug that this contract closed).
