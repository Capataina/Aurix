# Backtest

## Scope / Purpose

- Position-simulation engine for Uniswap V3 LP positions. Walks every swap in `[entry_block, exit_block]` for a single pool and replays its effect on a configured LP position: per-swap fee accrual when in-range, impermanent-loss accumulation vs hold-only baseline, LVR (Loss-Versus-Rebalancing per Milionis-Moallemi-Roughgarden) discrete approximation, management-gas costs at chain-historical block prices, optional MEV haircut, and a per-sample equity curve.
- Load-bearing computational core of Vector A — every LP backtest, every cell of the [strategies](strategies.md) grid, and every month of the [headline](headline.md) verdict synthesis flows through `Engine::simulate`.

## Boundaries / Ownership

- Owns: per-swap simulation loop, fee accumulation, IL computation, LVR computation, mgmt-gas accounting, rebalance triggering (via `RebalanceContext` consumed by `RebalanceRule`), equity-curve emission, run-summary aggregation.
- Does **not** own: the V3 mathematics (those live in [math](math.md)), persistence (those live in [storage](storage.md)), the strategy grid (that's [strategies](strategies.md)), the headline verdict prose (that's [headline](headline.md)).
- Composite hotspot 0.93 — second-highest in the repo.

## Current Implemented Reality

```text
backtest/
├── mod.rs              # Engine + tests fixture + integration tests
├── engine.rs           # Engine::simulate — the per-swap simulation loop (374 lines)
├── error.rs            # BacktestError thiserror enum
├── gas.rs              # MgmtGasOp + cost_usd + mev_haircut_usd
├── metrics.rs          # Sharpe / Sortino / Calmar / max-drawdown / annualise
├── position.rs         # PositionConfig (Rust↔TS DTO) + tick_lower / tick_upper validation
├── price.rs            # sqrt_price_x96_to_human_price + position_usd_value(_explicit)
└── rebalance.rs        # RebalanceRule enum (Static / Schedule / OutOfRange) + RebalanceContext
```

**`Engine::simulate(config, rule)` flow.** Source: `engine.rs:51-358`.

1. Validate config (bounds, deposit non-zero).
2. Load swaps via `storage.query_swaps_for_pool_range`.
3. Initialise position state from first swap's `sqrt_price_x96`: compute liquidity from deposit + entry price via `math::liquidity::liquidity_for_amounts`.
4. Pay mint cost at entry (`MgmtGasOp::Mint` priced at first-swap gas).
5. **Per-swap loop:**
   a. Parse `swap.sqrt_price_x96` / `liquidity` / `amount0` / `amount1` from TEXT decimal strings.
   b. Determine `in_range` (current tick in `[tick_lower, tick_upper)`).
   c. Compute fees via `math::fees::fee_share_token0/1` (zero when out-of-range).
   d. Accumulate LVR (in-range, when prev_sqrt is non-zero) via the discrete approximation `0.5 * Δsqrt² * L / (sqrt * Q96)`.
   e. Check rebalance trigger; if firing, pay rebalance gas + MEV haircut + recentre range around current tick (preserving width) + recompute liquidity.
   f. Compute current position USD value via `value_usd(a0_cur, a1_cur, cur_price)` plus accumulated fees.
   g. Compute hold-only baseline + IL = position - hold-only.
   h. Emit equity-curve point.
6. Pay burn cost at exit (`MgmtGasOp::Burn` priced at last-swap gas), update final equity-curve row.
7. Aggregate run summary (max drawdown, Sharpe, Sortino, Calmar, time-in-range, rebalance count).

**Rebalance rules.** `RebalanceRule` enum from `rebalance.rs`:
- `Static` — no rebalances.
- `Schedule { every_n_blocks }` — fixed-cadence rebalancing.
- `OutOfRange { trigger_after_blocks }` — rebalance when out-of-range for N blocks.

`RebalanceContext` carries `current_block`, `blocks_since_last_rebalance`, `current_tick`, `tick_lower`, `tick_upper`, `blocks_out_of_range`. The rule's `should_rebalance` method consumes the context; engine drives the trigger.

**`config_hash` keying.** `PositionConfig::config_hash()` is the deterministic hash used by [storage](storage.md) to key the run cache — re-running a backtest with identical inputs returns the existing run id. Frontend's auto-run pipeline is fully idempotent because of this.

## Key Interfaces / Data Flow

| Boundary | Direction | Shape | Notes |
|---|---|---|---|
| `commands::lp::run_lp_backtest` → engine | inbound | `PositionConfig` + `RebalanceRule` | from frontend via Tauri IPC, camelCase JSON serde |
| engine → [storage](storage.md) | inbound (read) | `Vec<SwapEventRow>` via `query_swaps_for_pool_range` | TEXT-decimal big-int columns parsed in-loop |
| engine → [math](math.md) | inbound (call) | `tick_to_sqrt_price_x96`, `liquidity_for_amounts`, `amounts_for_liquidity`, `fee_share_token0/1` | pure-function calls, no storage interaction |
| engine → [storage](storage.md) | outbound (write) | `(PositionRunSummary, Vec<EquityCurvePoint>)` via `persist_position_run` | idempotent on `config_hash` |
| engine → caller | outbound | `SimulationOutput { summary, equity_curve }` | also returned to IPC callers (via `BacktestResponse`) |

**Critical-path trace** (the user clicks Re-run on the LP page):

```
LpBacktestPage useEffect fires →
  api.runLpIngestion(pool, from, to, chain, proto) →
    Tauri IPC commands::lp::run_lp_ingestion →
      Ingester::backfill (3-tier fallback per ingest.md) →
        Storage::insert_swap_events_batch (idempotent) →
  api.runLpBacktest(config, rule) →
    Tauri IPC commands::lp::run_lp_backtest →
      Engine::simulate →
        Storage::query_swaps_for_pool_range (~1k rows) →
        per-swap loop calling math::* primitives →
        Storage::persist_position_run (idempotent on config_hash) →
        BacktestResponse { summary, equity_curve } →
  api.runLpGrid(grid_config) → strategies grid runner →
  api.runLpHeadline(headline_config) → headline runner →
  React state updates → block components render
```

Total systems touched: 8 (UI · IPC · ingest · storage · backtest · math · strategies · headline). Boundary data shapes: `PositionConfig` Rust↔TS, `EquityCurvePoint` Rust→TS via JSON, `SwapEventRow` storage↔backtest internal.

**Failure behaviour at each step:**
- ingest fail → CommandError surfaces in UI as red banner, pipeline halts
- empty swaps → `BacktestError::EmptyData` surfaces
- math overflow → `V3MathError` propagates to `BacktestError::MathError`
- storage write fail → `StorageError` propagates to `BacktestError::Storage`
- IPC serialisation fail → frontend rejects, error banner

## Implemented Outputs / Artifacts

- `position_runs` rows + `equity_curve_points` rows in [storage](storage.md).
- 6 in-crate integration tests in `backtest/mod.rs:117-305` — static rebalance, idempotent persist, empty-data error, invalid-config error, scheduled rebalance, out-of-range zero fees.

## Known Issues / Active Risks

- **Per-loop allocation pressure.** Every swap parses 4 BigUint/BigInt/u128 values from strings, creates a closure dispatch in `value_usd`, and re-converts the running fee accumulators to USD. For long backtests (10k+ swaps) this is the dominant cost. Recorded as 4 perf findings in [audit findings](../plans/code-health-audit/backtest.md). Downstream: every cell of the strategies grid + every month of the headline verdict pays this cost.
- **f64 LVR precision** at extreme sqrtPrice ranges. See [math](math.md) Known Issues.
- **`engine.rs` is monolithic at 374 lines.** Three concerns packed into one function (setup, per-swap loop, summary). Recorded as a split-recommended modularisation finding.

## Partial / In Progress

- None — engine is code-complete.

## Planned / Missing / Likely Changes

- Per-loop perf wins (hold-only hoist, incremental fees-USD, pre-parse swap rows, `Lazy<[BigUint; 20]>` magic constants in [math](math.md)). All recorded in [audit findings](../plans/code-health-audit/backtest.md).
- Engine refactor into `setup → step → summarise` per the modularisation finding.

## Durable Notes / Discarded Approaches

- **Per-swap fee distribution over per-block aggregation.** Block-level fee aggregation would be faster but loses information when multiple swaps cross tick boundaries within the same block. Per-swap is the canonical approach in production V3 backtesters; documented in `references/v3-position-validation-methodology.md`.
- **Discrete LVR approximation over continuous-time integral.** The continuous-time Milionis-Moallemi-Roughgarden integral requires arbitrage-rate parameters that vary per pool; the discrete `0.5 * Δsqrt² * L / sqrt` approximation is the standard practical substitute. Both forms agree in the high-arbitrage limit.
- **`raw_position_value - hold_only_usd` for IL** (rather than including fees). Earlier semantics mixed accumulated fees into IL, which made fee-vs-IL trade-offs invisible. Corrected in commit 391eadd ("IL semantic corrected (raw position − hold-only, no fees mixed in)").
- **Defensive `position_L ≤ active_L` clamp** in `math::fees::fee_share`. Real V3 mainnet pools enforce this structurally (the position is part of active liquidity). Synthetic data can violate it; the clamp prevents pathological fee-share > 1. Commit 391eadd.

## Obsolete / No Longer Relevant

- None.

## Cross-references

- Caller: `commands::lp::run_lp_backtest`, `strategies::GridRunner` (per cell), `headline::HeadlineRunner` (per month), `validation::*` (replays through engine).
- Consumer of: [storage](storage.md), [math](math.md).
- Producer for: `position_runs` + `equity_curve_points` in [storage](storage.md).
- Related research: `references/v3-lp-profitability-literature.md`, `references/v3-position-validation-methodology.md`, `references/lp-rebalancing-strategies.md`, `references/backtest-statistical-methodology.md`.
- Related notes: `notes/round-trip-fee-math.md`, `notes/idempotent-runs.md`.
