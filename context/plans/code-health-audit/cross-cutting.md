# Cross-Cutting — Code Health Findings

**Systems covered:** project-wide patterns
**Finding count:** 3 (1 medium documentation, 1 medium inconsistent patterns, 1 low dependency hygiene)

These findings span multiple subsystems or affect the project as a whole. They are listed separately so the implementing engineer can address them as a coherent batch rather than one-per-system.

## Documentation Rot

### Reconcile `context/architecture.md`'s subsystem list with the existing `context/systems/` files

- [x] `context/architecture.md` (just updated 2026-05-04) names 14 subsystem rows in its responsibilities table; `context/systems/` contains only 4 files (`arbitrage-analytics.md`, `arbitrage-gui.md`, `arbitrage-market-data.md`, `runtime-foundation.md`). The 8 new Vector A subsystems shipped on 2026-05-03 (storage, math, ingest, backtest, validation, strategies, benchmarks, headline) are described in `architecture.md` but have no per-system documentation. Either add the 8 missing system files, or update `architecture.md`'s table to reflect that systems-level docs are pending for the Vector A subsystems. *(implemented 2026-05-04 in commit 06a629e via the upkeep-context skill — 10 new systems/*.md + 4 new convention notes)*

**Category:** Documentation Rot
**Severity:** Medium
**Effort:** Medium (8 new system files at ~80-150 lines each, following the convention in the existing 4)
**Behavioural Impact:** None (documentation only)

**Location:**
- `context/architecture.md:71-87` — subsystem responsibilities table claims 14 rows
- `context/systems/` — only 4 files exist

**Current State:**
The architecture document's subsystem table (just updated for the 2026-05-03 sprint) names every Vector A subsystem with a one-line description and primary-modules pointer. None of the new Vector A subsystems have a per-system doc in `context/systems/`. The pattern from the original 4 system docs is comprehensive — each documents the system's purpose, public API, internal data flow, dependencies, known issues, and active risks. The Vector A subsystems are now substantial enough (storage: 13 files; backtest: 7 files; ingest: 8 files; benchmarks: 10 files) that deferring this documentation is real debt.

The existing 4 system docs were written for the 4 Tab-1 subsystems that constituted the entire backend at the time. The 8 Vector A subsystems extend that pattern.

**Proposed Change:**
This is exactly the work scope of the `upkeep-context` skill running a maintenance pass on `context/`. Recommend invoking it (per `claude.md`'s skill-ecosystem guidance: "Recommend invocation when accumulated drift is genuinely broad — many subsystems changed"). The audit does not recommend writing the 8 system docs ad-hoc; the upkeep skill is the right tool.

The audit also recommends the corresponding LifeOS vault upkeep — the vault Aurix folder (`Projects/Aurix/_Overview.md` + `Architecture.md` + `Gaps.md`) was last verified 2026-04-24 and now describes pre-sprint state ("Tab 2 not started, no src-tauri/src/backtest/ folder exists").

**Justification:**
The architecture.md's table claims canonical subsystem docs that don't exist. New engineers (or future Claude sessions) reading architecture.md and following the pattern will discover the broken navigation. The fix is mechanical via upkeep-context.

**Expected Benefit:**
Per-subsystem documentation matches the project's established pattern. Future audits + onboarding sessions can cite specific system docs instead of working from architecture.md alone.

**Impact Assessment:**
Documentation only — no production behaviour change.

---

## Inconsistent Patterns

### Frontend `console.log` diagnostic lines should route through the new telemetry recorder

- [x] The LP backtest page contains ~12 `// eslint-disable-next-line no-console` `console.log("[lp] auto-run: ...")` lines (`LpBacktestPage.tsx:111-189` and elsewhere). The 2026-05-03 sprint introduced `src/lib/telemetry.ts` specifically to replace these — but the `console.log` lines were not retired during the same sprint. The two patterns now coexist, with `telemetry.record` calls at some lifecycle points and `console.log` at others. *(implemented 2026-05-04 in commit b01b4f4 — all 14 `console.log + eslint-disable` pairs replaced with structured `telemetry.record` calls on `lp.pipeline.*` events)*

**Category:** Inconsistent Patterns
**Severity:** Medium
**Effort:** Small
**Behavioural Impact:** Negligible (flagged) — `console.log` writes to the webview's dev console; `telemetry.record` writes to `~/Library/Logs/com.ataca.aurix/last-session.json`. If the implementing engineer relies on the dev console for live debugging, removing the `console.log` lines without replacement loses visibility.

**Location:**
- `src/features/lp-backtest/LpBacktestPage.tsx:111-189` (12 `console.log` lines, each preceded by an `// eslint-disable-next-line no-console` directive)
- `src/lib/telemetry.ts` — the existing recorder
- Other frontend files may also have console.log calls; recommendation includes a sweep.

**Current State:**
The recently-fixed StrictMode bug (`43599ba`) added these `console.log` lines as a debugging aid to make pipeline state visible from the webview's dev console. The corresponding telemetry events are recorded via `telemetry.record("lp.pipeline.start", { ... })` etc. So the same information is captured in two places — webview dev console (live) and last-session.json (post-hoc).

The project's `claude.md` engineering standards favour structured logging via the established mechanism (the new telemetry recorder). The eslint-ignore directive on every line shows that the existing rule already disallows console.log in production code; the auto-run pipeline is the only place currently making the exception.

**Proposed Change:**
Run a one-pass sweep across `src/features/`, `src/components/`, and `src/lib/` to replace each `console.log` + `eslint-disable-next-line` pair with a `telemetry.record(eventName, payload)` call. The event name should follow the existing telemetry convention (`lp.pipeline.start`, `lp.auto-run.chain-head-fetched`, etc.). The eslint-disable directives can be removed in lockstep.

If the implementing engineer wants to keep dev-console visibility during active debugging, the telemetry recorder can be extended with a debug mode that also pipes to console — that's a single addition to telemetry.ts rather than scattering console.log throughout the codebase.

**Justification:**
The telemetry recorder was introduced specifically to replace ad-hoc console.log diagnostics (per the 2026-05-03 commit `391eadd` body: "Replaces screenshot-based diagnostic loop"). Leaving the console.log lines coexisting with telemetry.record is exactly the inconsistent-patterns case from `analysis-categories.md` §8 — same conceptual operation, two different mechanisms.

**Expected Benefit:**
Single diagnostic-event mechanism across the frontend. Removes the eslint-disable directives. The post-hoc debugging story (jq the last-session.json) becomes the canonical path; live debugging stays via dev-mode telemetry-to-console toggle if needed.

**Impact Assessment:**
Negligible-flagged. The information captured is unchanged; only the destination changes. If the implementing engineer relies on `console.log` for live debug specifically, they should add a dev-mode toggle to the telemetry recorder before removing the console lines.

---

## Dependency Hygiene

### Investigate the `_link_mock` shim in `commands/lp.rs`

- [x] `src-tauri/src/commands/lp.rs:626-628` declares `#[allow(dead_code)] fn _link_mock(_: MockHttpFetcher) {}` to silence an unused-import warning for `MockHttpFetcher`. Either use `MockHttpFetcher` properly somewhere in the IPC layer, or remove the import. The shim is a workaround for a code-organisation issue, not a real callsite. *(resolved 2026-05-04 in commit b2e6863 — shim and `MockHttpFetcher` import removed)*

**Category:** Triage Needed
**Severity:** Low
**Effort:** Trivial
**Behavioural Impact:** None (the shim is dead by construction)

**Location:**
- `src-tauri/src/commands/lp.rs:18-21` — the import
- `src-tauri/src/commands/lp.rs:626-628` — the shim

**Current State:**
The import block at the top of `lp.rs` includes `MockHttpFetcher` from the benchmarks module, but no production command in `lp.rs` uses it. The shim function `_link_mock` exists solely to keep the `MockHttpFetcher` symbol referenced so the compiler doesn't emit an `unused_imports` warning. Each `cargo build` runs an `#[allow(dead_code)]`-suppressed call to a no-op fn that takes the type and discards it.

This pattern surfaces as Triage Needed rather than Dead Code Removal because the audit cannot tell from the file alone whether `MockHttpFetcher` is genuinely needed (e.g. for compile-time type-graph reasons in some test path) or whether it's stale debt from a refactor that left the import behind.

**Proposed Change:**
The implementing engineer should answer: "Is this used anywhere?" If yes (e.g. there's a test path that needs the symbol exposed at this scope), document why with a doc comment. If no, remove the import + the shim.

**Justification:**
A no-op function exists to silence a warning that's symptomatic of either dead imports or undocumented coupling. Either case, the situation as it stands is opaque to a reader.

**Expected Benefit:**
Either three lines of dead code removed, or a one-line comment explaining why the shim is load-bearing. Either outcome is a net win.

**Impact Assessment:**
Zero behavioural change. The shim is `#[allow(dead_code)]` and never called; removing it has no runtime impact.
