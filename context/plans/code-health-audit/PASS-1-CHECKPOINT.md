# Pass 1 Checkpoint — Aurix Code Health Audit

**Date:** 2026-05-04
**Audit folder:** `context/plans/code-health-audit/`

## Project shape (broad sweep)

- **Stack:** Tauri 2 desktop app · React 19 + TypeScript 5.8 frontend · Rust edition 2021 backend · SQLite (rusqlite) persistence · pnpm workspace.
- **Source tree:** 142 source files total — 79 Rust files in `src-tauri/src/` (~10.5k lines), 63 TypeScript files in `src/` (~9k lines).
- **30-day activity:** 29 commits across 4 active days, +15,820 / -1,041 LOC. Bursty cadence — 21-commit M2.0 → M2.8 + 4-tier extension sprint on 2026-05-03 surrounded by 11 quiet days.
- **Subsystems shipped this sprint (Vector A):** `storage/` (15 files), `math/` (6 files), `ingest/` (8 files), `backtest/` (6 files), `validation/` (3 files), `strategies/` (2 files), `benchmarks/` (10 files), `headline/` (2 files), `commands/lp.rs` (Tauri IPC). Plus frontend `src/features/lp-backtest/` (8 files) and the `src/components/blocks/` UI restructure.
- **Existing subsystems (Tab 1):** `dex/` (uniswap_v2.rs + uniswap_v3.rs), `ethereum/` (client.rs), `commands/market.rs`, `market/types.rs`, `config/` (env + chains), `src/features/arbitrage/`.
- **Telemetry recorder:** `src/lib/telemetry.ts` (397 lines) + `src-tauri/src/commands/telemetry.rs`. Captures IPC start/end/error + clicks + lifecycle to `~/Library/Logs/com.ataca.aurix/last-session.json`. New this sprint.

## Test-suite baseline

- **Backend (`cd src-tauri && cargo test`):** 139 pass · 0 fail · 3 ignored · 0 measured. Compiles clean, no warnings surfaced. Run-time: 0.41s.
- **Ignored tests (3):** all `ingest::tests::live_alchemy_*` — gated on `#[ignore]` because they need live RPC access. Not failures; the team's idiomatic way to keep live-data tests out of CI.
- **Frontend:** No test script in `package.json`, no `vitest` / `jest` dep, no `tests/` directory under `src/`. Frontend is uncovered. Recorded as Test-Coverage-Gap finding seed (will land in cross-cutting.md).
- **Build:** `cargo build` finishes clean; `tsc --noEmit` blocked by missing global `tsc`. Frontend type checking happens at `vite build` time only.
- **Pre-existing failures recorded as Known Issues:** none — backend suite is fully green.

The `scripts/test_baseline.sh` heuristic emitted "no recognised stack marker" because the audit was run from the repo root; both `Cargo.toml` and `package.json` exist but in non-standard locations relative to the script's expectation. Recorded as a script-level reasoned omission (the test was run directly via `cargo test` from `src-tauri/`).

## Modularisation candidate list (mandatory enumeration)

**Rust candidates (8 files):**

| Path | Lines | Qualifies because |
|------|------:|-------------------|
| `src-tauri/src/commands/lp.rs` | 628 | ≥350 lines, top-decile |
| `src-tauri/src/ingest/subgraph.rs` | 549 | ≥350 lines, top-decile |
| `src-tauri/src/ingest/decoder.rs` | 515 | ≥350 lines, top-decile |
| `src-tauri/src/headline/verdict.rs` | 414 | ≥350 lines, top-decile |
| `src-tauri/src/ingest/mod.rs` | 392 | ≥350 lines, top-decile |
| `src-tauri/src/backtest/engine.rs` | 374 | ≥350 lines, top-decile |
| `src-tauri/src/storage/runs.rs` | 340 | top-decile |
| `src-tauri/src/math/liquidity.rs` | 322 | top-decile |

**TypeScript candidates (7 files):**

| Path | Lines | Qualifies because |
|------|------:|-------------------|
| `src/features/lp-backtest/LpBacktestPage.tsx` | 668 | ≥300 lines, top-decile |
| `src/components/blocks/arbitrage/PriceChartBlock.tsx` | 540 | ≥300 lines, top-decile |
| `src/features/arbitrage/insights.ts` | 490 | ≥300 lines, top-decile |
| `src/features/lp-backtest/LpSettingsForm.tsx` | 409 | ≥300 lines, top-decile |
| `src/lib/telemetry.ts` | 397 | ≥300 lines, top-decile |
| `src/components/blocks/lp/StrategyControlsBlock.tsx` | 360 | ≥300 lines, top-decile |
| `src/components/blocks/lp/MultiAssetCompareBlock.tsx` | 319 | ≥300 lines, top-decile |

**Total: 15 modularisation candidates.** Each gets a per-file verdict (`split-recommended` / `leave-as-is` / `not-applicable`) during Pass 2.

## Prioritisation signals

### Top by composite hotspot score (≥ 0.80 — near-certain Pass-2 targets)

| Rank | Path | Lines | Fan-in | Churn (90d) | Composite |
|-----:|------|------:|-------:|-------:|--:|
| 1 | `src-tauri/src/ingest/mod.rs` | 392 | 8 | 3 | 0.95 |
| 2 | `src-tauri/src/backtest/mod.rs` | 306 | 11 | 3 | 0.93 |
| 3 | `src-tauri/src/storage/swaps.rs` | 288 | 4 | 4 | 0.87 |
| 4 | `src-tauri/src/storage/pool_events.rs` | 284 | 4 | 3 | 0.83 |
| 5 | `src-tauri/src/ethereum/client.rs` | 173 | 6 | 2 | 0.80 |

### Top by fan-in (load-bearing — changes ripple)

| Rank | Path | Fan-in | Note |
|-----:|------|-------:|------|
| 1 | `src-tauri/src/storage/mod.rs` | 26 | Persistence is the central artery; almost every backend module imports from `storage::*`. |
| 2 | `src-tauri/src/math/mod.rs` | 12 | Q64.96 + tick + liquidity + fees + IL primitives are reused across backtest/strategies/headline/validation. |
| 3 | `src-tauri/src/storage/error.rs` | 12 | Storage error type leaks across the backend. |
| 4 | `src-tauri/src/backtest/mod.rs` | 11 | Backtest is the workhorse that strategies, headline, validation all consume. |
| 5 | `src-tauri/src/ingest/mod.rs` | 8 | Archive ingest is the upstream of everything. |

### Top by fan-out (integration hubs — coupling concentrated)

- `commands/lp.rs` (16) — the IPC hub for the LP backtester; orchestrates ingest + backtest + grid + headline + benchmarks.
- `storage/mod.rs` (13) — re-exports the persistence façade.
- `lib.rs` (13) — Tauri command registration; wires everything to the frontend.

## Pass-2 prioritisation (substantive systems list)

These are the systems the audit will deep-dive in Pass 2. Each gets a WebSearch call, code reading, diagnostic test writing where applicable, Data Layout analysis, and a row in the Obligation Evidence Map.

**Tier 1 — math + storage + backtest (largest LOC, highest fan-in, just-shipped, security-relevant):**

1. **Math primitives** — `src-tauri/src/math/{q96,tick,liquidity,fees,il}.rs`. Q64.96 fixed-point, tick ↔ sqrtPriceX96, liquidity↔amounts, per-swap fees, IL closed forms. Already has 30+ unit tests. Research mode 2 (Uniswap V3 SDK reference implementation comparison).
2. **Storage layer** — `src-tauri/src/storage/*.rs` (13 files). SQLite + WAL + migrations + idempotent inserts for swaps / pool_events / runs / strategies / benchmarks / headline / snapshots / gas / state. Fan-in 26. Research mode 1 (production SQLite/Rust patterns) + mode 3 (concurrency anti-patterns).
3. **Backtest engine** — `src-tauri/src/backtest/{engine,mod,price,metrics}.rs`. Position simulation (per-swap fee distribution, in-range tracking, LVR, gas costs, equity curve). Composite 0.93. Research mode 2 (LVR / Milionis et al. specifics).

**Tier 2 — ingest + IPC + strategies + benchmarks:**

4. **Ingest pipeline** — `src-tauri/src/ingest/*.rs` (8 files: alchemy, subgraph, decoder, pipeline, source, mock, mod, error). Archive log fetcher + ABI decoder. Three sources (Alchemy `eth_getLogs`, Uniswap subgraph, synthetic mock). Composite 0.95. Research mode 1 (production archive ingestion patterns) + mode 3 (eth_getLogs anti-patterns).
5. **IPC commands** — `src-tauri/src/commands/{lp,market,telemetry}.rs`. `lp.rs` is the largest single file in the repo at 628 lines, fan-out 16. Tauri command surface for the LP backtester. Research mode 1 (Tauri command-pattern conventions).
6. **Strategies grid** — `src-tauri/src/strategies/{grid,mod}.rs`. Grid search over `range × rule × deposit × period`, Sharpe/Sortino/Deflated Sharpe/max DD. Research mode 2 (deflated Sharpe specifics).
7. **Benchmarks module** — `src-tauri/src/benchmarks/*.rs` (10 files). DefiLlama (Aave/Compound/Lido), FRED + Stooq (T-bills, S&P, gold), beaconcha.in, V2 LP, HODL, alpha decomposition. Research mode 1 (alpha decomposition methodology).

**Tier 3 — headline + dex + ethereum + validation + config:**

8. **Headline (capital allocation)** — `src-tauri/src/headline/{verdict,mod}.rs`. Adaptive-tercile vol regime classifier + per-month best/naive/median LP vs Aave/Lido/HODL + verdict prose synthesis. Research mode 2 (tercile classifier robustness).
9. **DEX adapters + Ethereum client** — `src-tauri/src/dex/{uniswap_v2,uniswap_v3}.rs`, `src-tauri/src/ethereum/client.rs`. Tab 1 live venue readers + JSON-RPC transport. Composite 0.80 on `client.rs`. Research mode 3 (RPC client anti-patterns under retries).
10. **Validation harness** — `src-tauri/src/validation/{mod,synthetic}.rs`. Replay LP-position fixtures, compute fees/gas/value diffs vs ground truth. Research mode 2 (validation methodology — already covered in `context/references/v3-position-validation-methodology.md`).
11. **Config + chains** — `src-tauri/src/config/{mod,chains}.rs`. ChainConfig per chain, env-backed RPC, per-(chain, protocol) subgraph URL routing. Research mode 1 (multi-chain config patterns).

**Tier 4 — frontend (no test coverage; depth limited by no in-test-suite verification path):**

12. **LP backtest frontend** — `src/features/lp-backtest/*` + `src/components/blocks/lp/*` (LpBacktestPage 668, LpSettingsForm 409, StrategyControlsBlock 360, MultiAssetCompareBlock 319, EquityCurveBlock, StrategyHeatmapBlock). Tab 2 dashboard. Research mode 1 (React hot-path patterns).
13. **Arbitrage frontend** — `src/features/arbitrage/*` + `src/components/blocks/arbitrage/*` (ArbitragePage, insights.ts 490, PriceChartBlock.tsx 540, BlockRegistry, ArbRouteBlock). Tab 1 dashboard. Research mode 3 (rolling-window React anti-patterns).
14. **Telemetry + shell** — `src/lib/telemetry.ts` (397) + `src/components/shell/{TopBar,SettingsMenu}.tsx` + `src/App.tsx`. Custom IPC tracer + global UI shell. Research mode 1 (telemetry capture patterns in desktop apps).

**Total: 14 substantive systems → 14 WebSearch obligations + reasoned-omission rows for any system that lands at `not applicable`.**

Trivial systems (no research, no diagnostic tests, recorded as reasoned omissions in the map):
- `src-tauri/src/main.rs` (entrypoint), `src-tauri/src/lib.rs` (Tauri builder + handler list — high fan-out but mechanically thin), `src-tauri/src/market/types.rs` (type definitions only), per-module `error.rs` files (thin `thiserror::Error` enums), per-module `mod.rs` files that are pure re-exports.
- TypeScript: `src/main.tsx` (React mount), `src/App.tsx` shell except for the routing logic (covered under "Telemetry + shell" Tier 4).

## Known issues already surfaced from context files

Pulled from `context/notes/`, `context/architecture.md`, and the LifeOS vault Aurix/_Overview/Gaps:

| # | Issue (already-known, captured here so the audit doesn't double-count) | Source |
|---|---|---|
| K1 | Fail-fast error model in Tab 1 — any one of 5 concurrent venue reads fails the whole `fetch_market_overview`. | `Aurix/Gaps.md` Gap 2 + `context/architecture.md` |
| K2 | Hard-coded WETH/USDC pool addresses + decimals in V2/V3 adapters (Tab 1). | `Aurix/Gaps.md` Gap 3 + `notes/dex-name-contract.md` |
| K3 | f64 precision at the IPC boundary (Tab 1). Not urgent for Tab 1; noted for any Tab-2 path that crosses Rust→TS for raw numbers. | `Aurix/Gaps.md` Gap 5 + `notes/wire-convention.md` |
| K4 | Duplicated analytical primitives in TS (median, formatUsd, GAS_UNITS_ESTIMATE, gas-adjusted formula) across 3-4 files. `formatUsd` already drifted on `signDisplay` argument. | `Aurix/Gaps.md` Gap 4 |
| K5 | Stale "three venues" copy in 3 places (PriceCard, ArbitragePage, market.rs rustdoc) + scaffolding residue (Tauri starter assets). | `Aurix/Gaps.md` Gap 8 + `context/architecture.md` |
| K6 | No per-adapter timestamp aggregation — `MarketOverview.fetched_at_unix_ms` is copied from V3 5bps snapshot, not command-local. | `Aurix/Gaps.md` Gap 10 |
| K7 | No automated IPC contract check — Rust `MarketOverview` and TS mirror are manually kept in sync. | `Aurix/Gaps.md` Gap 11 + `notes/wire-convention.md` |

These will be cross-referenced from any audit findings that touch the same files, but not re-derived as new findings — they already exist as documented gaps.

## Pre-existing skill plans in `context/plans/`

- `vector-a-v3-lp-backtester.md` — active plan (largely shipped this sprint).
- `vector-b-mev-detector.md` — proposed.
- `vector-c-ml-arbitrage-survival.md` — proposed.

The audit will not flag any work-in-progress on these as incomplete — they are active plans, not technical debt.

## Project preferences captured (will gate findings)

From `context/notes/*` and the `claude.md` principal-engineering brief:

- **No-inline-rationale convention** — design rationale lives in `context/`, never in `// WHY` / `// NOTE` annotations. Findings that propose adding decorative comments are out-of-scope.
- **Solo-contributor / commit-to-master** — no feature branches, no PR workflow assumptions.
- **No-paid-APIs** — Aurix uses only free/no-key data sources. Findings cannot propose adding paid services.
- **No synthetic in user-facing flows** — synthetic stays in tests + dev only. Findings that ask user-facing UI to fall back to synthetic are out-of-scope.
- **Crypto-domain hiring positioning** — depth on Tab 2 (LP backtester) is the strategic centre. Findings that improve LP-side rigour rank above Tab 1 polish.
- **Rust doc style** — backend rustdoc uses Inputs/Outputs/Errors/Side effects four-line contract.
- **DEX-name contract** — `dex_name` strings are the implicit identity key across Rust↔TS.
- **Wire convention** — `#[serde(rename_all = "camelCase")]` is the Rust↔TS bridge; prices/gas use `f64`.

## Scope-boundary notes

The audit will respect the following project-specific boundaries:

- **Frontend has no test infrastructure.** Audit will not stand up Vitest unless a finding genuinely requires it. Coverage-gap findings will recommend the implementing engineer add Vitest at the time the first test is needed.
- **Rust visibility constraint:** several functions of interest (e.g. `decoder::parse_swap_log`, `engine::distribute_swap_fees`) are `pub(crate)`. Diagnostic tests will exercise public surfaces (e.g. `IngestPipeline::run`, `Backtest::simulate`) per the `pub(crate)` workaround.
- **Live RPC tests are gated `#[ignore]`** by project convention. The audit will not unignore them and will not depend on live RPC access.

## Pass-2 ordering plan

Tier 1 (math, storage, backtest) → Tier 2 (ingest, IPC, strategies, benchmarks) → Tier 3 (headline, DEX, validation, config) → Tier 4 (frontend). Math first because it is the foundation everything else builds on; storage second because fan-in is highest; backtest third because it is the load-bearing computational core. Ingest before benchmarks because benchmarks compose results that ingest produces. Frontend last because the audit's diagnostic-test budget is highest in Rust where the test infrastructure already exists.

Pass-1 complete. Entering Pass 2.
