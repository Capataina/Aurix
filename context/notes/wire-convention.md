# Wire Convention

## Current Understanding

The Rust backend and the TypeScript frontend communicate through 19 Tauri IPC commands (registered in `lib.rs`'s `tauri::generate_handler!` block) carrying multiple payload families. Both runtimes maintain parallel type definitions kept in sync manually:

| Family | Rust types | TS mirror |
|---|---|---|
| Tab 1 — Arbitrage | `src-tauri/src/market/types.rs` (`MarketOverview`, `PriceSnapshot`) | `src/features/arbitrage/types.ts` |
| Tab 2 — LP backtest | inline DTOs in `src-tauri/src/commands/lp.rs` (`PoolMetadataDto`, `FirstSwapInfo`, `TokenPricesDto`, `BacktestResponse`, `CommandError`) + types in `src-tauri/src/backtest/position.rs` (`PositionConfig`), `src-tauri/src/backtest/rebalance.rs` (`RebalanceRule`), `src-tauri/src/storage/runs.rs` (`PositionRunSummary`, `EquityCurvePoint`), `src-tauri/src/storage/strategy.rs` (`StrategyResultRow`), `src-tauri/src/storage/headline.rs` (`HeadlineMonthlyRow`), `src-tauri/src/storage/benchmarks.rs` (`BenchmarkPoint`), `src-tauri/src/strategies/grid.rs` (`GridConfig`), `src-tauri/src/headline/mod.rs` (`HeadlineConfig`, `HeadlineRunSummary`), `src-tauri/src/ingest/mod.rs` (`IngestionReport`, `PoolMetadata`) | `src/features/lp-backtest/types.ts` (217 lines) |
| Telemetry | `src-tauri/src/commands/telemetry.rs` (`TelemetryEvent`) | `src/lib/telemetry.ts` |

The bridge is a single Serde attribute on the Rust side:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionConfig { ... }
```

This rewrites `pool_address` → `poolAddress`, `tick_lower` → `tickLower`, `fee_tier_bps` → `feeTierBps`, etc. at serialisation time. The TypeScript interface uses the camelCase names directly; no runtime transformation happens on the frontend.

Numeric fields crossing the boundary use `f64` / `number` for monetary and gas values. Integers use `u16`/`u32`/`u64`/`i32`/`i64` for fee tiers, timestamps, block numbers, and ticks. Big integers (uint160 sqrtPriceX96, uint128 liquidity, int256 amount0/amount1) are encoded as **decimal strings** — see [storage](../systems/storage.md) for why the same precision-preserving encoding is used at the persistence boundary.

## Guiding Principles

- When adding a field to either type, update **both** the Rust type and `types.ts` in the same change. There is no automated contract check (`ts-rs` / `specta` not in use — see Rationale below); a single-sided change compiles clean on both runtimes and fails silently at runtime.
- Keep field names snake_case in Rust, camelCase in TypeScript. The `rename_all` attribute does the rewriting — do not work around it by writing camelCase names in Rust.
- Use `f64` for monetary values where ULP-level precision is acceptable (chart rendering, headline numbers, grid metrics). Use **decimal-string encoding** for raw integer values where precision is load-bearing (token amounts, sqrtPriceX96, liquidity). The same TEXT-encoding-for-precision rule that [storage](../systems/storage.md) applies to its persistent columns applies on the wire — keeps the two encodings consistent.
- For error responses, use `CommandError { message: string, keyRequired?: string }`. The optional `keyRequired` field communicates "this path needs an API key" so the frontend can surface a "configure your key" prompt without parsing the message string. Used by the live-Alchemy + ETH.STORE benchmark paths.
- Do not introduce a third representation (an adapter layer, a serialisation DTO separate from the domain type). The current one-hop bridge is the whole reason this convention works.

## Rationale

Serde's `rename_all` plus hand-kept TypeScript interfaces is the minimum viable bridge for a project where the type-mirror surface is medium-sized and the change cadence is bursty (one big sprint added 90% of the LP types in a single day). The alternative — code generation via `ts-rs` or `specta` — was considered but not adopted because:

1. The two type files are still small enough that a missing mirror is caught by manual testing of the dashboard during development. The 2026-05-03 sprint shipped the entire LP type set without runtime mismatches.
2. Adding codegen introduces a build step that the implementing engineer must invoke after every Rust edit; the cost-of-build-step exceeds the cost-of-vigilance for the current cadence.
3. The `serde(rename_all = "camelCase")` attribute is one line per struct and is uniformly applied; reviewing a Rust diff for matching TS edits is tractable.

If the command count grows past ~30 or the payload structure starts to nest more than 2 levels deep, revisit this decision. `ts-rs` is the canonical choice for the migration target.

## Verification Question

- Does every currently-serialised field on the LP DTOs have its camelCase twin in `types.ts`? (Answered yes as of 2026-05-04 — `PositionConfig` aligned in commit 391eadd's correctness fixes; the previously-missing `pool_address` field was added there.)
- Does the telemetry event shape match between TS and Rust? (Yes — see [telemetry](../systems/telemetry.md).)
