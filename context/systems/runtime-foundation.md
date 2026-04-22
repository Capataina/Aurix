# Runtime Foundation

## Scope / Purpose

- This document owns the shared runtime substrate beneath the current arbitrage feature: application entrypoints, environment-backed RPC configuration, the reusable Ethereum transport, and the cross-boundary market payload contract.

## Boundaries / Ownership

- This file owns the parts of the repository that every live read currently depends on before any feature-specific analytics or UI interpretation begins.
- It includes `src/main.tsx`, `src/App.tsx`, `src-tauri/src/lib.rs`, `src-tauri/src/config.rs`, `src-tauri/src/ethereum/client.rs`, and the Rust and TypeScript market type definitions.
- It does not own protocol-specific DEX decoding, feature composition, chart logic, or CSS-heavy presentation concerns; those live in the arbitrage system documents.

## Current Implemented Reality

- `src/App.tsx` mounts one feature page and does not provide routing, tab navigation, or a broader application shell abstraction.
- `src-tauri/src/lib.rs` registers a single IPC command, `fetch_market_overview`, and initialises only the opener plugin.
- `src-tauri/src/config.rs` resolves the backend RPC endpoint from `MAINNET_RPC_URL` first, then falls back to constructing an Alchemy mainnet URL from `ALCHEMY_API_KEY`.
- The dotenv bootstrap is process-wide and one-time, using `.env` in the backend directory and `../.env` as a fallback path.
- `src-tauri/src/ethereum/client.rs` provides the shared read-only JSON-RPC client with `eth_call` and `eth_gasPrice` support.
- `src-tauri/src/market/types.rs` defines the serialised `PriceSnapshot` and `MarketOverview` payloads, and `src/features/arbitrage/types.ts` mirrors those fields on the frontend.

## Key Interfaces / Data Flow

| Interface | Direction | Current contract |
| --- | --- | --- |
| `AppConfig::from_environment()` | Process env -> backend runtime | Resolves one Ethereum mainnet RPC URL or errors if neither supported env var exists |
| `EthereumRpcClient::eth_call()` | Backend modules -> Ethereum JSON-RPC | Sends read-only calldata and returns raw hex payloads |
| `EthereumRpcClient::gas_price_gwei()` | Backend modules -> Ethereum JSON-RPC | Returns the latest gas price converted to gwei as `f64` |
| `MarketOverview` / `PriceSnapshot` | Rust backend -> TypeScript frontend | Carries chain label, pair label, timestamp, gas price, and per-venue snapshot fields |
| `fetchMarketOverview()` in `api.ts` | Frontend -> Tauri IPC | Requests the current market overview without frontend-side parameters |

- The runtime contract is deliberately small: one command, one shared transport, and one payload family.
- Timestamp, chain, and pair labels are currently embedded in the market payload rather than managed by a broader app-state layer.

## Implemented Outputs / Artifacts

| Artifact | Role | Consumed by |
| --- | --- | --- |
| `src/main.tsx` | Boots React and applies the shared stylesheets (`theme.css`, `dashboard.css`) | Browser shell (via `index.html`) |
| `src/App.tsx` | Single-screen React root that mounts `ArbitragePage` | `main.tsx` |
| `src-tauri/src/main.rs` | Desktop binary entrypoint; delegates to `aurix_lib::run()` | OS process loader |
| `src-tauri/src/lib.rs` | Tauri builder; registers `fetch_market_overview` and the opener plugin | `main.rs` |
| `src-tauri/src/config.rs` | Environment resolution for mainnet RPC access | `commands/market.rs` |
| `src-tauri/src/ethereum/client.rs` | Low-level JSON-RPC transport (`eth_call`, `eth_gasPrice`) | Every DEX adapter + the market command |
| `src-tauri/src/market/types.rs` | Rust-side `MarketOverview`/`PriceSnapshot` definitions (snake_case; camelCase at wire) | Every backend adapter + `commands/market.rs` |
| `src/features/arbitrage/types.ts` | TypeScript mirror of the market payload | Every frontend file that touches market data |

The runtime substrate below the feature layer is a narrow line:

```text
.env / ALCHEMY_API_KEY / MAINNET_RPC_URL
       |
       v (Once-guarded dotenv load)
  AppConfig::from_environment  ----->  one EthereumRpcClient  ----->  DEX adapters
                                                         `----->  gas_price_gwei
                                                                        |
                                                                        v
                                                             MarketOverview (camelCase JSON)
                                                                        |
                                                                        v
                                                             React ArbitragePage state
```

## Known Issues / Active Risks

- Configuration support is hard-coded to direct RPC URLs or Alchemy, so swapping providers later will require editing this shared layer rather than configuration-only changes.
- The runtime contract has no explicit stale-data, partial-success, or per-venue health fields, which limits how the frontend can explain backend failures (see `systems/arbitrage-market-data.md` for how this propagates).
- The shared payload uses floating-point values for prices and gas, which is convenient for presentation but not ideal for precision-critical historical or execution-grade logic.
- There is no automated verification around env fallback, JSON-RPC error decoding, or serialisation compatibility.
- A single RPC endpoint backs every feature in the product. If the endpoint rate-limits, drops, or changes its JSON-RPC semantics, every venue read and the gas read fail simultaneously — the 1 Hz loop surfaces a continuous error banner with no alternate path. This is the most consequential external dependency in the repository and is called out in `architecture.md` §Critical Paths and Blast Radius.
- The Rust ↔ TypeScript payload mirror has no automated contract check. A field rename in `src-tauri/src/market/types.rs` compiles clean on both sides and fails silently at runtime unless `src/features/arbitrage/types.ts` is updated in the same change. The Serde `rename_all = "camelCase"` attribute is the only wire-level convention that bridges the two.

## Partial / In Progress

- The current runtime foundation is sufficient for one feature, but it has not yet been generalised into reusable command boundaries for additional tabs.
- The Tauri layer is functional rather than product-polished; the opener plugin is configured even though the current UI does not appear to expose opener-driven behaviour.

## Planned / Missing / Likely Changes

- Introduce richer backend status fields when venue-level failure isolation or stale-state handling is implemented.
- Add shared command boundaries only when a second implemented feature genuinely needs them; creating platform abstractions earlier would be speculative.
- Add tests around configuration fallback and RPC decoding once these interfaces stabilise enough to justify them.
- Revisit the payload contract if persistence, configurable pairs, or more precise analytics require stronger typing than today's presentation-oriented `f64` fields.

## Durable Notes / Discarded Approaches

- The runtime layer is intentionally read-only and should stay separate from any future write-capable wallet or transaction flow unless the project scope itself changes at the README level.
- The shared RPC client already isolates JSON-RPC transport from protocol decoding, which is worth preserving because V2 and V3 readers have different decoding paths and failure modes.
- `ENVIRONMENT_BOOTSTRAP: Once` in `config.rs` guarantees `.env` loading is idempotent for the process lifetime. This matters because `from_environment()` is called on every `fetch_market_overview` invocation (once per second); without the `Once`, dotenv files would be re-parsed every tick. Before changing this to a per-call load or moving env resolution elsewhere, confirm the 1 Hz cadence has changed — otherwise the current shape is deliberately efficient.
- The rustdoc style across backend modules (`config.rs`, `ethereum/client.rs`, `commands/market.rs`, `dex/uniswap_v3.rs`) uses an explicit four-line contract: `Inputs:`, `Outputs:`, `Errors:`, `Side effects:`. See `notes/rust-doc-style.md`. This is informal but consistent; future code should follow it to keep the public surface self-describing.
- Error handling is one `thiserror::Error` enum per module, with `#[error(transparent)]` wrapping `EthereumRpcError` in adapter error enums. See `notes/error-handling.md`. This keeps boundaries self-contained and avoids a grand unified error type.

## Obsolete / No Longer Relevant

- The default Tauri greeting scaffold is no longer part of the runtime structure.
- Describing Aurix as a generic multi-tab platform today would be inaccurate; the codebase currently exposes one feature path only.
