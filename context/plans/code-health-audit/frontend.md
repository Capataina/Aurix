# Frontend — Code Health Findings

**Systems covered:** `src/features/lp-backtest/*`, `src/features/arbitrage/*`, `src/components/blocks/{lp,arbitrage}/*`, `src/lib/telemetry.ts`
**Finding count:** 4 (1 high modularisation, 1 medium coverage gap, 2 modularisation verdicts)

The frontend has no test infrastructure (no vitest, no jest, no `tests/` directory under `src/`). All findings below are necessarily backed by analysis-only evidence — the audit cannot write equivalence tests or baseline pins on the frontend without standing up vitest. Coverage Gap 1 (below) recommends doing so as part of the first frontend refactor.

## Modularisation

### Split `src/features/lp-backtest/LpBacktestPage.tsx` into a hook + presentational component

- [ ] Extract the auto-run pipeline orchestration in `LpBacktestPage.tsx:98-` into a custom hook `useLpPipeline(settings, rerunNonce)` that returns the entire derived state (`summary`, `curve`, `strategies`, `headline`, `headlineMonthly`, `benchmarks`, `resolved`, `poolMeta`, `busy`, `status`). Keep the JSX-rendering responsibility in the component.

**Category:** Modularisation
**Severity:** High
**Effort:** Medium
**Behavioural Impact:** None (pure refactor; same hooks, same useEffect structure, same fetch sequence — only the location of the orchestration code changes)

**Location:**
- `src/features/lp-backtest/LpBacktestPage.tsx` (entire 668-line file; the largest TS/TSX file in the project)

**Current State:**
The component declares 8 `useState` hooks (lines 66-84) for orchestration state, plus 1 `useEffect` for parent busy mirroring (87-89), plus 1 mega-`useEffect` for the auto-run pipeline (98+). The pipeline `useEffect` does the full ingest → first-swap-resolve → backtest → grid → headline → benchmarks → multi-asset-comparison sequence inline, with `console.log` diagnostic lines at every step.

The component's JSX (the part that *renders* the dashboard) is far down the file, after the orchestration. The pattern conflates "what the page is composed of" with "how it gets its data" — two concerns that change at very different rates. Adding a new block to the dashboard requires reading 400+ lines of pipeline code to find the JSX; tweaking a fetch retry or telemetry tag requires reading 200+ lines of JSX before getting to the useEffect.

The recent StrictMode-fix commit (`43599ba`) — fixing the `initialised` ref bug that hung the pipeline at `busy=true` — was a bug *of orchestration*, but the diff touched the same file as the visual layout. With the orchestration extracted, future StrictMode-class issues would be diagnosable in one file with one concern.

**Proposed Change:**
Extract `useLpPipeline` into `src/features/lp-backtest/useLpPipeline.ts`. The hook owns all 8 `useState` declarations, the diagnostic console.log lines (or equivalently, telemetry events — see `Coverage Gap 1` below), the chain-head fetch, the ingest step, the backtest, grid, headline, benchmarks fetches, and the resolved-state computation. It returns `{ busy, status, summary, curve, strategies, headline, headlineMonthly, benchmarks, resolved, poolMeta }`.

`LpBacktestPage.tsx` becomes a presentational component:

```tsx
export function LpBacktestPage({ settings, rerunNonce, onBusyChange }: LpBacktestPageProps) {
  const { busy, status, summary, curve, strategies, headline, headlineMonthly, benchmarks, resolved, poolMeta }
    = useLpPipeline(settings, rerunNonce);
  useEffect(() => onBusyChange(busy), [busy, onBusyChange]);
  return (
    <main>
      <KeyMetricsBlock summary={summary} resolved={resolved} status={status} />
      <PositionRangeBlock resolved={resolved} ... />
      ...
    </main>
  );
}
```

The hook becomes independently testable (with vitest + React Testing Library, see Coverage Gap 1) — feed it different `settings` shapes, assert that `busy` cycles correctly, that `summary` lands non-null, etc. The visual layout stays unchanged.

**Justification:**
The Modularisation §3 case from `analysis-categories.md` — large file mixing two concerns (data orchestration and visual composition) at different change cadences. Custom-hook extraction is the React-idiomatic fix; React 19's `use` hook (per the research) makes the orchestration even cleaner if the implementing engineer wants to migrate the data fetches to it.

The recent StrictMode-bug fix (`43599ba` from yesterday's sprint) is direct evidence that the orchestration code has its own complexity worth isolating. A separate hook file means the next StrictMode-class bug surfaces a small, focused diff instead of touching the same file as the dashboard layout.

**Expected Benefit:**
- `LpBacktestPage.tsx` shrinks from 668 lines to ~200 lines (just the JSX + the hook call + the busy mirror).
- `useLpPipeline.ts` becomes ~450 lines of focused orchestration code.
- The hook is independently testable when vitest lands (Coverage Gap 1).
- Future block additions to the dashboard touch only the JSX file.
- Future fetch-retry / telemetry-instrumentation changes touch only the hook file.

**Impact Assessment:**
Zero functional change. The hook returns identical state at identical times; React's hook ordering and useEffect rules are preserved. The audit verified by reading the existing useEffect that the only state mutations are `setBusy`, `setStatus`, and the eight derived setters — moving them into a hook (`useState` is hook-safe inside another custom hook) preserves the exact same observable behaviour.

The `console.log` diagnostic lines should move with the orchestration — or be replaced with `telemetry.record` calls (per the existing telemetry recorder; see Coverage Gap 1's note).

---

## Test Coverage Gaps

### Stand up vitest as part of the first frontend refactor

- [ ] When the implementing engineer touches the LP backtest frontend (e.g. for the modularisation finding above), add `vitest` + `@testing-library/react` to `devDependencies` and stand up a `src/__tests__/` directory or co-located `*.test.tsx` files. Add a `test` script to `package.json`. Write at least one test for `useLpPipeline` — assert that the hook fires the pipeline on mount, that `busy` cycles to `true` then `false`, and that the synthetic-data path is correctly disabled (per the `no-synthetic-in-user-facing-flows` project rule).

**Category:** Test Coverage Gaps
**Severity:** Medium
**Effort:** Small (initial setup) + Medium (cumulative test writing as findings are implemented)
**Behavioural Impact:** None (pure addition of dev infrastructure; no production code change)

**Location:**
- `package.json` — add `test` script + `vitest` dev dep
- `vite.config.ts` — extend with `vitest`'s `defineConfig` plus a `test:` block
- `src/features/lp-backtest/useLpPipeline.test.ts` (after the modularisation lands) — first test file

**Current State:**
The frontend has zero tests. `package.json` declares no test framework; `package.json:7-12` shows scripts `dev`, `build`, `preview`, `tauri` only. The backend has 139 unit/integration tests (rusqlite + tokio); the frontend has none. This is consistent with the Vector-A backend-first sprint (yesterday's work landed the entire backend stack with full test coverage; frontend assembly was second-priority), but the gap is now load-bearing — every backend refactor finding the audit produces is verifiable by `cargo test`, and every frontend refactor finding is verifiable only by manual inspection.

**Proposed Change:**
Minimal vitest setup:

```json
// package.json devDependencies addition
{
  "vitest": "^2.0.0",
  "@testing-library/react": "^16.0.0",
  "@testing-library/jest-dom": "^6.0.0",
  "jsdom": "^25.0.0"
}
```

```ts
// vite.config.ts
import { defineConfig } from 'vitest/config';
export default defineConfig({
  plugins: [react()],
  test: { globals: true, environment: 'jsdom' },
});
```

```json
// package.json scripts addition
"test": "vitest run",
"test:watch": "vitest"
```

First test target: `useLpPipeline` (after the modularisation finding lands). Asserts:
1. On mount with valid settings, the hook progresses `busy` from `false` → `true` → `false`.
2. `summary` becomes non-null after the pipeline completes.
3. The synthetic-fallback path is never taken (per the `no-synthetic-in-user-facing-flows` rule — the hook should throw on chain-head failure rather than fall back).

**Justification:**
The project's `claude.md` engineering-standards section emphasises testability as a first-tier engineering principle. The backend stack already meets that standard with 139 tests; the frontend should reach the same standard. Vitest is the canonical Vite-native test runner with minimal config overhead — it shares the project's existing build pipeline.

The audit's frontend findings (modularisation, telemetry-vs-console.log, etc.) gain rigour once vitest is in place; until it is, every frontend recommendation is analysis-only.

**Expected Benefit:**
- Frontend refactors become verifiable with the same rigour as backend refactors.
- The Modularisation finding above becomes implementable with a regression check (the hook's behavioural envelope can be pinned).
- Future StrictMode-class bugs (like the `43599ba` fix) become catchable by a unit test instead of by manual reproduction.

**Impact Assessment:**
Zero functional change to the production code. New dev dependencies + a new test runner are added. The Tauri build pipeline (`tauri build` → `tsc && vite build`) is unchanged. The dev-server `vite` runs unchanged. No frontend behaviour changes.

---

## Modularisation verdicts (no findings, just per-file dispositions for the floor obligation)

### `src/components/blocks/arbitrage/PriceChartBlock.tsx` (540 lines)

**Verdict:** `split-recommended`

**Justification:** Mixes (a) chart-mode state (Raw / Spread / Net P/L), (b) SVG path generation per mode, (c) event-marker rendering, (d) tooltip + interaction logic. Each is ~100-150 lines and independently extractable. The recent commit `cd5f7e8` ("drop Deviation mode + drop fill from Raw") shows that chart-mode logic changes independently of the SVG-path generation logic — a per-mode-handler module would make those edits localised. Recommended structure: `PriceChartBlock.tsx` (orchestrator) + `chart-modes/{raw,spread,net-pnl}.ts` (per-mode path computation) + `event-markers.ts` (decorations) + `tooltip.tsx` (interaction).

### `src/features/arbitrage/insights.ts` (490 lines)

**Verdict:** `split-recommended`

**Justification:** This file owns the entire Tab 1 analytical primitive surface — `median`, `formatUsd` (drifted per `Aurix/Gaps.md` Gap 4), `GAS_UNITS_ESTIMATE`, gas-adjusted spread, plus the insight-card derivation, severity classification, recent-event derivation, and the per-card threshold logic. The file mixes "shared helpers used in 3+ places" with "Tab-1-specific derivation logic." The shared helpers should move to `src/features/arbitrage/analytics.ts` per the existing Gaps.md Gap 4 prescription; the derivation logic stays in `insights.ts`. This is also the canonical home for the `formatUsd` consolidation that closes the existing drift.

### `src/features/lp-backtest/LpSettingsForm.tsx` (409 lines)

**Verdict:** `leave-as-is`

**Justification:** Forms with 10+ controls naturally hit ~400 lines in idiomatic React. Each control is ~30-50 lines (label + input + hint + validation render); 10 controls × 40 lines + 50 lines of state-binding boilerplate = ~450 lines is the irreducible floor. The file owns one concern (the LP settings form) and is internally cohesive — splitting per-control would scatter related controls (e.g. "deposit token0 amount" and "deposit token1 amount") across files for no readability gain.

### `src/lib/telemetry.ts` (397 lines)

**Verdict:** `leave-as-is`

**Justification:** The telemetry recorder is one cohesive concern (capture IPC start/end/error + clicks + lifecycle + lastState; serialise to `~/Library/Logs/com.ataca.aurix/last-session.json` on flush). The file's size reflects the surface it covers — multiple capture types each with their own typed event shapes — but it owns a single responsibility.

### `src/components/blocks/lp/StrategyControlsBlock.tsx` (360 lines) and `src/components/blocks/lp/MultiAssetCompareBlock.tsx` (319 lines)

**Verdict:** `leave-as-is`

**Justification:** Both are domain-specific UI blocks — strategy-control inputs (range × rule × deposit × period × MEV haircut) and multi-asset comparison rendering (LP vs S&P / Gold / Aave / Lido / T-bill / HODL). The size reflects the matrix of controls / metrics each block owns. Splitting would require a parent prop-drilling layer that doesn't reduce complexity; the current structure is internally cohesive.
