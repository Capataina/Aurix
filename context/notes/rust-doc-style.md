# Rust Doc Style

## Current Understanding

Every public function on the Rust backend that is non-trivial has a rustdoc block with a one-line summary followed by a fixed four-line contract:

```rust
/// Loads backend configuration from process environment or local dotenv files.
///
/// Inputs: none; reads environment variables from the current process and
/// optional `.env` files.
/// Outputs: a validated configuration object containing the resolved RPC URL.
/// Errors: returned when neither `MAINNET_RPC_URL` nor `ALCHEMY_API_KEY` is
/// available.
/// Side effects: lazily loads dotenv files once per process.
pub fn from_environment() -> Result<Self, ConfigError> { ... }
```

Observed in: `src-tauri/src/config.rs`, `src-tauri/src/ethereum/client.rs`, `src-tauri/src/commands/market.rs`, `src-tauri/src/dex/uniswap_v3.rs`. Adapter helpers in `dex/uniswap_v2.rs` omit the four-line block in favour of a one-line summary, which is acceptable for private async helpers.

## Guiding Principles

- Keep the ordering `Inputs → Outputs → Errors → Side effects`. This order matches how a reader reasons about a call site: what goes in, what comes out, what goes wrong, what mutates the world.
- Put each section on its own line prefix — the style is plain prose, not a docstring-table DSL.
- Private helpers do not need the four-line block. A single-line `/// …` summary is enough when the function is only called from one place in the same module.
- When a function has no side effects, still include the `Side effects:` line and state `none` — the absence of the field reads as "this was not considered."

## Rationale

The four-line convention emerged naturally from the initial milestone commits and has held for every module since. It keeps the public surface self-describing without introducing a docstring macro or relying on external tooling. No linter enforces it; consistency is the responsibility of whoever is extending the backend next.
