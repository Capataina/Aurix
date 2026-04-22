# Wire Convention

## Current Understanding

The Rust backend and the TypeScript frontend communicate through a single Tauri IPC command (`fetch_market_overview`) carrying two payload types: `MarketOverview` and nested `PriceSnapshot[]`. The two runtimes maintain parallel type definitions:

- Rust: `src-tauri/src/market/types.rs`
- TypeScript: `src/features/arbitrage/types.ts`

The bridge is a single Serde attribute on the Rust side:

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceSnapshot { ... }
```

This rewrites `dex_name` → `dexName`, `price_usd` → `priceUsd`, `fetched_at_unix_ms` → `fetchedAtUnixMs`, etc. at serialisation time. The TypeScript interface uses the camelCase names directly; no runtime transformation happens on the frontend.

Numeric fields crossing the boundary use `f64` / `number` — including `price_usd`, `gas_price_gwei`, and derivations. Integers use `u16`/`u64` for fee tiers and timestamps.

## Guiding Principles

- When adding a field to either type, update **both** `types.rs` and `types.ts` in the same change. There is no automated contract check; a single-sided change compiles clean on both runtimes and fails silently at runtime.
- Keep field names snake_case in Rust, camelCase in TypeScript. The `rename_all` attribute does the rewriting — do not work around it by writing camelCase names in Rust.
- Do not introduce a third representation (e.g. an adapter layer, a serialisation DTO separate from the domain type). The current one-hop bridge is the whole reason this convention works.
- Use `f64` for monetary values at this layer. Precision risk has been accepted for the current dashboard (see `systems/runtime-foundation.md` and `systems/arbitrage-market-data.md`). If stricter precision is needed later, change the Rust type first and propagate the JSON representation (`string` carrying a decimal) to the TypeScript side, updating all call sites.

## Rationale

Serde's `rename_all` plus hand-kept TypeScript interfaces is the minimum viable bridge for a project with exactly one IPC command and one payload family. The alternative — code generation from a schema (e.g. `ts-rs`, `specta`) — was not chosen because the two type files are small, the change cadence is low, and the cost of a missed mirror update is caught by manual testing of the single dashboard screen during development. Revisit this if the command count grows past a handful or if the payload structure nests more deeply.

## Verification Question

- Does every currently-serialised field on `MarketOverview` and `PriceSnapshot` have its camelCase twin in `types.ts`? (Answered yes as of 2026-04-22 — both structs have five fields, field lists match.)
