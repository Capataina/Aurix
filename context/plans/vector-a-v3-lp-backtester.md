---
title: "Vector A — Uniswap V3 LP Backtester"
status: proposed
created: 2026-05-01
vector: A
hiring-audience: quant LP desks, Uniswap Labs, DeFi-aware trading firms
estimated-effort: 4-6 weeks focused
depends-on: persistence layer (M2.0 — also unblocks Tab 1 Gap 1)
---

# Vector A — Uniswap V3 LP Backtester

## Goal

Implement an off-chain Uniswap V3 liquidity-provision backtester with exact Q64.96 fixed-point math, validated against on-chain reference positions, with multi-strategy comparison and statistical analysis. This is Tab 2 in the README, executed at the depth that turns "I built Tab 2" into "I implemented and validated V3 tick math against N on-chain positions with per-swap fee distribution within $X tolerance."

## Why This Vector

- **Closes the resume credibility gap.** The Aurix resume bullet explicitly promises "Uniswap V3 LP backtesting" — currently vapor. Shipping this makes the bullet honest.
- **Defends the AMM Mathematics skill claim.** Currently rests on the V3 sqrtPriceX96 decode in `dex/uniswap_v3.rs`. A working backtester with on-chain validation turns a one-line claim into a defended position.
- **Hiring signal.** Quant LP desks (Wintermute, GSR, Cumberland) and DeFi protocols (Uniswap Labs, Aave) all run internal tools shaped exactly like this. Existing OSS implementations (e.g. `uniswap-python`, various GitHub one-offs) are mostly approximations — getting the per-swap fee distribution right is where most attempts fail. Implementing it correctly is portable signal.
- **Compounds with later vectors.** A correct V3 backtester is also infrastructure for Tab 5 (risk modelling on LP strategies) and a piece a Vector C ML model could ingest as features.

## Architecture

```
                ┌─────────────────────────────────┐
                │  React frontend                 │
                │  Tab 2: LP Backtester           │
                │  · range selector               │
                │  · equity curve                 │
                │  · strategy comparison heatmap  │
                └────────────────┬────────────────┘
                                 │ IPC
                ┌────────────────▼────────────────┐
                │  Rust backend                   │
                │                                 │
                │  ┌──────────────────────────┐   │
                │  │ Strategy comparison      │   │
                │  │ (Sharpe, DD, fee/IL    ) │   │
                │  └────────────┬─────────────┘   │
                │  ┌────────────▼─────────────┐   │
                │  │ Position simulation      │   │
                │  │ engine                   │   │
                │  └────────────┬─────────────┘   │
                │  ┌────────────▼─────────────┐   │
                │  │ V3 math primitives       │   │
                │  │ (Q64.96 fixed-point)     │   │
                │  └────────────┬─────────────┘   │
                │  ┌────────────▼─────────────┐   │
                │  │ SQLite store             │   │
                │  │ (swap events, positions) │   │
                │  └────────────┬─────────────┘   │
                └───────────────┼─────────────────┘
                                │ eth_getLogs
                ┌───────────────▼─────────────────┐
                │  Ethereum archive node          │
                │  (V3 Swap event topic)          │
                └─────────────────────────────────┘
```

Five layers, each independently testable:
1. **Storage** — SQLite for swap events, position runs, strategy comparison results
2. **Math primitives** — Q64.96 arithmetic, tick ↔ price ↔ liquidity conversions
3. **Simulation engine** — replay historical swaps against a position, compute fees + IL
4. **Validation harness** — replay known on-chain positions, match within tolerance
5. **Strategy comparison** — grid search over ranges, statistical metrics

## Milestones

### M2.0 — Persistence layer (prerequisite, also closes Tab 1 Gap 1)

- [ ] SQLite schema designed for: swap events (V3), price snapshots (Tab 1), opportunity log, position simulation runs, strategy comparison results
- [ ] `rusqlite` (or `sqlx` for async) integrated into `src-tauri/src/storage/`
- [ ] Tauri commands: `store_snapshot`, `query_snapshots_range`, `store_swap_event`, `query_swaps_for_pool_range`, etc.
- [ ] WAL mode enabled, separate read/write connections per the Image Browser pattern (resume bullet)
- [ ] Migration framework (versioned schema, forward-only migrations)
- [ ] Backfill: persist Tab 1's existing in-memory window on first run

### M2.1 — Historical data ingestion

- [ ] Batched `eth_getLogs` fetcher for V3 `Swap(address,address,int256,int256,uint160,uint128,int24)` events
- [ ] Topic filtering by pool address (start with the WETH/USDC 5bps pool: `0x88e6...5640`)
- [ ] Reorg-safe ingestion: only ingest blocks confirmed at depth ≥ 12
- [ ] Backfill ≥ 30 days of WETH/USDC swaps (~100k+ events, ~50MB SQLite)
- [ ] Idempotency: re-running ingestion never duplicates events
- [ ] Rate-limit-aware: respect free-tier RPC quotas, batch requests

### M2.2 — Q64.96 math primitives

- [ ] `tick_to_sqrt_price_x96(tick: i32) -> U256` — exact, per V3 whitepaper §6.2.2
- [ ] `sqrt_price_x96_to_tick(sqrt_price: U256) -> i32` — inverse, with the documented edge cases
- [ ] `liquidity_for_amounts(sqrt_lower, sqrt_upper, sqrt_current, amount0, amount1) -> u128`
- [ ] `amounts_for_liquidity(sqrt_lower, sqrt_upper, sqrt_current, liquidity) -> (amount0, amount1)`
- [ ] Position fee growth tracking via `feeGrowthInside` snapshots
- [ ] Reference-output regression tests: every primitive validated against fixtures from V3 whitepaper section examples + at least 3 on-chain reference values per function

### M2.3 — Position simulation engine

- [ ] Given a position (lower tick, upper tick, liquidity, entry block), walk every swap in `[entry_block, exit_block]`
- [ ] Per swap: was the position in range? If yes, compute its share of total in-range liquidity at that moment
- [ ] Accumulate fees per swap, in token0 and token1
- [ ] Track impermanent loss: position value at current `sqrt_price` vs hold-only baseline at entry composition
- [ ] Output: equity curve (timestamp, position_value_usd, fees_accumulated_usd, il_usd, hold_only_value_usd)

### M2.4 — Validation harness

- [ ] Identify 5 known LP positions from on-chain (mint tx hash + burn tx hash + collected fees publicly verifiable)
- [ ] Run each through the simulation engine using the actual entry/exit blocks
- [ ] Compare engine output to on-chain reality:
  - Total fees collected within $X / 0.5% tolerance
  - Final position composition (token amounts) matches within rounding
  - IL number documented (no on-chain ground truth for IL itself, but sanity-check against hold-only)
- [ ] Document any discrepancies and their causes (rounding, missed events, missed liquidity changes)
- [ ] Acceptance criterion: 4 of 5 positions match within tolerance

### M2.5 — Multi-strategy comparison

- [ ] Configurable grid: N price ranges × M deposit amounts × P time periods
- [ ] Per cell: total fees ($), total IL ($), net return vs hold-only ($), time-in-range %, max drawdown %
- [ ] Sharpe ratio with 0% risk-free baseline (or USDC-yield baseline if available)
- [ ] Output as queryable SQLite table + heatmap visualisation
- [ ] Sort and filter: "show me top 10 strategies by Sharpe over the last 90 days"

### M2.6 — Frontend integration

- [ ] Tab shell in `App.tsx` (also covers Gap 9) — 5 tab slots, only Tab 1 + Tab 2 active initially
- [ ] LP Backtester tab with:
  - Strategy selector (pool, range, deposit, period)
  - Equity curve chart (position value, fees, IL, hold-only baseline overlaid)
  - Comparison heatmap: range × deposit grid coloured by Sharpe
  - Top-strategies table sortable by any metric
- [ ] Loading states (backtests over 30 days take seconds; show progress)
- [ ] Export: CSV of any backtest run for downstream Python/R analysis

## Validation Strategy

Three layers of validation, each falsifiable:

| Layer | Method | Acceptance |
|---|---|---|
| Math primitives | Reference outputs from V3 whitepaper + on-chain values | Every primitive matches reference within float-precision rounding |
| Simulation engine | Replay 5 known on-chain LP positions | 4 of 5 match collected fees within $X / 0.5% tolerance |
| Strategy comparison | Sanity checks (wider ranges → less IL but lower fee density; tighter ranges → opposite) | Qualitative monotonic relationships hold across grid |

## Open Decisions

- **Historical data source:** archive node directly via `eth_getLogs` (slower, free, real-data-from-source) vs Alchemy historical API (faster, free tier limits, easier setup). Recommendation: start with Alchemy free tier for prototyping, switch to archive node for the final "I ingested raw chain data" hiring story.
- **Math implementation:** roll our own Q64.96 from `num-bigint` (the resume bullet explicitly says "no ethers-rs" — keep it consistent) vs use `alloy-primitives` (which has `U160`/`U256` but is a much smaller dependency than `ethers-rs`). Recommendation: roll our own; the resume signal is worth the implementation cost.
- **Strategy comparison output:** persist every backtest run to SQLite for re-querying (scales to thousands of runs) vs recompute on demand (simpler, slower). Recommendation: persist — enables meta-analysis like "which strategies survived the March 2026 vol spike?"
- **Pool universe:** WETH/USDC 5bps only (matches Tab 1) vs all four WETH/USDC fee tiers (5bps, 30bps, 100bps, 1% if exists). Recommendation: ship with 5bps; add the others as configuration in M2.5.

## Dependencies / Blocked-by

- Status Decision must be "revive" before any code work begins
- M2.0 (persistence) is the prerequisite for everything downstream — also unblocks Tab 1 Gap 1 as a side effect
- M2.6 frontend depends on the tab shell (Gap 9) — tab shell can ship in M2.6 first half
- Independent of Vectors B and C (no shared math; could share storage layer)

## Out of Scope

- Live LP position tracking for currently-open positions (that's Tab 3 territory)
- Multi-chain (mainnet WETH/USDC only for V1)
- Multi-pool fan-out within a single backtest (one pool at a time)
- Real-time fee accumulation for active LPs (this is historical replay only)
- Optimisation algorithms over the strategy grid (no gradient-based search; just grid sampling)
- Solidity-side analysis (no contract reads beyond swap events; no `feeGrowthGlobal` reading off the live contract)

## Hiring Signal Payoff

Resume bullet upgrade options once shipped:

- "Implemented and validated Uniswap V3 LP backtester with exact Q64.96 tick math against 5 on-chain reference positions; per-swap fee distribution within 0.5% of collected ground truth across 30+ days of mainnet data."
- "Designed strategy-comparison framework over 50+ LP configurations with Sharpe / drawdown / fee-IL decomposition, identifying [N] non-obvious patterns in optimal range selection across volatility regimes."

Interview talking points:
- "Most OSS V3 implementations approximate fee distribution at the block-aggregate level. I implemented per-swap distribution because [specific case where the approximation breaks]."
- "I validated against 5 known positions before trusting the engine output."
- "Here's a bug I found in [popular library X] while building my validation harness."
