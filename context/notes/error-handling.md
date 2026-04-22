# Error Handling

## Current Understanding

Each Rust module owns its own `thiserror::Error` enum. Adapter error types wrap the shared transport error with `#[error(transparent)]` rather than collapsing everything into a single project-wide error.

Observed enums:

| Module | Error type | Wraps |
| --- | --- | --- |
| `src-tauri/src/config.rs` | `ConfigError` | Nothing; only `MissingEnvironmentVariable(&'static str)` |
| `src-tauri/src/ethereum/client.rs` | `EthereumRpcError` | `reqwest::Error` via `#[from]` |
| `src-tauri/src/dex/uniswap_v2.rs` | `UniswapV2Error` | `EthereumRpcError` via `#[error(transparent)] #[from]` |
| `src-tauri/src/dex/uniswap_v3.rs` | `UniswapV3Error` | `EthereumRpcError` via `#[error(transparent)] #[from]` |

At the command boundary, `fetch_market_overview` calls `.map_err(|error| error.to_string())?` on every concurrent future and returns `Result<MarketOverview, String>`. The stringification is deliberate — the Tauri IPC boundary serialises the error to the frontend as plain text, so structured error data ends at the command layer.

## Guiding Principles

- New Rust modules should define their own `#[derive(Debug, Error)]` enum rather than adding variants to an existing one.
- When a new module calls through `EthereumRpcClient`, wrap the transport error with `#[from] EthereumRpcError` and `#[error(transparent)]` so the underlying cause surfaces unchanged.
- Stringification belongs at the Tauri command boundary only. Anything deeper should preserve the structured enum so matching on specific variants remains possible.
- If an error needs to carry dynamic context (e.g. which pool address failed), add a variant with owned data (`String`, `u64`) rather than borrowing — the error crosses task boundaries through `tokio::join!`.

## Rationale

The per-module split is a deliberate trade: modules remain independent units and can evolve their failure model without churning a shared error type. The cost is a few extra `#[from]` conversions, which are cheap and obvious. This matches the project's general preference for narrow, composable surfaces over grand-unifying abstractions (see `CLAUDE.md` §Engineering Standards, "modularity and toggleability").

## What Was Tried

Not applicable — the project has only ever used this shape. No previous grand-unified `Error` type exists in the git history.
