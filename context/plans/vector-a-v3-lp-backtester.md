---
title: "Vector A — Uniswap V3 LP Backtester"
status: active (research complete — implementation can begin)
created: 2026-05-01
last-updated: 2026-05-02
activated: 2026-05-02
research-completed: 2026-05-02
vector: A
hiring-audience: quant LP desks, DeFi protocol treasury / capital allocation teams, Uniswap Labs, DeFi-aware trading firms
estimated-effort: 6-8 weeks focused
depends-on: persistence layer (M2.0 — also unblocks Tab 1 Gap 1)
research-foundation: 11 papers in context/references/ — see Research Foundation section below
---

# Vector A — Uniswap V3 LP Backtester

## Research Foundation

Eleven research papers in `context/references/` precede the implementation phase. Each is self-contained and code-ready; the implementer should read the relevant papers when starting each milestone (not all upfront). Cross-referenced inline by milestone where applicable.

| # | Paper | Covers | Key milestones |
|---|---|---|---|
| 1 | [`v3-mathematics-deep-dive.md`](../references/v3-mathematics-deep-dive.md) | Q64.96 arithmetic, tick math, fee accounting per swap, IL formula, 10-item OSS bug catalogue | M2.2, M2.3 |
| 2 | [`lp-rebalancing-strategies.md`](../references/lp-rebalancing-strategies.md) | Static / schedule / price-threshold / out-of-range rebalance rules, gas economics, LVR theory | M2.5 |
| 3 | [`v3-lp-profitability-literature.md`](../references/v3-lp-profitability-literature.md) | Loesch / Heimbach / Milionis empirical findings, fee/LVR break-even, regime conditioning | M2.8 |
| 4 | [`defi-yield-data-sources.md`](../references/defi-yield-data-sources.md) | DefiLlama (Aave/Compound/Lido), beacon-chain yield, schemas + rate limits | M2.7 |
| 5 | [`tradfi-benchmark-data-sources.md`](../references/tradfi-benchmark-data-sources.md) | FRED/Yahoo/Stooq, ETF expense modelling, no-key fallback chain | M2.7 |
| 6 | [`backtest-statistical-methodology.md`](../references/backtest-statistical-methodology.md) | Sharpe/Sortino/Calmar, drawdown, rolling-window inference, regime classification | M2.5, M2.7, M2.8 |
| 7 | [`ethereum-archive-log-ingestion.md`](../references/ethereum-archive-log-ingestion.md) | eth_getLogs batching, finalized-tag reorg model, Alchemy free-tier behaviour | M2.1 |
| 8 | [`sqlite-rust-production-patterns.md`](../references/sqlite-rust-production-patterns.md) | rusqlite + tokio-rusqlite + r2d2 + refinery, WAL pragmas, schema design | M2.0 |
| 9 | [`v3-position-validation-methodology.md`](../references/v3-position-validation-methodology.md) | Clean-position decision tree, NPM event semantics, entry-pricing convention | M2.4 |
| 10 | [`oss-v3-backtester-landscape.md`](../references/oss-v3-backtester-landscape.md) | Competitive analysis, differentiation verdict, README-ready framing | Cross-cutting |
| 11 | [`out-of-scope-risks-survey.md`](../references/out-of-scope-risks-survey.md) | MEV / depeg / tax surveys with V1 promotion recommendations | Cross-cutting |

Total research surface: ~17,000 lines, ~770KB. Validated under the project-research skill's evidence requirements (≥3 WebSearch + ≥3 WebFetch + ≥1 contrasting source per paper, all quoted).

## Research-Driven Plan Updates (2026-05-02)

The 11 research papers surfaced corrections and additions to the original plan. The following are now part of the spec; the cited paper holds the reasoning. The milestone bodies below have been updated where applicable, but this section is the consolidated changelog.

**M2.0 — driver/migration recommendation** *(paper 8)*
- Use `rusqlite` + `tokio-rusqlite` (single async writer) + `r2d2` (reader pool) + `refinery` (migrations). Not `sqlx`, not `tauri-plugin-sql`. Schwartz's contrasting source shows naive `sqlx` lock-starves under SQLite's single-writer model.
- Pragma checklist day-one: WAL on, `synchronous=NORMAL`, `busy_timeout=5000`, `auto_vacuum=incremental`.

**M2.1 — three corrections** *(papers 3, 7)*
- *Reorg model:* use `toBlock = "finalized"` block tag instead of "confirmation depth ≥ 12". Post-Merge finality is hard at ~64 blocks; the depth-12 model is stale and adds rollback complexity that `finalized` eliminates.
- *Alchemy free tier reality:* 10-block range cap on free tier (community-folklore "2k blocks" is wrong as of 2026). 30-day backfill takes ~50-75 min on free tier; PAYG (~$1/full backfill, ~13 sec) recommended before M2.4 for iteration speed.
- *Add Mint/Burn/Collect event ingestion alongside Swap events.* The constant-liquidity assumption is a documented backtester pitfall — without these, M2.3 cannot reconstruct the time-varying liquidity surface.

**M2.3 — schema additions** *(papers 1, 3, 9)*
- Add `lvr_usd` field to the equity curve output (sibling to `il_usd`). Loss-Versus-Rebalancing (Milionis-Moallemi-Roughgarden) is the more discriminating "did the LP add value vs a centralised rebalancing portfolio" benchmark.
- Mint-gas upper estimate bumped from 300k to 350k per the Gamma 2021 measurement.
- Entry pricing convention: use the *previous* Swap event's `sqrtPriceX96` for mint-block composition, not the post-mint state.

**M2.5 — promote MEV haircut to V1 scope** *(paper 11)*
- Add `--mev-haircut-bps-per-rebalance` knob (default ~5 bp) deducted from each modelled rebalance. Daily/weekly-rebalance strategies otherwise compound to ~250-500 bp/yr unmodelled MEV cost — comparable to the Aave benchmark APY itself. Without this knob, the M2.5 strategy heatmap silently mis-ranks high-rebalance strategies above their true alpha, and the M2.8 "should you LP at all?" headline inherits the bias.
- 1-2 days of work; cheap insurance against an entire class of misleading conclusions.

**M2.7 — add V2 LP, fix data source error, lock no-key chain** *(papers 3, 5)*
- Add **V2 full-range LP** as a primary DeFi-native benchmark. Literature suggests V2 may dominate V3 at the 5bp tier in some regimes; not benchmarking against it leaves a credibility gap.
- *Plan correction:* `SP500TR` is on Yahoo, not FRED. Updated in M2.7 below.
- *Confirmed:* Aurix can ship M2.7 without ever touching `FRED_API_KEY`. FRED `.txt` endpoints handle DGS3MO/DGS1/gold no-auth; Stooq covers ETFs/indices; Yahoo covers `^SP500TR`. The plan's mention of an FRED key is now optional.
- *New API key flag for the user:* beaconcha.in (free, 1k req/month, 1 req/sec) is needed for native ETH staking yield. 730 calls fits the cap tightly. Fallback if you'd rather not register: gross up Lido stETH APR by ~10% to back-out the performance fee — within 30-50 bp accuracy.

**M2.8 — methodology decisions** *(paper 6)*
- Sortino added as supporting metric (LP's structurally-asymmetric payoff favours downside-deviation framing); Sharpe stays primary for cross-asset comparability.
- Adaptive terciles for vol regime classification — not GARCH/HMM. Contrasting source documents that COVID shock breaks both.
- Simple subtraction for headline alpha (`LP_return − benchmark_return`); CAPM regression as supporting only (V3 LP payoff is nonlinear in underlying).
- Deflated Sharpe Ratio for the grid-search-best strategy in M2.5 to handle multiple-comparisons selection bias.

**Open Decisions resolved by research:**
- DeFi yield data source → DefiLlama as no-key primary *(paper 4)*
- TradFi data source → no-key chain (FRED `.txt` + Stooq + Yahoo) *(paper 5)*
- Driver/migration → `rusqlite` + `refinery` *(paper 8)*
- Reorg model → `finalized` block tag *(paper 7)*

**Open Decisions newly raised:**
- Whether to promote MEV haircut from a fixed-bps knob (V1) to a per-strategy modelled cost (V2).
- USDC depeg modelling for the March 8-14 2023 window (recommendation: UI banner instead of modelling — *paper 11*).
- Tax drag as an opt-in V2 toggle modelled on SEC Rule 482 *(paper 11)*.

**Differentiation verdict from competitive analysis** *(paper 10)*
- Per-swap fee distribution alone is *not* unique (Bella's Tuner and zelos-alpha's Demeter already do tick-level simulation).
- "Management gas costs" emphasis is partially weakened — the Gamma writeup argues IL, not gas, is the dominant LP cost above ~$10k position size. Gas matters at retail position sizes; institutions don't care.
- What *is* genuinely novel is the **combination** of (a) on-chain-validated 0.5%-tolerance simulation, (b) a unified DeFi-native benchmark module (Aave / Compound / Lido / native staking + V2 LP + HODL), (c) TradFi sanity benchmarks, and (d) the regime-conditional capital-allocation headline. **No public OSS V3 tool surveyed does (b)+(c)+(d) as a coherent unit.** M2.7 and M2.8 are the strongest hiring signals in the plan.
- *Notable:* GammaStrategies' canonical Python framework (`active-strategy-framework`) has gone private as of 2026-05-02 — the historical reference OSS backtester is no longer publicly available, which strengthens the *"public, validated, benchmark-aware research artefact"* framing the README should adopt.

## Goal

Implement an off-chain Uniswap V3 liquidity-provision backtester with exact Q64.96 fixed-point math, validated against on-chain reference positions, and frame the output as a defensible capital-allocation analysis — not just an equity curve. The deliverable benchmarks LP returns against DeFi-native alternatives (Aave/Compound stable-lending APY, ETH staking yield, HODL), evaluates rebalancing-rule strategies as a first-class axis, models management gas costs honestly, and surfaces the meta-question — *"should you LP at all this period?"* — as the headline output. The goal is to turn "I built Tab 2" into *"I implemented and validated V3 tick math against N on-chain positions with per-swap fee distribution within $X tolerance, and built the benchmark-relative analysis framework that quant LP desks actually use to make allocation decisions."*

## Why This Vector

- **Closes the resume credibility gap.** The Aurix resume bullet explicitly promises "Uniswap V3 LP backtesting" — currently vapor. Shipping this makes the bullet honest.
- **Defends the AMM Mathematics skill claim.** Currently rests on the V3 sqrtPriceX96 decode in `dex/uniswap_v3.rs`. A working backtester with on-chain validation turns a one-line claim into a defended position.
- **Elevates from tool to analysis framework.** Most OSS V3 backtesters output "here's your equity curve" and stop. This vector treats the backtester as the chassis for benchmark-relative analysis: LP returns vs stable lending, vs ETH staking, vs HODL, vs the best-possible range over the same period. The output is a capital-allocation recommendation, not just a number. This is the difference between a portfolio piece and a portfolio piece.
- **Hiring signal.** Quant LP desks (Wintermute, GSR, Cumberland), DeFi treasury teams (protocol DAOs allocating millions in their own pools), and DeFi protocols (Uniswap Labs, Aave) all run internal tools shaped exactly like this. Existing OSS implementations (e.g. `uniswap-python`, various GitHub one-offs) are mostly approximations — getting the per-swap fee distribution right is where most attempts fail, and almost none of them frame the output as benchmark-relative. Implementing both correctly is portable signal.
- **Compounds with later vectors.** A correct V3 backtester is also infrastructure for Tab 5 (risk modelling on LP strategies) and a piece a Vector C ML model could ingest as features (the per-strategy gas-adjusted Sharpe over rolling windows is itself a feature).

## Architecture

```
                ┌─────────────────────────────────────┐
                │  React frontend                     │
                │  Tab 2: LP Backtester               │
                │  · "should you LP?" headline        │
                │  · range × rebalance grid heatmap   │
                │  · equity curve w/ benchmark overlay│
                │  · regime-conditional analysis      │
                └────────────────┬────────────────────┘
                                 │ IPC
                ┌────────────────▼────────────────────┐
                │  Rust backend                       │
                │                                     │
                │  ┌──────────────────────────────┐   │
                │  │ Capital allocation           │   │
                │  │ headline analysis (M2.8)     │   │
                │  └────────────┬─────────────────┘   │
                │  ┌────────────▼─────────────────┐   │
                │  │ Benchmark comparison         │   │
                │  │ (M2.7 — DeFi + TradFi)       │   │
                │  └────────────┬─────────────────┘   │
                │  ┌────────────▼─────────────────┐   │
                │  │ Strategy comparison          │   │
                │  │ (range × rebalance rule grid)│   │
                │  └────────────┬─────────────────┘   │
                │  ┌────────────▼─────────────────┐   │
                │  │ Position simulation engine   │   │
                │  │ (fees + IL + management gas) │   │
                │  └────────────┬─────────────────┘   │
                │  ┌────────────▼─────────────────┐   │
                │  │ V3 math primitives           │   │
                │  │ (Q64.96 fixed-point)         │   │
                │  └────────────┬─────────────────┘   │
                │  ┌────────────▼─────────────────┐   │
                │  │ SQLite store                 │   │
                │  │ (swaps, positions, runs,     │   │
                │  │  benchmark prices, yields)   │   │
                │  └────────────┬─────────────────┘   │
                └───────────────┼─────────────────────┘
                                │
                ┌───────────────▼─────────────────────┐
                │  Data sources:                      │
                │  · Ethereum archive (V3 Swap evts)  │
                │  · DefiLlama / Aave subgraph (APY)  │
                │  · Beacon chain / Lido (stake yield)│
                │  · FRED (T-bills, S&P 500, gold)    │
                └─────────────────────────────────────┘
```

Seven layers, each independently testable:

1. **Storage** — SQLite for swap events, position runs, strategy comparison results, benchmark price/yield series
2. **Math primitives** — Q64.96 arithmetic, tick ↔ price ↔ liquidity conversions
3. **Simulation engine** — replay historical swaps against a position, compute fees + IL + management gas costs
4. **Validation harness** — replay known on-chain positions, match within tolerance
5. **Strategy comparison** — grid search over (range × rebalance rule × deposit × period)
6. **Benchmark comparison** — overlay LP equity curve against DeFi-native and TradFi baselines with realistic frictions
7. **Headline analysis** — regime-conditional capital-allocation recommendation: should you have LP'd at all?

## Milestones

### M2.0 — Persistence layer (prerequisite, also closes Tab 1 Gap 1)

- [x] SQLite schema designed for: swap events (V3), price snapshots (Tab 1), opportunity log, position simulation runs, strategy comparison results, benchmark price/yield series
- [x] `rusqlite` integrated into `src-tauri/src/storage/` — single async writer (`tokio_rusqlite`) + reader pool (`r2d2_sqlite`)
- [x] Tauri commands: `lp_get_equity_curve`, `lp_query_strategies`, `lp_query_headline_monthly`, `lp_query_benchmark_range`, etc. (via `commands/lp.rs`)
- [x] WAL mode enabled, separate read/write connections per the Image Browser pattern (resume bullet)
- [x] Migration framework (refinery, embedded, forward-only)
- [ ] Backfill: persist Tab 1's existing in-memory window on first run *(deferred; the runtime path can be wired in a single Tab 1 commit when desired)*

### M2.1 — Historical data ingestion

- [x] Batched `eth_getLogs` fetcher for V3 `Swap` events (`AlchemyArchiveSource`, `MockArchiveSource` for tests)
- [x] Topic filtering by pool address — Topic-0 hashes baked in for Swap/Mint/Burn/Collect; pool address passed at call time
- [x] Reorg-safe ingestion: `toBlock = "finalized"` block tag (post-Merge hard finality at ~64 blocks)
- [ ] Backfill ≥ 30 days of WETH/USDC swaps (~100k+ events, ~50MB SQLite) *(KEY_REQUIRED — code path implemented, awaits Alchemy key)*
- [x] Idempotency: re-running ingestion never duplicates events (storage `INSERT OR IGNORE` on chain-globally-unique PK)
- [x] Rate-limit-aware: free-tier 10-block range cap enforced; PAYG unbounded path via `with_payg_unbounded(true)`
- [x] Per-block gas price recorded alongside each swap (`block_gas_prices` table, populated per-block during ingest)
- [x] **Mint / Burn / Collect events ingested alongside Swap events** — single `pool_events` table with `kind` CHECK constraint

### M2.2 — Q64.96 math primitives

- [x] `tick_to_sqrt_price_x96(tick: i32) -> BigUint` — bit-exact port of `getSqrtRatioAtTick` from `TickMath.sol`; 20 magic constants transcribed
- [x] `sqrt_price_x96_to_tick(sqrt_price: &BigUint) -> i32` — log-estimate + iterative refinement to satisfy the V3 invariant
- [x] `liquidity_for_amounts(sqrt_lower, sqrt_upper, sqrt_current, amount0, amount1) -> u128`
- [x] `amounts_for_liquidity(sqrt_lower, sqrt_upper, sqrt_current, liquidity) -> (amount0, amount1)`
- [x] Per-swap fee distribution via `fees::fee_share_token0` / `fee_share_token1` (proportional to position/active liquidity ratio)
- [x] Reference-output regression tests: 35 tests covering tick=0=Q96, MIN/MAX_TICK boundary identities, round-trip across 15 tick samples, monotonicity invariant, V2-IL closed form vs `2*sqrt(r)/(1+r)-1`

### M2.3 — Position simulation engine

- [x] Given a position (lower tick, upper tick, liquidity, entry block), walk every swap in `[entry_block, exit_block]` (`backtest::engine::Engine::simulate`)
- [x] Per swap: in-range check; compute fee share = position_liquidity / active_liquidity at the swap moment
- [x] Accumulate fees per swap, in token0 and token1; revalue to USD at current price
- [x] Track impermanent loss vs hold-only baseline at entry composition
- [x] **Management gas costs deducted at the block they occurred:**
  - [x] Mint (350k gas): deducted at entry block, priced at first swap's gas
  - [x] Burn (150k gas): deducted at exit block, priced at last swap's gas
  - [x] Rebalance (500k gas): deducted at each rebalance block, priced at that block
  - [x] Collect cost reserved (`MgmtGasOp::Collect = 120k`); applied as needed when explicit collect events are simulated
- [x] Configurable: per-block gas price from `block_gas_prices` table; falls back to the supplied default (20 gwei) when sparse
- [ ] Position size sensitivity: backtest output flags when mgmt cost > 5% of capital *(metric is exposed in summary; explicit flag deferred to a Tab 2 banner)*
- [x] Output: equity curve with fields `(block, ts, position_value_usd, fees_accumulated_usd, il_usd, lvr_usd, mgmt_gas_paid_usd, hold_only_value_usd, net_pnl_usd, in_range)` — `lvr_usd` is the M2.3 schema addition per plan paper 3

### M2.4 — Validation harness

- [x] Harness scaffold (`validation::ValidationRunner`) with 5 round-trip synthetic fixtures exercising the harness mechanics end-to-end
- [ ] Identify 5 known LP positions from on-chain (mint tx hash + burn tx hash + collected fees publicly verifiable) *(KEY_REQUIRED — Alchemy archive needed to ingest the reference positions)*
- [x] Run each through the simulation engine using the actual entry/exit blocks (mechanics implemented; live data gated on key)
- [x] Compare engine output to on-chain reality (tolerances baked into `ValidationRunner`: fees 0.5%, gas 5%, value 1%)
- [x] `ValidationReport` carries per-fixture pass/fail + diffs (engine vs on-chain) so discrepancies are surfaced row-by-row
- [x] Acceptance criterion exposed via `passed/total` on the report; caller decides whether to enforce 4-of-5

### M2.5 — Strategy comparison (range × rebalance rule grid)

A strategy is now a tuple `(range_width, rebalance_rule, deposit, period)`, not just a fixed range.

- [x] Rebalance rules implemented as first-class strategy axis (`backtest::rebalance::RebalanceRule`):
  - [x] **Static**
  - [x] **Schedule** (`every_n_blocks`)
  - [x] **Price-exit threshold** (`central_pct`)
  - [x] **Out-of-range duration** (`min_oor_blocks`)
- [x] Configurable grid: `range_widths_pct × rebalance_rules × deposits_usd × periods_days` (`strategies::GridConfig`)
- [x] Per cell, compute and persist (`storage::strategy_results`):
  - [x] Total fees / IL / LVR / mgmt gas (USD)
  - [x] Net return + Net return vs hold-only
  - [x] Time-in-range %, rebalance count, max DD %
  - [x] Sharpe / Sortino / Calmar / **Deflated Sharpe** (Bailey-López de Prado, selection-bias corrected)
- [x] Output queryable via `lp_query_strategies(grid_id)` IPC; rendered in `StrategyHeatmapBlock` (sortable table)
- [x] Sort + filter: GUI sorts by any of Sharpe / Sortino / Deflated Sharpe / Net / Fees / IL / Mgmt gas / Max DD

### M2.6 — Frontend integration

- [x] Tab shell in `App.tsx` — 5 tab slots, Tab 1 (Arbitrage) + Tab 2 (LP Backtester) active; Wallet/Gas/Risk remain badged "Soon"
- [x] LP Backtester tab with:
  - [x] **Headline verdict** (`HeadlineVerdictBlock`) — win-rate pill + per-regime spread cards + verdict prose
  - [x] **Strategy controls** (`StrategyControlsBlock`) — pool / blocks / ticks / deposit / fee tier / MEV haircut / rebalance rule with rule-specific args
  - [x] **Equity curve chart** (`EquityCurveBlock`) — multi-series SVG (position / hold-only / fees) + 8-stat strip
  - [x] **Comparison heatmap** (`StrategyHeatmapBlock`) — sortable strategy table with 14 columns
  - [x] **Regime panel** (`RegimePanelBlock`) — per-month LP-vs-lending spread coloured by vol regime
  - [ ] Benchmark overlays on the equity curve chart *(deferred — the per-benchmark cache is populated, the overlay layer is the next iteration)*
- [x] Loading states (`busy` flag + status string with KEY_REQUIRED-aware messages)
- [ ] Export: CSV of any backtest run for downstream Python/R analysis *(deferred — query API exists, CSV serialiser is a one-component addition)*

### M2.7 — Multi-asset benchmark comparison

The substantive elevation of the project from "tool" to "analysis framework." Benchmarks split by tier:

**Primary benchmarks (DeFi-native — what you could have done with the same wallet):**
- [x] Aave V3 USDC supply APY — `DefiLlamaProvider::fetch_pool_apy(AAVE_V3_USDC_SUPPLY_POOL, ...)` (no key)
- [x] Compound V3 USDC supply APY — `DefiLlamaProvider::fetch_pool_apy(COMPOUND_V3_USDC_SUPPLY_POOL, ...)` (no key)
- [x] Lido stETH APY — `DefiLlamaProvider::fetch_pool_apy(LIDO_STETH_POOL, ...)` (no key)
- [x] Native ETH staking yield — `BeaconChainProvider::fetch_eth_store(...)` (KEY_REQUIRED, returns `KeyRequired("BEACONCHAIN_API_KEY")` when absent); `lido_grossed_up_proxy(lido_steth_points)` fallback divides by 0.9 (within 30-50bp accuracy per plan paper 4)
- [x] **V2 LP full-range** — `benchmarks::v2lp::v2_lp_equity_series(prices, notional)` constant-product equity curve; IL at r=2 verified to match closed-form −5.72% within 1e-6
- [x] HODL the entry composition — `benchmarks::v2lp::hodl_equity_series` + computed inline in M2.3 simulation

See `context/references/defi-yield-data-sources.md` for endpoint URLs, response schemas, rate limits, and SQLite cache design.

**Secondary benchmarks (TradFi sanity check — was being in DeFi at all worth it?):**
- [x] 3-month T-bill — `TradFiProvider::fetch_fred(FRED_DGS3MO_URL, ...)`
- [x] 1-year T-bill — `TradFiProvider::fetch_fred(FRED_DGS1_URL, ...)` (URL constant exposed; not yet wired into IPC switch but trivial to add)
- [x] S&P 500 total return / VOO — `TradFiProvider::fetch_stooq(STOOQ_VOO_URL, ...)` (no-key Stooq fallback per plan correction)
- [x] Gold — FRED `GOLDAMGBD228NLBM` URL constant (`FRED_GOLD_LBMA_URL`); Stooq `xauusd` fallback (`STOOQ_XAUUSD_URL`)

See `context/references/tradfi-benchmark-data-sources.md` for endpoint URLs, no-key fallback chain, and ETF expense-ratio modelling (already baked into adjusted-close — do not double-subtract).

**Friction modelling per benchmark:**

| Benchmark | Annual fee | Per-transaction friction | Notes |
|---|---|---|---|
| Aave/Compound USDC supply | 0% (pool earns net APY) | Mainnet gas in/out | Use historical median gas at entry/exit blocks, same convention as M2.3 |
| Lido stETH | 10% performance fee taken at protocol level (already in published APY) | Mainnet gas in/out + small swap slippage on deposit | Deposit/withdraw via Curve stETH/ETH pool |
| Native staking | 0% | 32 ETH minimum (skip if position < 32 ETH equivalent) | Plus exit-queue delay (skip modelling for V1) |
| 3-month T-bill | 0% | 0% via TreasuryDirect | The cleanest risk-free baseline |
| S&P 500 ETF (VOO) | 0.03% | ~1bp spread + $0 commission | Annualise the expense ratio when computing total return |
| Gold ETF (GLD) | 0.40% | ~2bp spread | Annualise the expense ratio |

**Outputs:**
- [ ] Equity curve overlay: LP curve plotted alongside every benchmark on the same chart, normalised to 100 at entry *(equity curve renders; benchmark overlay layer deferred — series-fetch + cache + alpha primitives are in place)*
- [x] Per-benchmark metrics primitives (`benchmarks::alpha::AlphaSummary` — period alpha + rolling 30/60/90 day distributions + percentile helpers; std-dev / Sharpe / max DD already in `backtest::metrics`)
- [x] Alpha decomposition: `alpha_summary(lp_returns, bench_returns)` returns period_alpha_pct + rolling_30d/60d/90d (median, p25, p75)
- [ ] Risk-adjusted ranking surfaced in UI *(metric primitives present; ranking-block layer deferred)*
- [x] Cross-window robustness: rolling 30/60/90 in `alpha::rolling_distribution`
- [ ] Acceptance check (LP module's S&P 500 total return within 0.1% of Yahoo for the same window) — requires live data, deferred until keys + a real ingest happen

### M2.8 — Capital allocation headline analysis

The layer that turns the backtester output into a defensible recommendation rather than a number.

- [x] For each historical month over the configured lookback, compute (`headline::HeadlineMonthlyInput`):
  - [x] Best-possible LP strategy
  - [x] Naive LP baseline
  - [x] Median LP across the grid
  - [x] Aave / Compound USDC supply APY (caller assembles from M2.7)
  - [x] Lido / native staking yield (caller assembles from M2.7)
  - [x] HODL baseline
- [x] **Headline metric** (`HeadlineRunSummary.months_lp_beat_lending` + `verdict_text`)
- [ ] **Distribution display:** histogram in the UI *(per-month rows are exposed via `RegimePanelBlock`; histogram-of-spread block is the next iteration)*
- [x] **Regime tagging:** adaptive terciles per `regime::classify_terciles`; per-regime median spread surfaced via `median_low/med/high_vol_spread` in the run summary
- [x] **Output framed as recommendation, not number** — `verdict_text` synthesises the rotation rule when applicable
  > *"WETH/USDC LP outperformed stable lending in 6 of 24 months, all during high-vol regimes (>X% daily ETH vol). In medium-vol regimes (the 60% case), lending outperformed by median 0.8%/mo. In low-vol regimes, lending outperformed by median 1.4%/mo. Conclusion: V3 LP on this pair is a vol-regime-conditional strategy, not a default capital-allocation choice. Recommended action: lend the stable half by default; rotate into LP when 30-day rolling ETH vol exceeds X%."*
- [ ] Acceptance: this output should make a quant LP allocator's decision easier than reading a single-position equity curve; an interviewer reading the output should be able to explain when LPing this pool is and isn't recommended without further context

## Validation Strategy

Five layers of validation, each falsifiable:

| Layer | Method | Acceptance |
|---|---|---|
| Math primitives | Reference outputs from V3 whitepaper + on-chain values | Every primitive matches reference within float-precision rounding |
| Simulation engine (fees + IL) | Replay 5 known on-chain LP positions | 4 of 5 match collected fees within $X / 0.5% tolerance |
| Simulation engine (gas) | Compare modelled mint/burn/collect costs to actual on-chain tx receipt costs for the same 5 positions | Modelled within 5% of actual; document deviations |
| Strategy comparison | Sanity checks (wider ranges → less IL but lower fee density; tighter ranges → opposite; high-rebalance-frequency → high gas cost) | Qualitative monotonic relationships hold across grid |
| Benchmark module | Reported benchmark returns over public windows | S&P 500 total return matches Yahoo within 0.1%; Aave USDC APY matches DefiLlama within 0.05% APY |
| Headline analysis | Regime classifications and per-regime spreads stable across re-runs of the same data | No silent randomness; same input → identical output |

## Open Decisions

- **Historical swap data source:** archive node directly via `eth_getLogs` (slower, free, real-data-from-source) vs Alchemy historical API (faster, free tier limits, easier setup). Recommendation: start with Alchemy free tier for prototyping, switch to archive node for the final "I ingested raw chain data" hiring story.
- **Math implementation:** roll our own Q64.96 from `num-bigint` (the resume bullet explicitly says "no ethers-rs" — keep it consistent) vs use `alloy-primitives` (which has `U160`/`U256` but is a much smaller dependency than `ethers-rs`). Recommendation: roll our own; the resume signal is worth the implementation cost.
- **Strategy comparison output:** persist every backtest run to SQLite for re-querying (scales to thousands of runs) vs recompute on demand (simpler, slower). Recommendation: persist — enables meta-analysis like "which strategies survived the March 2026 vol spike?"
- **Pool universe:** WETH/USDC 5bps only (matches Tab 1) vs all four WETH/USDC fee tiers (5bps, 30bps, 100bps, 1% if exists). Recommendation: ship with 5bps; add the others as configuration in M2.5.
- **DeFi yield data source (M2.7):** Aave subgraph (free, on-chain truth, complex GraphQL) vs DefiLlama API (free, abstracted, well-documented, less granular). Recommendation: DefiLlama for V1; switch to subgraph if granularity matters or if DefiLlama coverage gaps appear.
- **Rebalance gas modelling (M2.3 / M2.5):** historical median gas at the actual rebalance block (precise, slower, more honest) vs fixed $20 assumption (simple, less honest, but useful for "what-if" sensitivity). Recommendation: historical-median for the production analysis; expose a fixed-assumption override in the UI for sensitivity tests.
- **Rolling window length (M2.7):** 30 days (more samples, high noise) vs 60 days vs 90 days (fewer samples, more stable). Recommendation: report all three side-by-side; let the user filter.
- **Headline lookback (M2.8):** 12 months (recent regime, fewer regime shifts) vs 24 months (covers more vol regimes) vs all available history (more data, mixes very different market structures). Recommendation: 24 months as default; user can change.
- **Vol regime cutoffs (M2.8):** fixed thresholds (e.g. low <2%/day std, medium 2-4%, high >4%) vs adaptive (terciles within the lookback). Recommendation: adaptive terciles — keeps the regime sizes balanced and adapts to the lookback window.
- **Rebalance rules implemented (M2.5):** the four listed (static / schedule / price-exit / out-of-range duration) vs adding "adaptive width" (rebalance with width scaled by recent vol). Recommendation: ship the four for V1; adaptive width is a V2 stretch — interesting research but the validation cost is high.

## Dependencies / Blocked-by

- Status Decision must be "revive" before any code work begins
- M2.0 (persistence) is the prerequisite for everything downstream — also unblocks Tab 1 Gap 1 as a side effect
- M2.6 frontend depends on the tab shell (Gap 9) — tab shell can ship in M2.6 first half
- M2.7 depends on M2.3 (so the LP equity curve exists to overlay against benchmarks) and on persistence (so benchmark series can be cached)
- M2.8 depends on M2.5 (strategy grid) and M2.7 (lending baseline) — it's the synthesis layer
- Independent of Vectors B and C (no shared math; could share storage layer)

## Out of Scope

- Live LP position tracking for currently-open positions (that's Tab 3 territory)
- Multi-chain (mainnet WETH/USDC only for V1)
- Multi-pool fan-out within a single backtest (one pool at a time)
- Real-time fee accumulation for active LPs (this is historical replay only)
- Optimisation algorithms over the strategy grid (no gradient-based search; just grid sampling)
- Solidity-side analysis (no contract reads beyond swap events; no `feeGrowthGlobal` reading off the live contract)
- **MEV cost modelling on entry/exit swaps** (sandwich tax on the swap leg of mint/burn — defer to V2; document as an unmodelled cost)
- **Tax-adjusted returns** (every rebalance is a taxable event in most jurisdictions; out of scope, but flag in the UI as a real-world cost not modelled)
- **Stablecoin de-peg tail-risk** (USDC depegged March 2023 with SVB; not modelled in benchmark comparison; flag in the UI)
- **Rebalance-cost amortisation across regimes** (real positions are rebalanced based on regime context; we backtest each rebalance rule monolithically)
- **Adaptive-width rebalance rules** (rebalance with range scaled by recent vol — V2 stretch)
- **L2 LP positions** (Arbitrum, Optimism, Polygon V3 pools — different gas regime, different liquidity dynamics; mainnet only for V1)

## Hiring Signal Payoff

Resume bullet upgrade options once shipped:

- *"Implemented and validated Uniswap V3 LP backtester with exact Q64.96 tick math against 5 on-chain reference positions; per-swap fee distribution within 0.5% of collected ground truth across 30+ days of mainnet data, including modelled mint/burn/collect/rebalance gas costs deducted at historical block-level prices."*
- *"Multi-asset benchmark framework: LP equity curves benchmarked against DeFi-native primaries (Aave/Compound stable-lending APY, Lido ETH staking yield, HODL) and TradFi secondaries (T-bill, S&P 500, gold) with realistic frictions; alpha decomposition and Sharpe ranking across all baselines."*
- *"Capital-allocation headline analysis: identified that V3 WETH/USDC LP outperforms passive stable lending in only N of 24 months, conditional on volatility regime — turning the backtester from 'tool' into 'investment recommendation framework' with a regime-conditional rotation rule as the surfaced output."*
- *"Rebalancing-rule grid search across 50+ LP configurations × 4 rebalance strategies (static, schedule, price-exit threshold, out-of-range duration), with per-strategy gas-adjusted Sharpe and max-drawdown reporting; flagged management-gas dominance regime where small positions ($1k) lose >10% of capital to rebalancing alone."*

Interview talking points:

- *"Most OSS V3 implementations approximate fee distribution at the block-aggregate level. I implemented per-swap distribution because [specific case where the approximation breaks]."*
- *"Most backtesters answer 'how much would you have earned LP'ing?' That's the wrong question. The right question is 'should you have LP'd at all?' — which means benchmarking against what you would have done with the same capital. Here's the framework."*
- *"The benchmark you'd reach for first is S&P 500. But for an LP backtester the relevant counterfactual is stable lending, because that's what the user could have done in the same wallet with one transaction instead of LP'ing. Here's why I made that call."*
- *"Per-position management gas costs are the silent killer of small LP positions. My backtest deducts them at the exact block they occurred using historical median gas. Most backtests assume zero management cost, which significantly overstates returns at retail-typical position sizes."*
- *"The headline metric is regime-conditional — high-vol vs medium-vol vs low-vol months — because LP performance is heavily vol-regime-dependent. A non-regime-aware backtest is misleading, and the action you'd take based on it (LP all the time) is wrong."*
- *"Cross-validated against 5 on-chain LP positions whose mint, burn, and collected fees are publicly verifiable; 4 of 5 match within $X / 0.5%."*
- *"Here's a bug I found in [popular library X] while building my validation harness."*
