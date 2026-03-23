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

- `src/main.tsx` boots React and applies the shared stylesheets.
- `src/App.tsx` provides the current single-screen root.
- `src-tauri/src/main.rs` and `src-tauri/src/lib.rs` provide the desktop process entrypoints.
- `src-tauri/src/config.rs` provides environment resolution for mainnet RPC access.
- `src-tauri/src/ethereum/client.rs` provides the low-level RPC transport reused by all adapters.
- `src-tauri/src/market/types.rs` and `src/features/arbitrage/types.ts` provide the current cross-runtime payload contract.

## Known Issues / Active Risks

- Configuration support is hard-coded to direct RPC URLs or Alchemy, so swapping providers later will require editing this shared layer rather than configuration-only changes.
- The runtime contract has no explicit stale-data, partial-success, or per-venue health fields, which limits how the frontend can explain backend failures.
- The shared payload uses floating-point values for prices and gas, which is convenient for presentation but not ideal for precision-critical historical or execution-grade logic.
- There is no automated verification around env fallback, JSON-RPC error decoding, or serialisation compatibility.

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

## Obsolete / No Longer Relevant

- The default Tauri greeting scaffold is no longer part of the runtime structure.
- Describing Aurix as a generic multi-tab platform today would be inaccurate; the codebase currently exposes one feature path only.
