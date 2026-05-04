# Pass 2 Systems Audited

Static snapshot of the per-system Pass 2 work as of 2026-05-04 audit close. The live evidence ledger lives in [obligation-evidence-map.md](obligation-evidence-map.md); this file is the audit-final summary.

| System | Research evidence | Tests written | Findings | Confidence |
|--------|-------------------|---------------|---------:|------------|
| Math primitives (`math/*.rs`, 1,095 lines) | mode 2 + mode 3, sources: shuhuiluo/uniswap-v3-sdk-rs, athaydes BigUint perf post, once_cell docs | 0 (existing 30+ tests pin envelope) | 3 (1 high, 1 low, 1 leave-as-is) | High |
| Storage layer (`storage/*.rs`, 13 files) | mode 1, sources: kerkour.com SQLite/Rust perf, tokio-rusqlite docs, CachedStatement | 0 (existing 18 storage-test tests sufficient; no findings warranted) | 0 (1 leave-as-is for runs.rs) | High |
| Backtest engine (`backtest/*.rs`) | mode 2, sources: Bailey/de Prado SSRN paper on Deflated Sharpe, Wikipedia DSR | 0 (see reasoned omission in obligation-evidence-map; existing 6 tests pin envelope) | 4 (2 high, 1 medium, 1 split-recommended) | High |
| Ingest pipeline (`ingest/*.rs`) | mode 3, sources: Alchemy eth_getLogs deep-dive, Nethermind log-index post | 0 (existing decoder tests + ingest tests sufficient) | 2 (1 medium perf, 1 high modularisation verdict) | High |
| IPC commands (`commands/lp.rs`, 628 lines) | mode 1, sources: Tauri 2 state mgmt docs, Tauri DB pool guide | 0 (modularisation is mechanical move; verifiable by `cargo build`) | 3 (1 high, 1 medium, 1 low) | High |
| Frontend (LP page + arbitrage + telemetry) | mode 1, sources: React 19 + useEffect cleanup posts | 0 (no frontend test infrastructure exists; standing it up is itself a finding) | 4 + 4 verdicts | Moderate (no test infra) |
| Cross-cutting | (no per-row research; coverage met across the four primary systems) | 0 (documentation/inconsistency findings; no test would resolve) | 3 | High |

**Total findings emitted: 19** across 7 finding-bearing files (math, backtest, ingest, ipc-commands, frontend, cross-cutting, plus 5 entries in potential-issues).

**Total modularisation candidate verdicts: 15** across all candidates from the file_size_scan output (8 Rust + 7 TypeScript at threshold).

**Diagnostic tests written by the audit: 0.** All diagnostic-test deferrals recorded as reasoned omissions in [obligation-evidence-map.md](obligation-evidence-map.md). Primary deferral: the project's `lib.rs` declares all backend modules as `mod` (not `pub mod`), so integration tests at `tests/audit_baselines.rs` cannot reach the modules' types. Making them `pub` would be a production-source change (forbidden by Rule 3); the `pub(crate)` workaround per `detection-strategies.md` §"Rust pub(crate) Visibility Constraint" requires exercising the public surface, which is the Tauri command handler list — only callable through the Tauri runtime (out-of-process state, no-test-physically-possible deferral applies). The existing in-crate `#[cfg(test)] mod tests` tests (139 passing across 13 modules) already pin the behavioural envelope at the same granularity an integration test would.

**Existing test baseline: 139 pass, 0 fail, 3 ignored** (per `cargo test` from `src-tauri/`). Frontend has zero test coverage — recommended remediation in [frontend.md](frontend.md) §Coverage Gap 1.

## Coverage of category taxonomy

| Category | Findings emitted |
|---|---:|
| Dead Code Removal | 0 |
| Triage Needed | 1 (`_link_mock` shim in cross-cutting.md) |
| Modularisation | 4 (engine, lp.rs, LpBacktestPage, subgraph) — plus 11 leave-as-is verdicts |
| Pattern Extraction | 0 (the audit could have surfaced the formatUsd duplication as Pattern Extraction, but it's already documented as `Aurix/Gaps.md` Gap 4 — the audit cross-references rather than re-derives) |
| Algorithm Optimisation | 2 (math.md tick.rs simplification, backtest.md fees-USD incremental accumulation) |
| Performance Improvement | 4 (math.md magic-precompute, backtest.md hold-only hoist, ipc-commands.md reqwest::Client reuse, ingest.md int24 parse) |
| Data Layout and Memory Access Patterns | 1 (backtest.md pre-parse swap rows) |
| Inconsistent Patterns | 2 (ipc-commands.md eprintln, cross-cutting.md console.log vs telemetry) |
| Dependency Hygiene | 0 |
| Configuration Drift | 0 (the hardcoded `220_000` gas estimate is documented in `Aurix/Gaps.md` Gap 7; not re-derived here) |
| API Surface Bloat | 0 |
| Test Coverage Gaps | 1 (frontend.md vitest setup) |
| Documentation Rot | 1 (cross-cutting.md systems/ docs gap) |
| Complexity Hotspots | 0 |
| Known Issues and Active Risks | 0 (no pre-existing test failures; 7 known gaps documented in vault are cross-referenced, not re-issued) |

## Data Layout applicability per system

The taxonomy mandates that Data Layout / Memory Access Patterns analysis is applied to every system audited in Pass 2. Per-system applicability:

| System | Data Layout finding? | Justification |
|---|---|---|
| Math primitives | Implicit — the `Lazy<[BigUint; 20]>` finding in math.md is technically a Performance Improvement (per category §6) but skirts Data Layout territory (allocation pattern reuse). Filed under Performance per the `mul_div` discriminator: "if the win comes from doing less work … Performance"; the magic constants are *unchanged in layout*, only the *frequency of allocation* is reduced. | The `bigint_to_u128` helper in `liquidity.rs:172-180` is the only place layout matters — it walks `to_u64_digits` to fit in u128. Audit confirmed this is correct for the cases (≤2 digits) and not a finding. |
| Storage | No finding. | Storage uses TEXT decimal columns for big integers per the precision-preservation comment. The audit's view: this is a Performance/Data-Layout discussion (BLOB vs TEXT for u256 columns) but the change is too large for a "free" finding — would touch all CRUD modules and the IPC contract. Recorded in backtest.md as Option B inside the pre-parse finding. |
| Backtest | YES — backtest.md "Pre-parse swap rows once" is a Data Layout finding (the layout question is "should the loop walk parsed structs or strings?"). | The per-iteration string→BigUint parse is the layout problem. Filed as Data Layout. |
| Ingest | No finding (the int24 finding is Performance — fewer bytes parsed, not different layout). | The ingest data pipeline is event-shaped; per-event allocations are bounded and the finding is in `parse_int24_word`'s constant-overhead reduction, not in the structural layout of the data. |
| IPC commands | No finding. | Commands are thin wrappers; data layout is downstream concern (in storage / backtest). |
| Frontend | No finding. | TypeScript engines have less explicit layout control; the audit didn't surface a layout-class win. |
| Cross-cutting | No finding. | Cross-cutting findings are documentation / inconsistency class. |

## Pre-existing failures recorded as Known Issues findings

None. The backend test suite is fully green (139 pass, 0 fail). The 3 ignored tests are correctly gated for live-network use.

## Production source changes

Verified via `git status -sb` and `git diff HEAD --stat`: the audit modified zero production source files. All audit output lives in `context/plans/code-health-audit/`. The transient `tests/audit_baselines.rs` was created for diagnostic purposes, failed to compile due to private modules in `lib.rs`, was removed within the audit, and does not appear in the final tree.
