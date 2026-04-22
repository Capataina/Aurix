# Arbitrage Market Data

## Scope / Purpose

- This document owns the backend market-ingestion path for the current arbitrage screen: DEX-specific reads, gas-price reads, command assembly, and the constraints around turning live Ethereum state into one comparable market overview.

## Boundaries / Ownership

- This file owns `src-tauri/src/commands/market.rs` plus the DEX adapter modules in `src-tauri/src/dex/`.
- It also owns the backend-side meaning of the current venue set, pair hard-coding, and snapshot timestamp generation.
- It does not own the shared RPC transport itself, cross-runtime contract definitions, or frontend analytics derived after the overview reaches React.

## Current Implemented Reality

- The backend reads four live Ethereum mainnet venues on every refresh: Uniswap V3 `5bps`, Uniswap V3 `30bps`, Uniswap V2, and SushiSwap.
- `commands/market.rs` constructs a single `EthereumRpcClient`, launches all four venue fetches plus a gas-price read concurrently with `tokio::join!`, and fails fast if any one of them errors.
- The command returns a single `MarketOverview` with one chain label, one pair label, one gas price, and a `venues` array ordered as V3 5bps, V3 30bps, Uniswap V2, then SushiSwap.
- `dex/uniswap_v3.rs` reads canonical pool addresses directly, calls `slot0()`, decodes `sqrtPriceX96`, and derives the WETH price in USDC terms from fixed token decimal assumptions.
- `dex/uniswap_v2.rs` resolves pool addresses from factory contracts at runtime, reads `token0()` and `getReserves()`, then derives price from reserve ratios with a decimal adjustment between USDC and WETH.
- Both adapter families stamp `fetched_at_unix_ms` from local system time inside the adapter rather than receiving a command-level shared timestamp.

## Key Interfaces / Data Flow

| Step | Owner | Behaviour |
| --- | --- | --- |
| Market request entry | `commands::market::fetch_market_overview` | Creates the shared RPC client and orchestrates one sampling tick |
| V3 spot-price read | `dex/uniswap_v3.rs` | Reads `slot0()` from a known pool and converts `sqrtPriceX96` into a price |
| V2 reserve read | `dex/uniswap_v2.rs` | Resolves pair address from a factory, reads reserves, and derives a reserve-ratio price |
| Gas read | `EthereumRpcClient::gas_price_gwei()` via command | Reads `eth_gasPrice` and converts wei to gwei |
| Overview assembly | `commands::market.rs` | Normalises results into one `MarketOverview` for the frontend |

- The command currently acts as the orchestration boundary; adapters remain narrowly focused on protocol-specific decoding.
- V2 and V3 remain separate modules because their discovery paths, calldata, and decoding failure modes are materially different.

## Implemented Outputs / Artifacts

- `src-tauri/src/commands/market.rs` provides the sole market overview command exposed to Tauri.
- `src-tauri/src/dex/uniswap_v3.rs` provides the canonical Uniswap V3 WETH/USDC readers.
- `src-tauri/src/dex/uniswap_v2.rs` provides the Uniswap V2 and SushiSwap WETH/USDC readers.
- `src-tauri/src/dex/mod.rs` exposes those adapters to the command module.

## Known Issues / Active Risks

- One venue failure currently fails the whole market overview, so the GUI cannot show partial success or source-specific degradation.
- The command-level timestamp is taken from the first V3 snapshot rather than from an explicit orchestration timestamp, and each adapter captures time independently anyway.
- Prices are hard-coded to one chain and one pair, so there is no protection yet against the assumptions leaking into later pair-selection work.
- The gas read is a single-point estimate only and should not be treated as a robust execution model.
- The V3 price conversion uses `BigUint` to `f64` conversion, which is acceptable for the current dashboard but introduces precision risk if the backend is later used for stricter quantitative modelling.
- `dex_name` string values (`"Uniswap V3 5bps"`, `"Uniswap V3 30bps"`, `"Uniswap V2"`, `"SushiSwap"`) are an implicit cross-system contract. The GUI's `VENUES` array in `src/features/arbitrage/ArbitragePage.tsx` and `SERIES_META` in `src/features/arbitrage/components/MarketChart.tsx` both use these exact strings as lookup keys for price binding and chart-line colour mapping. Renaming a label here without updating both frontend modules silently drops the rendered price and the chart colour for that venue — see `systems/arbitrage-gui.md` and `architecture.md` §Critical Paths and Blast Radius.
- The rustdoc on `commands::market::fetch_market_overview` still says "three live venue snapshots" at `src-tauri/src/commands/market.rs:11`; the code actually returns four. This is one of three stale-copy sites flagged in `architecture.md` §Structural Notes.

## Partial / In Progress

- The venue set is already beyond the README's earliest milestone, but the ingestion path is still fixed and non-configurable.
- Concurrent reads are implemented, but failure isolation, stale-source handling, and historical storage are not.

## Planned / Missing / Likely Changes

- Add venue-level error handling so the command can surface partial results and explicit source-health status.
- Introduce persistence before any context document describes the repository as having historical market data rather than session-only analytics.
- Revisit timestamp handling if downstream consumers need a single command-level sampling moment instead of per-adapter capture times.
- Add pair configurability only after the current hard-coded path remains auditable and the payload contract can represent that extra state cleanly.

## Durable Notes / Discarded Approaches

- Price correctness matters more than venue count in this repository because Aurix is positioned as an analytics surface rather than a decorative market board.
- Keeping V2-style and V3-style decoding separate is a durable boundary worth preserving; trying to flatten them into one generic adapter would hide protocol-specific assumptions and make failures harder to reason about.

## Obsolete / No Longer Relevant

- Earlier single-venue assumptions are no longer true.
- Any claim that local snapshot persistence or historical market storage already exists is obsolete; the codebase still performs live reads only.
