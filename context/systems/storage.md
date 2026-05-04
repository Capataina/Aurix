# Storage

## Scope / Purpose

- The persistence layer for all Vector A backend state. Owns SQLite-on-disk via `rusqlite` for sync paths and `tokio-rusqlite` for the async writer, runs migrations through `refinery`, and offers a domain-organised public API (one submodule per table family).
- Topology choice: one writer connection on its own dedicated thread (the tokio-rusqlite async writer) plus a fixed-size reader pool via `r2d2_sqlite`. Production pragmas (`journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`, `temp_store=MEMORY`) are applied on every connection acquisition. Forward-only migrations are embedded via `refinery` macros from `migrations/`.

## Boundaries / Ownership

- This subsystem owns: every table in `aurix.sqlite`, every read-side query against those tables, schema migration management, the writer/reader topology, the WAL checkpoint policy hook (`Storage::checkpoint`), and the per-domain row types (`SwapEventRow`, `PoolEventRow`, `EquityCurvePoint`, `PositionRunSummary`, `StrategyResultRow`, `HeadlineMonthlyRow`, `BenchmarkPoint`, `IngestionCheckpoint`, etc.).
- This subsystem does **not** own: the data semantics (those live in [backtest](backtest.md), [ingest](ingest.md), [strategies](strategies.md), [headline](headline.md), [benchmarks](benchmarks.md)) — the storage layer is deliberately a CRUD façade; the meaning of `sqrt_price_x96` belongs to [math](math.md), not here.
- Database file path: `~/.aurix/aurix.sqlite` by default; overridable via the `AURIX_DB_PATH` environment variable (per `lib.rs:resolve_db_path`).

## Current Implemented Reality

The crate-public surface lives at `src-tauri/src/storage/mod.rs`:

```text
storage/
├── mod.rs                       # Storage handle + write/read/checkpoint API
├── connection.rs                # DbLocation, async writer, reader pool, pragmas
├── error.rs                     # StorageError thiserror enum (fan-in 12)
├── migrations.rs                # refinery embed + run
├── migrations/
│   ├── V001__initial.sql        # Initial schema for all Vector A tables
│   └── V002__multi_asset_headline.sql  # Adds months_lp_beat_* + per-asset returns
├── snapshots.rs                 # Tab 1 price-snapshot CRUD (legacy)
├── swaps.rs                     # V3 Swap event CRUD + idempotent batch insert
├── pool_events.rs               # V3 Mint/Burn/Collect event CRUD
├── gas.rs                       # Per-block gas-price persistence + upsert
├── runs.rs                      # Position simulation runs + equity curve points
├── strategy.rs                  # Strategy grid results
├── benchmarks.rs                # Daily benchmark series cache
├── headline.rs                  # Headline runs + per-month outputs (M2.8)
└── state.rs                     # Ingestion checkpoints (per pool)
```

Each domain module hangs `impl Storage { ... }` blocks on the shared `Storage` handle, exposing async write methods and sync (or `spawn_blocking`-ed) read methods. Callers compose via `storage.insert_swap_events_batch(...)` etc., not by importing per-table types separately.

**Idempotency contract.** Every batch-insert path uses `INSERT OR IGNORE` keyed on a domain-natural unique constraint (`(pool_address, block_number, log_index)` for swaps and pool events; `config_hash` for runs and headlines; `(grid_id, cell_index)` for strategies). Re-running ingestion against the same block range or re-running a backtest with the same config is a no-op. This is load-bearing for the LP-page auto-run pipeline (cold-start hits identical inputs every time and must reuse cached results).

**Address-case normalisation.** Pool addresses are lowercased on **both** insert and query paths inside `storage/swaps.rs::insert_swap_events_batch` and `storage/pool_events.rs::insert_pool_events_batch`. Live Alchemy logs arrive lowercase by chain convention; user-supplied EIP-55-checksummed addresses arrive mixed-case. Both must agree at the DB level — see commit 53f99eb for the bug this fix closed.

**Synthetic vs live separation.** Every synthetic swap row carries `transaction_hash = SYNTHETIC_TX_HASH` (the all-zero-prefix `"0x...deadbeef"` constant). `delete_synthetic_swaps_in_range` uses this to wipe-then-reinsert when the synthetic generator is re-run with tweaked parameters; live ingestion never touches rows with this hash. Real Ethereum tx-hash collision probability with the synthetic constant is ~2⁻²²⁴ — effectively zero.

## Key Interfaces / Data Flow

The public surface is `Storage` plus its inherent methods. The handle is cheap to clone (both halves are Arc-internally) and is registered as Tauri-managed state in `lib.rs:run` via `tauri::Builder::default().manage(Arc::new(storage))`.

| Method | Purpose | Caller |
|---|---|---|
| `Storage::open(DbLocation)` | open + run migrations + build reader pool | `lib.rs::open_storage` (once at startup) |
| `Storage::write<R, F>(F)` | one-shot async write via the dedicated writer thread | every domain module's batch-insert helpers |
| `Storage::read()` | acquire a reader connection from the pool | sync read paths |
| `Storage::checkpoint()` | issue `PRAGMA wal_checkpoint(TRUNCATE)` | callable but currently unscheduled (see Known Issues) |
| `Storage::insert_swap_events_batch(Vec<SwapEventRow>) -> usize` | bulk-insert swaps inside one transaction; idempotent | [ingest](ingest.md) `Ingester::backfill` |
| `Storage::insert_pool_events_batch(Vec<PoolEventRow>) -> usize` | bulk-insert mints/burns/collects | ingest |
| `Storage::query_swaps_for_pool_range(pool, from, to) -> Vec<SwapEventRow>` | range query ordered by `(block, log_index)` | [backtest](backtest.md) `Engine::simulate` |
| `Storage::persist_position_run(summary, curve)` | persist run summary + equity curve atomically; returns existing id on duplicate `config_hash` | [backtest](backtest.md), [strategies](strategies.md), [headline](headline.md) |
| `Storage::persist_strategy_results(grid_id, rows)` | persist per-cell strategy outputs | [strategies](strategies.md) |
| `Storage::persist_headline_run(...)` + `persist_headline_monthly_rows(...)` | persist M2.8 capital-allocation outputs | [headline](headline.md) |
| `Storage::insert_benchmark_points_batch(rows)` | persist daily benchmark series | [benchmarks](benchmarks.md) |
| `Storage::delete_synthetic_swaps_in_range(...)` | wipe synthetic rows before re-inserting | `commands::lp::run_lp_synthetic_ingest` |

**Boundary data shape.** Domain row structs live alongside their CRUD module (`SwapEventRow` in `storage/swaps.rs`, etc.). Big-integer columns (`amount0`, `amount1`, `sqrt_price_x96`, `liquidity`) are stored as TEXT decimal strings to preserve full uint160/int256/uint128 precision; callers parse to `BigInt`/`BigUint`/`u128` on read. See [audit findings](../plans/code-health-audit/backtest.md) for the per-loop parse cost this creates and the recommended hoist.

## Implemented Outputs / Artifacts

- `aurix.sqlite` (WAL-mode SQLite database) at `~/.aurix/aurix.sqlite`.
- Migration history table `refinery_schema_history` (managed by refinery).
- 18 unit/integration tests across the storage submodules — full per-table round-trip + idempotency coverage.

## Known Issues / Active Risks

- **WAL checkpoint cadence.** `Storage::checkpoint()` exists but is not called by any scheduled task. The WAL grows unbounded under sustained ingest workloads (per the [potential-issues sweep](../plans/code-health-audit/potential-issues.md) §5). SQLite's default 1000-page auto-truncate threshold catches the worst case but is not a substitute for explicit cadence. Downstream impact: long-running backtest sessions can produce hundreds of MB of WAL before the auto-trigger fires.
- **f64 boundary on the IPC contract.** `EquityCurvePoint`'s USD fields cross to TypeScript as `f64`; for very long backtests this can accumulate ULP-level drift relative to a fully-bigint accumulator. Documented in [Aurix/Gaps Gap 5](https://github.com/Capataina/LifeOS/blob/main/Projects/Aurix/Gaps.md) for Tab 1; the same constraint applies here.
- **TEXT-encoded big integers vs BLOB.** Storing 256-bit integers as decimal strings is correct for precision but costs per-loop parse work in the backtest engine. Could be moved to BLOB encoding (32-byte big-endian) — recorded as Option B in [audit findings](../plans/code-health-audit/backtest.md) §"Pre-parse swap rows once".

## Partial / In Progress

- None — the storage layer is code-complete for Vector A as of the 2026-05-03 sprint.

## Planned / Missing / Likely Changes

- Periodic WAL-checkpoint task at startup (60-second tokio interval, calling `Storage::checkpoint`).
- Optional BLOB-encoded big-int columns if the per-loop parse cost in the backtest engine becomes load-bearing.
- A `DROP TABLE` migration path (refinery is forward-only by design; non-trivial to remove a table once shipped).

## Durable Notes / Discarded Approaches

- **`tokio-rusqlite` writer + `r2d2_sqlite` reader pool was chosen over a single thread-pool.** The writer-on-its-own-thread isolates write contention from reads under WAL mode (where readers do not block writers and vice versa, but writer concurrency is still bounded). See `references/sqlite-rust-production-patterns.md` for the design rationale.
- **`refinery` was chosen over `rusqlite_migration` and `sqlx`-style migrations.** `refinery` embeds raw SQL files, which keeps migrations grep-friendly and lets the team write SQL directly rather than through a Rust DSL. See `references/sqlite-rust-production-patterns.md`.
- **Per-pool ingestion checkpoints (`storage::state`).** Originally considered storing in `pool_events` as a flag column; moved to a dedicated `ingest_state` table because the checkpoint advances per ingest pass independently of any specific event.

## Obsolete / No Longer Relevant

- The Tab 1 `snapshots` table predates the Vector A persistence design and is not used by any LP path. It remains for the Tab 1 historical-chart feature (M1.5 in `README.md`'s roadmap, unchecked). Not a candidate for removal until Tab 1's roadmap is resolved.

## Cross-references

- Consumers of this system: [ingest](ingest.md), [backtest](backtest.md), [strategies](strategies.md), [headline](headline.md), [benchmarks](benchmarks.md), [validation](validation.md), `commands/lp.rs`.
- Producer of: every persisted Vector A row.
- Related research: `references/sqlite-rust-production-patterns.md`.
- Related convention: `notes/storage-conventions.md` (idempotent INSERT OR IGNORE, address-case normalisation, synthetic-row separation).
