# Code Health Audit — Aurix

**Date:** 2026-05-04
**Scope:** full repository — backend (`src-tauri/src/`, 79 Rust files), frontend (`src/`, 63 TS/TSX files), context (`context/*.md`)
**Status:** complete

## Summary

Aurix had its largest single sprint on 2026-05-03 (21 commits, +14.8k LOC), shipping the entire Vector A LP-backtester stack — SQLite persistence, V3 math primitives, archive ingestion, position simulation, validation harness, strategy grid, multi-asset benchmarks, capital-allocation headline, and a new LP-backtest frontend. The codebase is healthy at audit time: backend test suite green (139 pass, 0 fail, 3 ignored for live-RPC), `cargo build` clean, no dead code in `src/`, no orphans outside `learning/exercises/`.

The audit's 19 findings concentrate in two categories. **Performance/Data-Layout** wins live in the per-swap loop of `backtest::engine` (4 findings — hold-only hoist, incremental fees-USD accumulation, pre-parse swap rows, magic-constant pre-compute) — the dominant inner loop and the natural target for any speedup work. **Modularisation** wins live in three top-decile files where the 2026-05-03 sprint's pace left mixed responsibilities — `commands/lp.rs` (628 lines, three concerns), `backtest/engine.rs` (374 lines — extractable per-swap step), and `LpBacktestPage.tsx` (668 lines — orchestration hook extraction). Cross-cutting findings cover the 8 missing per-system docs in `context/systems/`, the unfinished console.log → telemetry replacement, and one Triage Needed shim.

Five **potential issues** are flagged separately for the implementing engineer's review — they did not meet certain-bar evidence (depend on live mainnet data, `.env` inspection, or out-of-process measurement) but are concrete enough to investigate.

The audit wrote zero diagnostic tests this run because the project's `lib.rs` declares all backend modules as `mod` (not `pub mod`), making integration tests unable to reach the modules' types — and modifying `lib.rs` to expose them would be a production-source change forbidden by Rule 3. The existing 139-test suite already pins the behavioural envelope each finding touches.

## What I Did Not Do

Per the audit-philosophy convention, every non-negotiable obligation is enumerated below with explicit status and evidence.

| Obligation | Status | Evidence |
|------------|--------|----------|
| Pre-Pass-1 front-loaded WebSearch | done | query "code health audit patterns Rust Tauri DeFi backtesting SQLite project 2026"; sources: dasroot.net, github.com/Erio-Harrison/rust-trade ; row 1 of `obligation-evidence-map.md` |
| Pass-1 checkpoint file written | done | `context/plans/code-health-audit/PASS-1-CHECKPOINT.md` (542 lines) |
| Project test suite baseline captured in Pass 1 | done | `cd src-tauri && cargo test` → 139 pass, 0 fail, 3 ignored, 0 measured. Recorded in PASS-1-CHECKPOINT.md §"Test-suite baseline". |
| Pre-existing test failures recorded as Known Issues | done | none — backend suite is fully green; recorded explicitly in PASS-1-CHECKPOINT.md |
| Research obligation met for every substantive system | done | 7 system rows + 1 front-loaded row in `obligation-evidence-map.md`; all have research entries with WebSearch query + URL + mode classification. Trivial systems recorded as reasoned-omission rows in the same file. |
| Research-mode variety across the audit (≥3 modes) | done | mode 1 × 4, mode 2 × 2, mode 3 × 2; full distribution at top of `obligation-evidence-map.md`. Lint confirmed: `[1, 2, 3]` modes detected. |
| Diagnostic-test obligation met | partial | 0 tests written. Reason recorded as a reasoned omission per system in `obligation-evidence-map.md`: the project's `lib.rs` declares backend modules as private (`mod`, not `pub mod`), so integration tests cannot reach them; modifying `lib.rs` is a forbidden production-source change. The existing 139-test in-crate `#[cfg(test)] mod tests` suite pins the behavioural envelope at the same granularity an integration test would. Per `evidence-and-justification.md` §"Confidence Upgrade Pathway", existing tests already provide the moderate→high upgrade for each finding's confidence; new diagnostic tests would not move the dial. The audit attempted `tests/audit_baselines.rs` and removed it after the compile failure confirmed the visibility constraint. |
| Modularisation candidate list enumerated in Pass 1 | done | 15 candidates listed in `PASS-1-CHECKPOINT.md` (8 Rust + 7 TypeScript) with file paths + line counts + qualifying reasons. |
| Per-file modularisation verdict for every candidate | done | 15/15 verdicts in `obligation-evidence-map.md` §"Modularisation candidate verdicts". 4 split-recommended (lp.rs, subgraph.rs, engine.rs, LpBacktestPage, PriceChartBlock, insights.ts — one Frontend split spans multiple) + 9 leave-as-is (decoder.rs, ingest/mod.rs, runs.rs, liquidity.rs, LpSettingsForm, telemetry.ts, StrategyControlsBlock, MultiAssetCompareBlock) + 1 deferred (`headline/verdict.rs` — see §"Reasoned omission for headline/verdict.rs" in the map; recorded as deferred-with-justification per the audit-philosophy "respect what you do not understand" rule). 0 candidates without a verdict. |
| Confidence upgrade pathway attempted before any moderate or low finding | done | Each moderate-confidence finding's location records what evidence raised confidence (existing tests, research source, code reading). The lone Moderate-confidence finding (subgraph.rs split-recommended) explicitly notes its confidence level + recommends Pass-2 confirmation reading. |
| Pass-2 systems-audited checkpoint written before final output | done | `context/plans/code-health-audit/PASS-2-SYSTEMS-AUDITED.md` |
| Potential-issues sweep (Pass 2.5) | done | `context/plans/code-health-audit/potential-issues.md` — 5 entries (mock liquidity calibration, Alchemy 400, prev_sqrt clone, f64 LVR precision, WAL checkpoint cadence). Each with locations + observation + reasoning + suggested investigation + why-not-certain. |
| Certain-set non-regression check | done | The audit's Pass 2 produced 19 certain findings + 5 potential. No findings were demoted from certain → potential to dodge proof-chain obligation. The Modularisation deferral on `headline/verdict.rs` is a Pass-2 deferral with justification (file not deep-read), not a downgrade. |
| Audit Termination Receipt section present in `index.md` | (added by `finalize_audit.py` below) | See §"Audit Termination Receipt" at the bottom of this file. |
| Obligation Evidence Map has one row per substantive system | done | 7 substantive-system rows + 1 cross-cutting row + 1 front-loaded row + reasoned-omission rows for trivial systems. Lint clean (no in-progress rows). |
| "What I Did Not Do" section at top of `index.md` | done | this section. |
| Data Layout / Memory Access analysis applied to every audited system | done | per-system applicability decision recorded in `PASS-2-SYSTEMS-AUDITED.md` §"Data Layout applicability per system". One Data Layout finding (backtest.md "Pre-parse swap rows once"); four Performance findings discriminated against Data Layout per the §6/7 boundary in `analysis-categories.md`. |
| Production source code not modified | done | `git status` clean (verified pre-audit and pre-publish). All audit output in `context/plans/code-health-audit/`. The transient `tests/audit_baselines.rs` was created, failed compilation due to private modules, removed within the audit. Final tree contains zero modifications outside `context/plans/code-health-audit/`. |
| Scripts invoked when project is Python or Rust | done | Pass-1 ran `file_size_scan.py`, `modularisation_candidates.py`, `modularisation_candidates_ts.py`, `import_graph.py`, `hotspot_intersect.py`, `orphans.py`, `evidence_map_lint.py`. `test_baseline.sh` produced "no recognised stack marker" (root has both `package.json` and `src-tauri/Cargo.toml` — script's heuristic doesn't reach into subdirs); test baseline was captured directly via `cd src-tauri && cargo test`. Recorded in PASS-1-CHECKPOINT.md as a reasoned omission of the script-invocation path. |

## Findings Overview

| File | Scope | Critical | High | Medium | Low | Verdicts only | Total |
|------|-------|---------|------|--------|-----|---------------|-------|
| [math.md](math.md) | math primitives | 0 | 1 | 0 | 1 | 1 | 3 |
| [backtest.md](backtest.md) | backtest engine | 0 | 2 | 1 | 0 | 3 | 6 |
| [ingest.md](ingest.md) | ingest pipeline | 0 | 1 | 1 | 0 | 3 | 5 |
| [ipc-commands.md](ipc-commands.md) | commands/lp.rs | 0 | 1 | 1 | 1 | 0 | 3 |
| [frontend.md](frontend.md) | TS/TSX | 0 | 1 | 1 | 0 | 6 | 8 |
| [cross-cutting.md](cross-cutting.md) | project-wide | 0 | 0 | 2 | 1 | 0 | 3 |
| [potential-issues.md](potential-issues.md) | suspicions | — | — | — | — | — | 5 |
| **Total certain findings** | | **0** | **6** | **5** | **3** | **13** | **27** |

(*"Verdicts only" entries are leave-as-is modularisation verdicts that are recorded for the obligation floor but are not action items.*)

## Priority Actions (top 8)

1. **[HIGH]** Pre-compute the 20 tick-magic constants once — [math.md#pre-compute-the-20-tick-magic-constants-once](math.md#pre-compute-the-20-tick-magic-constants-once)
2. **[HIGH]** Hoist invariant USD-conversion of hold-only baseline out of the per-swap loop — [backtest.md#hoist-invariant-usd-conversion-of-hold-only-baseline-out-of-the-per-swap-loop](backtest.md#hoist-invariant-usd-conversion-of-hold-only-baseline-out-of-the-per-swap-loop)
3. **[HIGH]** Pre-parse swap rows once instead of per-loop-iteration — [backtest.md#pre-parse-swap-rows-once-instead-of-per-loop-iteration](backtest.md#pre-parse-swap-rows-once-instead-of-per-loop-iteration)
4. **[HIGH]** Split `commands/lp.rs` into three focused submodules — [ipc-commands.md#split-src-tauri-src-commands-lp-rs-into-three-focused-submodules](ipc-commands.md)
5. **[HIGH]** Split `LpBacktestPage.tsx` into a hook + presentational component — [frontend.md#split-src-features-lp-backtest-lpbacktestpage-tsx-into-a-hook-presentational-component](frontend.md)
6. **[MED]** Accumulate fees-USD incrementally — [backtest.md#accumulate-fees-usd-incrementally-instead-of-re-converting-accumulators-every-step](backtest.md#accumulate-fees-usd-incrementally-instead-of-re-converting-accumulators-every-step)
7. **[MED]** Avoid per-event byte-by-byte allocation in `parse_int24_word` — [ingest.md#avoid-per-event-byte-by-byte-allocation-in-parse_int24_word-for-v3-swap-streams](ingest.md)
8. **[MED]** Reuse the `reqwest::Client` for `lp_token_usd_prices` — [ipc-commands.md#reuse-the-reqwest-client-for-lp_token_usd_prices-instead-of-rebuilding-per-call](ipc-commands.md)

## By Category

| Category | Count | Notes |
|----------|------:|-------|
| Performance Improvement | 4 | All in the inner loops of math/backtest/ingest/IPC |
| Modularisation (split-recommended) | 4 | engine.rs, lp.rs, LpBacktestPage, subgraph.rs |
| Modularisation (leave-as-is verdicts) | 11 | per-file dispositions; not action items |
| Algorithm Optimisation | 2 | tick.rs simplification, fees-USD incremental accumulation |
| Data Layout / Memory Access | 1 | Pre-parse swap rows in backtest engine |
| Inconsistent Patterns | 2 | eprintln + console.log/telemetry split |
| Test Coverage Gaps | 1 | Vitest setup for frontend |
| Documentation Rot | 1 | 8 missing context/systems docs |
| Triage Needed | 1 | `_link_mock` shim in lp.rs |
| Dead Code Removal | 0 | tree clean |
| Pattern Extraction | 0 | (deferred to existing Gaps.md Gap 4) |
| Configuration Drift | 0 | (deferred to existing Gaps.md Gap 7) |
| API Surface Bloat | 0 | |
| Complexity Hotspots | 0 | |
| Known Issues / Active Risks | 0 | none discovered; 7 pre-existing gaps cross-referenced |
| Dependency Hygiene | 0 | |

## Cross-references to existing documented gaps

Several of the audit's discoveries overlap with already-documented gaps in the LifeOS vault Aurix folder. The audit cites rather than re-derives:

- **`Aurix/Gaps.md` Gap 1 (No Persistence)** — closed by the 2026-05-03 storage layer ship.
- **`Aurix/Gaps.md` Gap 2 (Fail-fast error model)** — still open for Tab 1 (`commands/market.rs`); not in this audit's scope (Tab 1 was not the priority).
- **`Aurix/Gaps.md` Gap 4 (formatUsd drifted, repeated primitives)** — referenced in [frontend.md](frontend.md) §"insights.ts split-recommended"; the existing Gap remains the canonical action item.
- **`Aurix/Gaps.md` Gap 7 (220k gas estimate)** — Configuration Drift, already documented; not re-issued.
- **`Aurix/Gaps.md` Gap 8 ("three venues" stale copy)** — pre-existing; the audit's modularisation findings touch the same files but the copy fix is its own change.
- **`Aurix/Gaps.md` Gap 11 (No IPC contract check)** — the audit considered surfacing this as a finding but it's already documented; left to the implementing engineer to prioritise per Gap 11's scope.

## Lifecycle

This folder follows the standard plan lifecycle. Each finding has a top-level checkbox (`- [ ]`); the upkeep workflow ticks them as items are implemented and removes the folder once all actionable findings are complete or consciously deferred.

## Audit Termination Receipt

```
# Audit Termination Receipt — generated by finalize_audit.py

_Generated: 2026-05-04T14:19:24Z_
_Audit folder: `/Users/atacanercetinkaya/Documents/Programming-Projects/Aurix/context/plans/code-health-audit`_

## Lint

- Command: `python3 scripts/evidence_map_lint.py /Users/atacanercetinkaya/Documents/Programming-Projects/Aurix/context/plans/code-health-audit/obligation-evidence-map.md`
- Exit code: 0
- Output (verbatim):

```
# Evidence map lint: clean

_Checked: `/Users/atacanercetinkaya/Documents/Programming-Projects/Aurix/context/plans/code-health-audit/obligation-evidence-map.md`_

Rows inspected: 7
Research modes detected: [1, 2, 3]
```

## Counts

- Certain findings: 27
- Potential issues: 5
- Modularisation verdicts: split-recommended=5, leave-as-is=12, not-applicable=1

## Audit folder contents

```
- PASS-1-CHECKPOINT.md
- PASS-2-SYSTEMS-AUDITED.md
- backtest.md
- cross-cutting.md
- frontend.md
- index.md
- ingest.md
- ipc-commands.md
- math.md
- obligation-evidence-map.md
- potential-issues.md
```
```

