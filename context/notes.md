# Notes

- [dex-name-contract](notes/dex-name-contract.md) — `dex_name` strings are the implicit cross-system identity key; GUI metadata and chart colouring both depend on exact match
- [error-handling](notes/error-handling.md) — one `thiserror::Error` enum per module; adapter errors wrap transport with `#[error(transparent)]`
- [free-data-fallback-chain](notes/free-data-fallback-chain.md) — only free no-key data sources; tiered ingest fallback ends in empty state, never synthetic
- [idempotent-runs](notes/idempotent-runs.md) — `config_hash` keying + `INSERT OR IGNORE` make every backend run idempotent; load-bearing for StrictMode + re-run UX
- [lp-backtester-data-sources](notes/lp-backtester-data-sources.md) — per-IPC data source mapping (subgraph → Alchemy → public RPC → error; DefiLlama for prices; FRED for TradFi)
- [no-inline-rationale](notes/no-inline-rationale.md) — the codebase has no `WHY`/`NOTE`/`TODO`/`HACK` annotations and thin commit bodies; design rationale lives in `context/`, not in code comments
- [no-synthetic-in-user-facing](notes/no-synthetic-in-user-facing.md) — synthetic data permitted only in tests + dev-only IPCs; auto-run pipeline ends in error, never fabricated numbers
- [round-trip-fee-math](notes/round-trip-fee-math.md) — fee math semantics for V3 round-trip swap simulation
- [rust-doc-style](notes/rust-doc-style.md) — backend rustdoc uses a four-line `Inputs:/Outputs:/Errors:/Side effects:` contract on public items
- [storage-conventions](notes/storage-conventions.md) — INSERT OR IGNORE idempotency, lowercase pool addresses, SYNTHETIC_TX_HASH separation, TEXT decimal for >64-bit integers
- [wire-convention](notes/wire-convention.md) — Rust ↔ TypeScript payloads bridge via `#[serde(rename_all = "camelCase")]`; prices/gas use `f64`; raw integers use TEXT decimal strings
