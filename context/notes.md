# Notes

- [rust-doc-style](notes/rust-doc-style.md) — backend rustdoc uses a four-line `Inputs:/Outputs:/Errors:/Side effects:` contract on public items
- [error-handling](notes/error-handling.md) — one `thiserror::Error` enum per module; adapter errors wrap transport with `#[error(transparent)]`
- [wire-convention](notes/wire-convention.md) — Rust ↔ TypeScript payloads bridge via `#[serde(rename_all = "camelCase")]`; prices and gas use `f64` across the boundary
- [dex-name-contract](notes/dex-name-contract.md) — `dex_name` strings are the implicit cross-system identity key; GUI metadata and chart colouring both depend on exact match
- [no-inline-rationale](notes/no-inline-rationale.md) — the codebase has no `WHY`/`NOTE`/`TODO`/`HACK` annotations and thin commit bodies; design rationale lives in `context/`, not in code comments
