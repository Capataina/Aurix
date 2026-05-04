# Strategies

## Scope / Purpose

- Grid search over LP strategy variants — `range × rule × deposit × period` — invoking [backtest](backtest.md)'s `Engine::simulate` per cell, collecting per-cell metrics (Sharpe, Sortino, Calmar, max drawdown, time-in-range, fees, IL), and persisting the full grid for the strategy heatmap UI.
- Drives the "compare strategies" section of the LP dashboard. The grid result feeds both the heatmap visualisation and the headline verdict synthesis.

## Boundaries / Ownership

- Owns: grid configuration shape (`GridConfig`), grid execution loop (`GridRunner::run_grid`), per-cell metric aggregation, persistence of `StrategyResultRow` rows.
- Does **not** own: the per-cell simulation (delegated to [backtest](backtest.md)), the metric formulas (those live in `backtest::metrics`), the grid result rendering (that's [lp-backtest-gui](lp-backtest-gui.md)).

## Current Implemented Reality

```text
strategies/
├── mod.rs              # GridRunner + tests (3 tests)
└── grid.rs             # GridConfig + axis enumeration + per-cell driver (277 lines, 3 tests)
```

**`GridConfig` axes:**
- `range_widths_ticks: Vec<i32>` — per-side tick distance from current tick (e.g. `[100, 200, 400]` produces 3 candidate ranges)
- `rebalance_rules: Vec<RebalanceRule>` — Static / Schedule { every_n } / OutOfRange { trigger_after_blocks }
- `deposit_splits: Vec<(f64, f64)>` — token0/token1 split ratios (e.g. `[(1.0, 0.0), (0.5, 0.5), (0.0, 1.0)]`)
- `period_days: Vec<u32>` — backtest window length (e.g. `[7, 30, 90]`)

Total cell count = product of axis lengths. `validate` rejects empty axes (per `grid.rs:tests::validate_rejects_empty_axes`).

**Cell execution.** For each axis combination, `GridRunner::run_grid`:
1. Constructs a `PositionConfig` from the cell parameters + the user's pool and chain context.
2. Calls `Engine::simulate(config, rule)` (cached via `config_hash` in [storage](storage.md)).
3. Aggregates the resulting `PositionRunSummary` into a `StrategyResultRow` (preserves Sharpe / Sortino / Calmar / max DD / fees / IL / time-in-range / rebalance count).
4. Persists the row.

**`config_hash` reuse.** Because [backtest](backtest.md) keys runs by `config_hash`, re-running a grid with overlapping cells (e.g. same range × same rule × same deposit × different period) hits the cache for shared cells. Idempotent.

**Per-chain block-time conventions** — added in commit 391eadd's Tier 2 cross-chain extension. Each chain has a different average block time (Ethereum 12s, Arbitrum 0.25s, Optimism 2s, Base 2s, Polygon 2s); the grid runner uses `config::chains::ChainConfig::block_time_seconds` to convert period-days into block ranges per chain.

## Key Interfaces / Data Flow

| Boundary | Direction | Shape | Notes |
|---|---|---|---|
| `commands::lp::run_lp_grid` → grid runner | inbound | `GridConfig` | from frontend via Tauri IPC |
| grid runner → [backtest](backtest.md) | outbound (call) | `PositionConfig` + `RebalanceRule` per cell | sequential per-cell loop today; future parallelism candidate |
| grid runner → [storage](storage.md) | outbound (write) | `Vec<StrategyResultRow>` via `persist_strategy_results` | persisted under a generated `grid_id` |
| `commands::lp::lp_query_strategies` → grid storage | outbound (read) | `Vec<StrategyResultRow>` for `grid_id` | for the heatmap re-render path |

## Implemented Outputs / Artifacts

- `strategy_results` table in [storage](storage.md), keyed by `(grid_id, cell_index)`.
- 3 tests in `strategies/{grid,mod}.rs`: cell count multiplies axes, human→raw rounding, empty-axis rejection.

## Known Issues / Active Risks

- **Sequential per-cell execution.** Each cell runs one after the other. For a `3 × 3 × 3 × 3 = 81`-cell grid over 1000 blocks, that's 81 sequential `Engine::simulate` calls, each touching ~1000 swap rows. With the per-loop perf findings in [backtest](backtest.md) unaddressed, total grid runtime scales as N_cells × N_swaps × per-swap_cost. Parallelising via `tokio::join_all` over independent cells is a future option — recorded here, not as a finding (architectural-class change, out of audit scope).
- **Deflated Sharpe ratio not yet implemented.** The backtest's `metrics::sharpe_ratio` is the standard form. The `references/backtest-statistical-methodology.md` paper covers the Bailey/de Prado deflated form for selection-bias correction; the strategies grid has a strong selection-bias risk (picking the best Sharpe out of N cells inflates its expected value). Documented in research; not currently in code.

## Partial / In Progress

- None — basic grid is code-complete.

## Planned / Missing / Likely Changes

- Deflated Sharpe ratio per cell (post-audit finding).
- Optional parallelism via `tokio::join_all`.
- Adaptive grid pruning (skip cells whose deposit split is structurally redundant).

## Durable Notes / Discarded Approaches

- **Cartesian-product axes over arbitrary cell lists.** Earlier design considered letting users specify each cell directly; the cartesian product is more constraining but produces interpretable grids (rows × columns map to axis variations). The heatmap UI relies on the cartesian shape.

## Obsolete / No Longer Relevant

- None.

## Cross-references

- Caller: `commands::lp::run_lp_grid`.
- Consumer of: [backtest](backtest.md) (per cell), [storage](storage.md).
- Producer for: `strategy_results` rows.
- Used by: [headline](headline.md) consumes the persisted strategy grid for the verdict synthesis.
- Related research: `references/backtest-statistical-methodology.md`, `references/lp-rebalancing-strategies.md`.
