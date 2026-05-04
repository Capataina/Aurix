# Staleness Report — 2026-05-04

Snapshot produced by `upkeep-context` per Execution Workflow step 3b. Overwrites on each upkeep run; this is a snapshot, not an accumulating log.

## Per-file table (every `.md` under `context/`)

| File | Verdict | Evidence |
|------|---------|----------|
| `context/architecture.md` | updated this run | Added Inter-System Relationships section split into Tab 1 + Tab 2; added Critical Paths section with 8-system Tab 2 trace and Tab 1 1Hz tick trace |
| `context/notes.md` | updated this run | Index now lists 11 notes (added 4 new convention notes from this run) |
| `context/notes/dex-name-contract.md` | up-to-date | Tab 1 specific; `dex_name` strings still load-bearing per `commands/market.rs` |
| `context/notes/error-handling.md` | up-to-date | one-thiserror-per-module convention still holds; verified against StorageError, BacktestError, IngestError |
| `context/notes/free-data-fallback-chain.md` | created this run | New convention note covering the no-paid-APIs constraint + tiered fallback |
| `context/notes/idempotent-runs.md` | created this run | New convention note covering `config_hash` + `INSERT OR IGNORE` + StrictMode safety |
| `context/notes/lp-backtester-data-sources.md` | up-to-date | Re-read confirmed it covers 4-tier chain, Alchemy 400 note, cross-chain + V3 forks, telemetry diagnostics — all current as of commit 391eadd |
| `context/notes/no-inline-rationale.md` | up-to-date | grep `WHY|NOTE|HACK|IMPORTANT|TODO|FIXME|SAFETY|XXX` across `src/` and `src-tauri/src/` returns 0 hits — convention still holds |
| `context/notes/no-synthetic-in-user-facing.md` | created this run | Formalises the user's stated preference (in MEMORY.md) into project context |
| `context/notes/round-trip-fee-math.md` | up-to-date | fee math semantics unchanged this sprint; `math::fees::fee_share` still uses the documented round-trip approach |
| `context/notes/rust-doc-style.md` | up-to-date | Inputs/Outputs/Errors/Side-effects four-line contract observed in math/q96.rs, math/tick.rs, storage/mod.rs |
| `context/notes/storage-conventions.md` | created this run | New convention note (idempotent INSERT OR IGNORE, lowercase pool addresses, SYNTHETIC_TX_HASH separation, TEXT decimal for >64-bit ints) |
| `context/notes/wire-convention.md` | updated this run | Extended from Tab 1 only to cover full Vector A type families + LP DTOs + `CommandError` shape + `KeyRequired` variant |
| `context/plans.md` | up-to-date | indexes 3 vector plans correctly |
| `context/plans/code-health-audit/index.md` | preserved | just-written audit output (this session); not subject to staleness check on first run |
| `context/plans/code-health-audit/PASS-1-CHECKPOINT.md` | preserved | audit checkpoint |
| `context/plans/code-health-audit/PASS-2-SYSTEMS-AUDITED.md` | preserved | audit checkpoint |
| `context/plans/code-health-audit/obligation-evidence-map.md` | preserved | audit ledger |
| `context/plans/code-health-audit/backtest.md` | preserved | audit findings |
| `context/plans/code-health-audit/cross-cutting.md` | preserved | audit findings |
| `context/plans/code-health-audit/frontend.md` | preserved | audit findings |
| `context/plans/code-health-audit/ingest.md` | preserved | audit findings |
| `context/plans/code-health-audit/ipc-commands.md` | preserved | audit findings |
| `context/plans/code-health-audit/math.md` | preserved | audit findings |
| `context/plans/code-health-audit/potential-issues.md` | preserved | audit potential-issues |
| `context/plans/vector-a-v3-lp-backtester.md` | updated this run | Frontmatter status banner refreshed: `active (research complete)` → `shipped (code-complete; 4-tier verification pending)` plus new status-note callout |
| `context/plans/vector-b-mev-detector.md` | up-to-date | proposed plan, not started |
| `context/plans/vector-c-ml-arbitrage-survival.md` | up-to-date | proposed plan, not started |
| `context/references/backtest-statistical-methodology.md` | preserved | research paper; still relevant for strategies grid Sharpe/Sortino/DSR |
| `context/references/defi-yield-data-sources.md` | preserved | DefiLlama / Aave / Compound / Lido endpoints used by current `benchmarks/defi.rs` |
| `context/references/ethereum-archive-log-ingestion.md` | preserved | reference for current `ingest/alchemy.rs` chunking logic |
| `context/references/lp-rebalancing-strategies.md` | preserved | reference for `backtest/rebalance.rs` |
| `context/references/oss-v3-backtester-landscape.md` | preserved | comparative reference; not implementation-truth |
| `context/references/out-of-scope-risks-survey.md` | preserved | risk catalogue; durable |
| `context/references/sqlite-rust-production-patterns.md` | preserved | research that grounded the current `storage/` topology |
| `context/references/tradfi-benchmark-data-sources.md` | preserved | FRED + Stooq endpoints used by current `benchmarks/tradfi.rs` |
| `context/references/v3-lp-profitability-literature.md` | preserved | research paper; durable |
| `context/references/v3-mathematics-deep-dive.md` | preserved | grounds current `math/q96.rs` + `math/tick.rs` |
| `context/references/v3-position-validation-methodology.md` | preserved | grounds `validation/` subsystem |
| `context/systems/arbitrage-analytics.md` | up-to-date | Tab 1 analytics layer unchanged this sprint |
| `context/systems/arbitrage-gui.md` | updated this run | Added block-grid restructure (commit 334e8ac), chart mode reduction (commit cd5f7e8), settings menu (commit 611ee40), per-component CSS organisation |
| `context/systems/arbitrage-market-data.md` | up-to-date | Tab 1 backend market pipeline unchanged this sprint |
| `context/systems/backtest.md` | created this run | New system file for the Vector A position-simulation engine |
| `context/systems/benchmarks.md` | created this run | New system file for the multi-asset benchmark module |
| `context/systems/headline.md` | created this run | New system file for the M2.8 capital-allocation verdict synthesis |
| `context/systems/ingest.md` | created this run | New system file for the archive-log ingestion + ABI decoder + tiered fallback |
| `context/systems/lp-backtest-gui.md` | created this run | New system file for the Tab 2 dashboard frontend |
| `context/systems/math.md` | created this run | New system file for the V3 math primitives |
| `context/systems/runtime-foundation.md` | updated this run | Refreshed Current Implemented Reality + Partial sections to reflect multi-tab + 19-IPC + Storage handle |
| `context/systems/storage.md` | created this run | New system file for the SQLite + WAL persistence layer |
| `context/systems/strategies.md` | created this run | New system file for the grid-search runner |
| `context/systems/telemetry.md` | created this run | New system file for the cross-cutting IPC tracer |
| `context/systems/validation.md` | created this run | New system file for the ground-truth replay harness |

**Total files walked: 51 (after this run's additions).** 0 verdicts unset.

## Coverage gap report

| Repository area | Inferred system name | Proposed filename | Disposition |
|-----------------|---------------------|-------------------|-------------|
| `src-tauri/src/storage/` | storage | `systems/storage.md` | **created this run** |
| `src-tauri/src/math/` | math | `systems/math.md` | **created this run** |
| `src-tauri/src/ingest/` | ingest | `systems/ingest.md` | **created this run** |
| `src-tauri/src/backtest/` | backtest | `systems/backtest.md` | **created this run** |
| `src-tauri/src/strategies/` | strategies | `systems/strategies.md` | **created this run** |
| `src-tauri/src/benchmarks/` | benchmarks | `systems/benchmarks.md` | **created this run** |
| `src-tauri/src/headline/` | headline | `systems/headline.md` | **created this run** |
| `src-tauri/src/validation/` | validation | `systems/validation.md` | **created this run** |
| `src/features/lp-backtest/` + `src/components/blocks/lp/` | lp-backtest-gui | `systems/lp-backtest-gui.md` | **created this run** |
| `src/lib/telemetry.ts` + `src-tauri/src/commands/telemetry.rs` | telemetry | `systems/telemetry.md` | **created this run** |
| `src-tauri/src/config/` (mod.rs + chains.rs) | config | (none) | folded into `systems/runtime-foundation.md` — `config/` is sufficiently small and tightly coupled to runtime startup that a separate file would just split a coherent concern |
| `src-tauri/src/dex/` | dex-adapters | (none) | covered by `systems/arbitrage-market-data.md` (Tab 1 backend) — V2/V3 adapters are part of that system's responsibility |
| `src-tauri/src/ethereum/` | ethereum-rpc | (none) | covered by `systems/runtime-foundation.md` — single shared transport, owned at the foundation layer |
| `src-tauri/src/commands/` | commands-ipc | (none) | covered transversally — `commands/market.rs` lives under `arbitrage-market-data`, `commands/lp.rs` is documented as the IPC surface in `lp-backtest-gui.md` + the audit's `ipc-commands.md`, `commands/telemetry.rs` lives under `systems/telemetry.md` |
| `src/components/primitives/` | shared primitives | (none) | shared UI primitives (Heatmap, Icon) used by both Tabs; small enough to not warrant a system file. Mentioned in `systems/arbitrage-gui.md` and `systems/lp-backtest-gui.md` |
| `src/components/shell/` | shell | (none) | the global shell (TopBar + SettingsMenu) is small (~400 lines combined) and route-aware behaviour is covered in `systems/arbitrage-gui.md` and `systems/lp-backtest-gui.md` |
| `src/styles/` | styling | (none) | per-component CSS organisation is documented in `systems/arbitrage-gui.md` Current Implemented Reality |

**Coverage gap report: zero uncovered subsystems.** All 10 source-tree gaps from the initial scan are either now covered by a new file or explicitly folded into an existing file with documented justification.

## Inspection-scope tracking

- **Inspected this run** (read in full or substantively): math/{q96,tick,liquidity,fees}, storage/{mod,swaps}, backtest/{engine,mod}, ingest/decoder, commands/lp.rs, lib.rs, LpBacktestPage.tsx (first 200 lines), all 4 existing systems/, all 7 existing notes/, all 3 vector plans, all 11 references (titles + cross-referenced from new system files).
- **Noted-but-not-read this run**: storage's other 11 files, ingest's 7 other files, backtest's 5 other files, all of strategies + headline + validation + benchmarks + config + dex internals — but covered via the audit's prior reading + scan_repo's import-graph evidence + Pass-1 prioritisation.
- **Inferred from structure**: src/components/blocks (block names from file_size_scan), src/components/primitives, src/components/shell, dist/, public/, src-tauri/gen/.

This pattern (deep-read for the audit + structural inference for upkeep coverage) is appropriate for a recently-audited project — the audit ran first and the system files cite the audit findings rather than re-deriving them.
