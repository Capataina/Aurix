# Runtime Foundation

## Scope / Purpose

- This document owns the shared runtime substrate beneath the current arbitrage feature: application entrypoints, environment-backed RPC configuration, the reusable Ethereum transport, and the cross-boundary market payload contract.

## Boundaries / Ownership

- This file owns the parts of the repository that every live read currently depends on before any feature-specific analytics or UI interpretation begins.
- It includes `src/main.tsx`, `src/App.tsx`, `src-tauri/src/lib.rs`, `src-tauri/src/config.rs`, `src-tauri/src/ethereum/client.rs`, and the Rust and TypeScript market type definitions.
- It does not own protocol-specific DEX decoding, feature composition, chart logic, or CSS-heavy presentation concerns; those live in the arbitrage system documents.

## Current Implemented Reality

- `src/App.tsx` is the top-level application root; routes between Tab 1 (Arbitrage) and Tab 2 (LP Backtest) via the `TopBar` shell. The 2026-05-03 sprint introduced the multi-tab shell with route-aware `SettingsMenu`.
- `src-tauri/src/lib.rs` registers **19 IPC commands** across three command modules (`commands::market` for Tab 1, `commands::lp` for Tab 2's backtest pipeline, `commands::telemetry` for the cross-cutting recorder). The Tauri builder also `manage()`s an `Arc<Storage>` initialised at startup via `tokio::Builder::new_multi_thread()` + `block_on(open_storage())`.
- `src-tauri/src/config/` is now a folder (was a single file pre-sprint) with `mod.rs` (env-resolution + `AppConfig`) + `chains.rs` (`ChainId` + `Protocol` + per-chain subgraph URLs / public RPCs / block times for Ethereum / Arbitrum / Optimism / Base / Polygon).
- The dotenv bootstrap is process-wide and one-time, using `.env` in the backend directory and `../.env` as a fallback path.
- `src-tauri/src/ethereum/client.rs` provides the shared read-only JSON-RPC client with `eth_call` and `eth_gasPrice` support — used by Tab 1 and as a fallback transport for Tab 2's archive ingest path 3 (free public RPC).
- **Storage handle** initialised at startup (`lib.rs::open_storage`) at `~/.aurix/aurix.sqlite` (override via `AURIX_DB_PATH`). See [storage](storage.md) for full details. The handle is registered as Tauri-managed state and is reachable from every Tauri command via `State<'_, Arc<Storage>>`.
- `src-tauri/src/market/types.rs` still defines the Tab 1 `PriceSnapshot` and `MarketOverview` payloads. Tab 2's payload types live alongside their owning subsystem (per the convention in [storage](storage.md), [backtest](backtest.md), [strategies](strategies.md), [headline](headline.md)) — no central types module for Tab 2.

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

- The runtime foundation generalised into multi-tab + multi-command surface in the 2026-05-03 sprint. Both Tab 1 (Arbitrage) and Tab 2 (LP Backtest) now share the same Storage handle + telemetry recorder.
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
- The pre-2026-05-03 single-feature framing ("Aurix exposes one feature path only") is no longer accurate. Tab 2 (LP Backtest) shipped in the same sprint that brought the entire Vector A backend stack — see [storage](storage.md), [backtest](backtest.md), [ingest](ingest.md), [strategies](strategies.md), [benchmarks](benchmarks.md), [headline](headline.md), [validation](validation.md), [math](math.md), [lp-backtest-gui](lp-backtest-gui.md), [telemetry](telemetry.md).
