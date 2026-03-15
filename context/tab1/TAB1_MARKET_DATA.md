# Tab 1 Market Data

## Scope / Purpose

- This document tracks Tab 1 venue ingestion, price reads, gas reads, and the normalised market overview returned to the frontend.

## Current Implemented System

- Tab 1 currently reads four live Ethereum mainnet WETH/USDC venues.
- The implemented live venues are Uniswap V3 5bps, Uniswap V3 30bps, Uniswap V2, and SushiSwap.
- The backend also reads the current Ethereum gas price and includes it in the market overview.
- Venue reads are performed through read-only Ethereum JSON-RPC calls and normalised into a shared `MarketOverview`.

## Implemented Outputs / Artifacts

- `src-tauri/src/dex/uniswap_v3.rs` reads Uniswap V3 pool state from `slot0()`.
- `src-tauri/src/dex/uniswap_v2.rs` resolves Uniswap V2-style pairs and reads reserve-based prices.
- `src-tauri/src/commands/market.rs` gathers venue snapshots and gas price into the current market overview payload.
- `src-tauri/src/market/types.rs` defines the normalised output contract used by the frontend.

## In Progress / Partially Implemented

- Only one token pair is currently supported.
- The implemented venue set is intentionally narrow and still needs runtime validation and relevance review.
- The market overview exists only as a live in-memory feed; there is no persisted historical storage.

## Planned / Missing / To Be Changed

- Add only further venues that are genuinely comparable for the tracked pair and can be decoded reliably.
- Introduce local persistence for snapshots so the market feed is not limited to the current app session.
- Add support for configurable pairs once the current WETH/USDC path is stable.

## Notes / Design Considerations

- Venue count should not grow just for show; each added venue must improve comparison quality.
- Pool-decoding correctness matters more than the number of integrations.
- The normalised market model should remain stable enough for both GUI and future TUI consumers.

## Discarded / Obsolete / No Longer Relevant

- The earlier single-venue-only Tab 1 state is obsolete.
