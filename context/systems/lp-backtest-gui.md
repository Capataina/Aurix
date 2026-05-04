# LP Backtest GUI

## Scope / Purpose

- The Tab 2 dashboard frontend — auto-run pipeline, equity curve, strategy heatmap, headline verdict block, multi-asset comparison hero block, and the LP settings form. Composes data fetched from the Vector A backend via Tauri IPC into the user's primary "did LP'ing make sense?" view.

## Boundaries / Ownership

- Owns: the LP page orchestration (`LpBacktestPage.tsx`), the LP settings form (`LpSettingsForm.tsx`), per-block component rendering for LP (`src/components/blocks/lp/*`), the LP-specific TS API client (`src/features/lp-backtest/api.ts`), curated pool list (`pools.ts`), chain configuration (`chains.ts`), and the LP-feature-scope types (`types.ts`).
- Does **not** own: the cross-cutting telemetry recorder ([telemetry](telemetry.md)), shared block primitives (`src/components/primitives/`), the global UI shell (`src/components/shell/`).

## Current Implemented Reality

```text
src/features/lp-backtest/
├── LpBacktestPage.tsx          # 668-line orchestrator — useEffect pipeline + 8 useState declarations + JSX
├── LpSettingsForm.tsx          # 409-line form — 10+ controls, route-aware via SettingsMenu
├── api.ts                      # Typed Tauri IPC client wrappers (10+ functions, mirrors commands/lp.rs)
├── chains.ts                   # CHAIN_CONFIGS — Ethereum / Arbitrum / Optimism / Base / Polygon
├── pools.ts                    # Curated pool list (per-chain, per-protocol)
├── defaults.ts                 # DEFAULT_GRID_RULES + DEFAULT_GRID_RANGE_WIDTHS + DEFAULT_GRID_PERIOD_DAYS
└── types.ts                    # 217 lines of Rust↔TS type mirrors (PositionConfig, EquityCurvePoint, ...)

src/components/blocks/lp/        # 10+ block components for the dashboard
├── BenchmarkCacheBlock.tsx
├── EquityCurveBlock.tsx
├── HeadlineVerdictBlock.tsx
├── KeyMetricsBlock.tsx
├── MultiAssetCompareBlock.tsx   # 319 lines — hero block, LP vs SP500 / Gold / Aave / Lido / T-bill / HODL
├── PositionPnlBlock.tsx
├── PositionRangeBlock.tsx
├── RegimePanelBlock.tsx
├── StrategyControlsBlock.tsx    # 360 lines — strategy-grid form
└── StrategyHeatmapBlock.tsx
```

**Auto-run pipeline.** The page mounts, the settings-form-stable JSON key changes, and `useEffect` fires the pipeline:

1. Fetch pool metadata (token0/token1 decimals + symbols + fee tier).
2. Fetch chain head via `lpGetChainHead` (defaults the block window to `[head − N, head]`).
3. Run live ingest via `runLpIngestion` (3-tier fallback orchestrated server-side).
4. Resolve realised first-swap tick + price via `lpQueryFirstSwapPrice`.
5. Fetch token USD prices via `lpTokenUsdPrices` (when pool isn't USD-quote).
6. Run `runLpBacktest` for the realised position config.
7. Run `runLpGrid` for the strategies grid.
8. Run `runLpHeadline` for the M2.8 verdict.
9. Fetch persisted benchmark series via `lpFetchBenchmarkSeries` for each tracked benchmark.

Each step records via `telemetry.record(eventName, payload)` ([telemetry](telemetry.md)). Failures throw and surface as red banners; no synthetic fallback in user-facing flow (per project convention `notes/no-synthetic-in-user-facing.md` — to be created).

**StrictMode discipline.** The pipeline uses a `mounted` flag (per-closure) only to gate state setters, not to short-circuit the pipeline body. Both StrictMode-mount invocations run the pipeline; the first's stale `setX` calls become no-ops. The pipeline is fully idempotent because [storage](storage.md) keys runs by `config_hash`. See commit `43599ba` for the bug this fix closed.

**`settingsKey = JSON.stringify(settings)`** is the auto-run trigger — typing into a stepper bumps the settings object identity but not the JSON, so the pipeline doesn't fire mid-keystroke.

## Key Interfaces / Data Flow

| Boundary | Direction | Shape | Notes |
|---|---|---|---|
| `LpBacktestPage` ← parent (`App.tsx`) | inbound | `LpSettings`, `rerunNonce`, `onBusyChange` | settings come from `LpSettingsForm` via parent state |
| `LpBacktestPage` → backend | outbound (IPC) | `lpPoolMetadata`, `lpGetChainHead`, `runLpIngestion`, `lpQueryFirstSwapPrice`, `lpTokenUsdPrices`, `runLpBacktest`, `runLpGrid`, `runLpHeadline`, `lpFetchBenchmarkSeries` | typed via `api.ts` mirrors of [commands](../plans/code-health-audit/ipc-commands.md) Rust commands |
| `LpBacktestPage` → block components | outbound (props) | derived state per block | `summary`, `curve`, `strategies`, `headline`, `headlineMonthly`, `benchmarks`, `resolved`, `poolMeta` |
| `LpBacktestPage` → [telemetry](telemetry.md) | outbound | `telemetry.record(name, payload)` per pipeline step | replaces the older `console.log` diagnostic loop |

**Rust↔TS contract** for the LP payloads is in `types.ts` — `PositionConfig`, `EquityCurvePoint`, `PositionRunSummary`, `StrategyResultRow`, `HeadlineRunSummary`, `HeadlineMonthlyRow`, `BenchmarkPoint`, `GridConfig`, `HeadlineConfig`, `HeadlineMonthlyInput`, `PoolMetadata`. Manually kept in sync with the Rust side (no codegen). Per [audit findings](../plans/code-health-audit/frontend.md), this is a Coverage-Gap finding (Vitest setup recommended for pinning the contract).

## Implemented Outputs / Artifacts

- The Tab 2 dashboard rendered in the user's webview.
- Telemetry events flowing to `~/Library/Logs/com.ataca.aurix/last-session.json`.
- No frontend tests (zero — Coverage-Gap finding in audit).

## Known Issues / Active Risks

- **`LpBacktestPage.tsx` is 668 lines mixing orchestration + JSX.** The audit recommends splitting orchestration into a `useLpPipeline` hook ([audit findings](../plans/code-health-audit/frontend.md) §"Split LpBacktestPage").
- **No frontend test infrastructure.** Zero coverage; Vitest not yet adopted. Recorded as Coverage-Gap finding in audit.
- **`console.log` diagnostic lines coexist with `telemetry.record` calls.** ~12 `console.log` lines in `LpBacktestPage.tsx` should route through telemetry per the [cross-cutting](../plans/code-health-audit/cross-cutting.md) audit finding.
- **Manual Rust↔TS type sync.** No codegen (`ts-rs` / `specta`); the implementing engineer must update `types.ts` whenever a Rust DTO changes. Documented in `notes/wire-convention.md`.

## Partial / In Progress

- 4-tier extension (chain selector + protocol selector + non-USD pool support) shipped untested 2026-05-04 01:28; verifying each tier on the next session is part of the carry-forward from commit 391eadd.

## Planned / Missing / Likely Changes

- `useLpPipeline` hook extraction.
- Vitest setup + first test for the hook.
- Replace remaining `console.log` lines with `telemetry.record`.

## Durable Notes / Discarded Approaches

- **Auto-run on settings change over manual "Run" button.** Earlier design required the user to click Run after every settings change; auto-run produces a more responsive dashboard but introduced the StrictMode bug fixed in commit `43599ba`. The trade-off is documented as "auto-run requires full pipeline idempotency" — which storage's `config_hash` keying provides.
- **Block-grid layout over draggable grid.** Earlier design used a draggable grid (react-grid-layout) for user-customisable block placement; the resulting state-management complexity outweighed the user benefit. Removed in commit `72d4fd7` ("drop draggable grid, all 15 blocks stacked + scrollable"). Documented in `systems/arbitrage-gui.md` for Tab 1 which followed the same migration.
- **`lookbackBlocks` over fixed from/to inputs.** Earlier UI had explicit from-block + to-block inputs; users found these meaningless because they don't intuit "what's the current block height?". Replaced with a single `lookbackBlocks` field (default 1000) anchored to the live chain head. Commit `53f99eb` documented the fix.

## Obsolete / No Longer Relevant

- The pre-`lookbackBlocks` from/to-block inputs.
- The pre-`telemetry.record` `console.log` debugging loop (still present in code, slated for removal per the audit cross-cutting finding).

## Cross-references

- Parent: `App.tsx` (top-level routing + busy mirror).
- Sibling: [arbitrage-gui](arbitrage-gui.md) (Tab 1 frontend), [telemetry](telemetry.md) (cross-cutting recorder).
- Backend boundary: `commands/lp.rs` (every IPC call traced in [audit findings](../plans/code-health-audit/ipc-commands.md)).
- Related notes: `notes/wire-convention.md`, `notes/no-synthetic-in-user-facing.md` (to be created).
