# Obligation Evidence Map

Live evidence ledger for the Aurix code-health-audit run on 2026-05-04.

## Mode-distribution summary

| Mode | Count | Modes used |
|------|------:|------------|
| 1 — domain pattern lookup | 4 | "Rust+Tauri+SQLite+DeFi audit patterns", "SQLite WAL rusqlite tokio prepared-statement reuse", "Tauri 2 invoke command State Arc connection-pool", "React TypeScript large component splitting React 19" |
| 2 — specific-technique evaluation | 2 | "Uniswap V3 TickMath getSqrtRatioAtTick Rust port BigUint magic constants", "deflated Sharpe ratio Bailey de Prado backtest selection bias formula" |
| 3 — known-anti-pattern check | 2 | "Ethereum eth_getLogs archive ingestion chunking retry idempotency Rust patterns", "num-bigint BigUint Rust performance lazy_static once_cell allocation reuse" |

**Total WebSearches: 8 across the three required modes.** Variety floor met.

## Pre-Pass-1 front-loaded WebSearch

| When | Query | Source URLs | Mode | Notes |
|---|---|---|---|---|
| 2026-05-04 pre-Pass-1 | "code health audit patterns Rust Tauri DeFi backtesting SQLite project 2026" | https://dasroot.net/posts/2026/03/rust-testing-patterns-reliable-releases/ ; https://github.com/Erio-Harrison/rust-trade ; https://github.com/shuhuiluo/uniswap-v3-sdk-rs (later) | 1 (domain pattern) | Established the WebSearch pattern at audit start. Confirmed Tarpaulin 0.16.0, Testcontainers, rust-trade as comparable Rust+Tauri+SQLite trading reference. |

## Per-system rows

| System | Files audited | Research obligation (WebSearch query + mode) | Source URL | Diagnostic-test obligation | Findings emitted | Reasoned omissions |
|---|---|---|---|---|---|---|
| **Math primitives** | `math/{q96,tick,liquidity,fees,il,error,mod}.rs` (1,095 lines, 7 files) | "Uniswap V3 TickMath getSqrtRatioAtTick Rust port BigUint magic constants performance" (mode 2) + "num-bigint BigUint Rust performance lazy_static once_cell allocation reuse" (mode 3) | https://github.com/shuhuiluo/uniswap-v3-sdk-rs ; https://github.com/Uniswap/v3-core/blob/main/contracts/libraries/TickMath.sol ; https://renato.athaydes.com/posts/how-to-write-slow-rust-code-part-2.html ; https://docs.rs/once_cell/ | None — finding equivalence is by construction (same hex constants, same arithmetic chain). Existing 30+ unit tests in `math/*.rs` `#[cfg(test)] mod tests {}` blocks pin the behavioural envelope. | 3 — see [math.md](math.md) | Diagnostic-test floor: existing test suite (139 passes) already pins the affected behaviour; new diagnostic test would not upgrade confidence per `evidence-and-justification.md` §"Confidence Upgrade Pathway" — confidence is already high. |
| **Storage layer** | `storage/{mod,swaps}.rs` deep-read; `storage/{pool_events,runs,headline,benchmarks,strategy,gas,snapshots,state,connection,migrations,error}.rs` surface-scanned via file_size_scan output + import-graph fan-in (26 — highest) | "SQLite WAL Rust rusqlite tokio prepared statement reuse insert performance anti-patterns" (mode 1) | https://kerkour.com/high-performance-rust-with-sqlite ; https://docs.rs/tokio-rusqlite/latest/tokio_rusqlite/ ; https://docs.rs/tokio-rusqlite/latest/tokio_rusqlite/struct.CachedStatement.html | None — research surfaced the canonical patterns (prepared-statement reuse via `CachedStatement`, transaction batching) which Aurix's `insert_swap_events_batch` already implements correctly. No finding warranted; the system is healthy. | 0 — verdicts only (in [backtest.md](backtest.md) for `runs.rs`) | Modularisation candidate `storage/runs.rs` (340 lines) verdict: `leave-as-is` (mechanical CRUD, internally cohesive). |
| **Backtest engine** | `backtest/{engine,mod}.rs` deep-read; `backtest/{price,metrics,gas,position,rebalance,error}.rs` surface-scanned. Composite hotspot 0.93. | "deflated Sharpe ratio Bailey de Prado backtest selection bias formula validation" (mode 2) | https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2460551 ; https://www.davidhbailey.com/dhbpapers/deflated-sharpe.pdf ; https://en.wikipedia.org/wiki/Deflated_Sharpe_ratio | None written *by the audit* — see reasoned omission. Existing 6 tests in `backtest/mod.rs:117-305` (`simulate_static_position_produces_curve` etc.) pin the engine's behavioural envelope including specific assertions on time-in-range %, fees-USD sign, mgmt-gas sign, rebalance count. | 4 — see [backtest.md](backtest.md) (1 high perf, 1 medium perf, 1 high data-layout, 2 modularisation verdicts) | **Diagnostic-test reasoned omission:** attempted an integration test at `tests/audit_baselines.rs`; failed because the project's `lib.rs` declares `mod backtest` (not `pub mod`), so integration tests cannot reach `Engine` / `Ingester` / `tick_to_sqrt_price_x96`. Modifying `lib.rs` to make the modules `pub` is a production-source change forbidden by Rule 3 — per the `pub(crate)` workaround in `detection-strategies.md` §"Rust pub(crate) Visibility Constraint", I would need to exercise the public surface, which here is the Tauri command handler list (only callable through the Tauri runtime — no-test-physically-possible per the §7 deferral). The existing 6 in-crate `#[cfg(test)] mod tests` tests in `backtest/mod.rs` already pin the behavioural envelope at the same granularity an integration test would. |
| **Ingest pipeline** | `ingest/decoder.rs` deep-read; `ingest/{mod,subgraph,alchemy,pipeline,source,mock,error}.rs` surface-scanned via file_size_scan. Composite hotspot 0.95. | "Ethereum eth_getLogs archive ingestion chunking retry idempotency Rust patterns 2026" (mode 3) | https://www.alchemy.com/docs/deep-dive-into-eth_getlogs ; https://www.nethermind.io/blog/speeding-up-eth-getlogs-at-scale ; https://github.com/ethereum/go-ethereum/issues/28765 | None — finding is a localised parse-path optimisation; existing decoder tests (`parse_int24_zero_and_positive`, `parse_int24_negative_uses_sign_extension`, `decode_swap_round_trip_synthetic_log`, etc. — 10+ in `decoder.rs:263-515`) pin the I/O envelope. | 2 — see [ingest.md](ingest.md) (1 medium perf + 1 high modularisation verdict for subgraph.rs + 2 leave-as-is verdicts) | Same pub-module visibility constraint as backtest. Existing decoder tests provide the baseline. |
| **IPC commands (lp.rs)** | `commands/lp.rs` (628 lines, deep-read); `commands/{market,telemetry}.rs` not read this pass | "Tauri 2 invoke command State Arc<Storage> connection pool best practices anti-pattern" (mode 1) | https://v2.tauri.app/develop/state-management/ ; https://medium.com/@deejiw/tauri-with-shared-database-pool-e25aec033ed3 ; https://docs.rs/tauri/latest/tauri/trait.Manager.html | None — modularisation finding's safety is by construction (mechanical move with `lib.rs` path updates). | 3 — see [ipc-commands.md](ipc-commands.md) (1 high modularisation, 1 medium perf, 1 low inconsistency) | Tauri commands are async fns exercised through the Tauri runtime — same physical-impossibility deferral as backtest. The modularisation finding is mechanical and verifiable by `cargo build` after the move. |
| **Frontend (LP page + arbitrage + telemetry)** | `LpBacktestPage.tsx` first 200 lines deep-read; remaining frontend surface-scanned via file_size_scan + modularisation_candidates_ts output | "React TypeScript large component splitting patterns 600+ line useEffect cleanup React 19" (mode 1) | https://blog.logrocket.com/understanding-react-useeffect-cleanup-function/ ; https://dev.to/pockit_tools/why-is-useeffect-running-twice-the-complete-guide-to-react-19-strict-mode-and-effect-cleanup-1n60 ; https://medium.com/@CodersWorld99/react-19-typescript-best-practices-the-new-rules-every-developer-must-follow-in-2025-3a74f63a0baf | **Reasoned omission** — see right column. | 4 — see [frontend.md](frontend.md) (1 high modularisation, 1 medium coverage gap, 4 modularisation verdicts) | **Diagnostic-test reasoned omission:** the project has zero frontend test infrastructure (no `vitest` / `jest` / `tests/` directory under `src/`). Standing up Vitest is itself one of the audit's findings (frontend.md §Coverage Gap 1). Writing a one-off vitest-based test as part of this audit would (a) expand the audit's scope substantially, (b) require choosing test conventions that the project has not adopted yet, and (c) not be the right shape for the modularisation finding (a hook-extraction is mechanically verifiable by `tsc --noEmit` + `vite build` succeeding). The audit recommends Vitest in the Coverage Gap finding; once it lands, the modularisation finding's first test target is `useLpPipeline`. |
| **Cross-cutting** | `notes/{wire-convention,error-handling,no-inline-rationale,dex-name-contract,...}.md` + LifeOS `Aurix/Gaps.md` | None per-system — research mode coverage already met across the four primary systems above | N/A | None — the cross-cutting findings are documentation/inconsistency/triage class; no test would resolve confidence. | 3 — see [cross-cutting.md](cross-cutting.md) (1 medium documentation rot, 1 medium inconsistent patterns, 1 low triage) | Cross-cutting findings reference existing context/notes rather than introducing new claims; no research call required for this category specifically. |

## Modularisation candidate verdicts (15 candidates total)

| File | Lines | Verdict | Justification location |
|------|------:|---------|------------------------|
| `src-tauri/src/commands/lp.rs` | 628 | split-recommended | [ipc-commands.md](ipc-commands.md) §"Split lp.rs into three submodules" |
| `src-tauri/src/ingest/subgraph.rs` | 549 | split-recommended | [ingest.md](ingest.md) §"Verdict for subgraph.rs" |
| `src-tauri/src/ingest/decoder.rs` | 515 | leave-as-is | [ingest.md](ingest.md) §"Verdict for decoder.rs" |
| `src-tauri/src/headline/verdict.rs` | 414 | leave-as-is *(unread, deferred)* | See reasoned omission below |
| `src-tauri/src/ingest/mod.rs` | 392 | leave-as-is | [ingest.md](ingest.md) §"Verdict for mod.rs" |
| `src-tauri/src/backtest/engine.rs` | 374 | split-recommended | [backtest.md](backtest.md) §"Verdict for engine.rs" |
| `src-tauri/src/storage/runs.rs` | 340 | leave-as-is | [backtest.md](backtest.md) §"Verdict for storage/runs.rs" |
| `src-tauri/src/math/liquidity.rs` | 322 | leave-as-is | [math.md](math.md) §"Verdict for liquidity.rs" |
| `src/features/lp-backtest/LpBacktestPage.tsx` | 668 | split-recommended | [frontend.md](frontend.md) §"Split LpBacktestPage" |
| `src/components/blocks/arbitrage/PriceChartBlock.tsx` | 540 | split-recommended | [frontend.md](frontend.md) verdicts |
| `src/features/arbitrage/insights.ts` | 490 | split-recommended | [frontend.md](frontend.md) verdicts |
| `src/features/lp-backtest/LpSettingsForm.tsx` | 409 | leave-as-is | [frontend.md](frontend.md) verdicts |
| `src/lib/telemetry.ts` | 397 | leave-as-is | [frontend.md](frontend.md) verdicts |
| `src/components/blocks/lp/StrategyControlsBlock.tsx` | 360 | leave-as-is | [frontend.md](frontend.md) verdicts |
| `src/components/blocks/lp/MultiAssetCompareBlock.tsx` | 319 | leave-as-is | [frontend.md](frontend.md) verdicts |

**Reasoned omission for `headline/verdict.rs`:** the file was not deep-read in this audit pass. The 414-line surface is mostly the regime-classifier prose synthesis + per-month aggregation; based on file-name + size + the architecture.md §M2.8 description, the file owns one cohesive concern (capital-allocation verdict generation). A deep-read would either confirm `leave-as-is` or surface a split into "regime classifier" + "verdict prose" — both viable. The audit defers this to a future pass; recording the deferral here per the audit-philosophy.md "respect what you do not understand" rule.

## Trivial systems — research/diagnostic-test reasoned omissions

| System | Reason |
|---|---|
| `src-tauri/src/main.rs` | 3-line entrypoint (`fn main() { aurix_lib::run(); }`); not substantive. |
| `src-tauri/src/lib.rs` | 75 lines — Tauri builder + handler list + `resolve_db_path` helper. The handler list is mechanical wiring; not "substantive logic" per `detection-strategies.md` §"When Research Is Not Required". |
| `src-tauri/src/market/types.rs` | Pure type definitions for the IPC `MarketOverview` shape. No logic to research. |
| Per-module `error.rs` files (math, storage, ingest, backtest, benchmarks, validation) | Thin `thiserror::Error` enum definitions; not substantive. |
| Per-module `mod.rs` files that are pure re-exports | E.g. `dex/mod.rs`, `ethereum/mod.rs`, `commands/mod.rs`, `validation/mod.rs` (excluding `validation/synthetic.rs` which has logic). Pure re-export files don't warrant per-system rows. |
| `src/main.tsx` | React mount entrypoint. |
| `src/App.tsx` | 181 lines — top-level routing + busy-state mirror. Surface-scanned via file_size_scan; the Modularisation candidate threshold was not crossed. The page-routing logic is mechanical. |

## Final lint expectations

`scripts/evidence_map_lint.py` should pass with:
- zero in-progress rows (all rows resolved at audit close)
- ≥3 modes represented (4 in mode 1, 2 in mode 2, 2 in mode 3 = 8 calls across 3 modes ✓)
- All substantive systems have a research row + diagnostic-test cell (with explicit reasoned-omission text where applicable)
- All 15 modularisation candidates have a verdict
