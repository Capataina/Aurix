# Architecture

## Scope / Purpose

- This document defines the current top-level repository structure, subsystem boundaries, execution flow, and dependency direction for Aurix.

## Current Implemented System

- Aurix is currently a Tauri desktop application with a React frontend in `src/` and a Rust backend in `src-tauri/`.
- The repository is presently organised around a single implemented product slice: Tab 1 on Ethereum WETH/USDC venue comparison.
- The current high-level repository tree is:
  `src/` for frontend composition, feature code, and global styles.
  `src-tauri/src/` for backend commands, configuration, RPC transport, DEX adapters, and market models.
  `context/` for architecture and feature/system truth documents.
- The frontend is currently split into:
  `src/features/arbitrage/` for Tab 1 GUI logic and chart composition.
  `src/styles/` for global theme tokens and dashboard styling.
- The backend is currently split into:
  `src-tauri/src/config.rs` for environment-driven configuration.
  `src-tauri/src/ethereum/` for read-only Ethereum JSON-RPC transport.
  `src-tauri/src/dex/` for protocol-specific venue adapters.
  `src-tauri/src/market/` for normalised transport models returned to the frontend.
  `src-tauri/src/commands/` for Tauri IPC command boundaries.
- The current execution flow is:
  frontend refresh or polling trigger -> Tauri command -> config resolution -> Ethereum RPC client -> venue adapters -> normalised market overview -> frontend chart and detail panels.
- Dependency direction is one-way:
  frontend depends on normalised backend outputs.
  commands depend on config, RPC, and adapters.
  adapters depend on RPC and market types.
  shared market types do not depend on GUI-specific concerns.

## Implemented Outputs / Artifacts

- `src/App.tsx` and `src/main.tsx` mount the current desktop GUI.
- `src/features/arbitrage/` contains the current Tab 1 page, command client, and chart component wiring.
- `src/styles/theme.css` and `src/styles/dashboard.css` define shared presentation tokens and dashboard styling.
- `src-tauri/src/config.rs` resolves backend configuration for Ethereum mainnet access.
- `src-tauri/src/ethereum/client.rs` performs read-only JSON-RPC contract calls and gas-price reads.
- `src-tauri/src/dex/uniswap_v3.rs` reads Uniswap V3 WETH/USDC pools and derives spot prices from `slot0()`.
- `src-tauri/src/dex/uniswap_v2.rs` resolves and reads Uniswap V2-style WETH/USDC pools for Uniswap V2 and SushiSwap.
- `src-tauri/src/market/types.rs` defines `PriceSnapshot` and `MarketOverview`.
- `src-tauri/src/commands/market.rs` exposes the current market-overview IPC command.

## In Progress / Partially Implemented

- The GUI presents a live comparative surface for four WETH/USDC venues, but the chart behaviour still needs refinement and validation against runtime behaviour.
- The GUI maintains in-session history only; there is no local persistence layer yet.
- The backend abstraction supports future DEX adapters, but the live set is currently limited to Uniswap V3 5bps, Uniswap V3 30bps, Uniswap V2, and SushiSwap.
- The architecture is being shaped to support a future TUI surface, but no terminal interface exists yet.

## Planned / Missing / To Be Changed

- Add further venue adapters only where the market is truly comparable and the pricing logic can remain trustworthy.
- Introduce time-series persistence so charts and spread views are backed by local history rather than session-only memory.
- Separate the analytics core further so GUI and future TUI surfaces consume the same derived metrics.
- Add a TUI entrypoint that reuses the backend market and analytics layers without depending on the GUI.

## Notes / Design Considerations

- Protocol-specific details must remain inside `src-tauri/src/dex/`.
- Market models should remain presentation-ready and protocol-agnostic so both GUI and TUI surfaces can consume them.
- Shared systems that will matter across multiple tabs should be documented separately rather than buried inside one tab document.

## Discarded / Obsolete / No Longer Relevant

- The default Tauri greeting scaffold is no longer part of the project architecture.
