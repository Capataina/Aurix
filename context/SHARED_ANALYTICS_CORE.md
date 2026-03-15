# Shared Analytics Core

## Scope / Purpose

- This document tracks systems that are intended to be reused across multiple tabs or multiple presentation surfaces rather than belonging only to Tab 1.

## Current Implemented System

- The Rust backend already contains a shared configuration path that resolves Ethereum RPC access from `MAINNET_RPC_URL` or `ALCHEMY_API_KEY`.
- The Rust backend already contains a shared Ethereum JSON-RPC client for read-only contract calls and gas-price reads.
- The Rust backend already contains normalised market transport models in the form of `PriceSnapshot` and `MarketOverview`.
- The frontend already uses a centralised theme and dashboard stylesheet rather than embedding presentation rules inside components.

## Implemented Outputs / Artifacts

- `src-tauri/src/config.rs` provides backend configuration loading.
- `src-tauri/src/ethereum/client.rs` provides read-only RPC access.
- `src-tauri/src/market/types.rs` provides normalised market payloads.
- `src/styles/theme.css` provides global theme tokens.
- `src/styles/dashboard.css` provides shared dashboard presentation rules.

## In Progress / Partially Implemented

- The current market models are shared, but the derived analytics still live too close to the Tab 1 GUI.
- The current styling system is centralised, but it only reflects the first desktop dashboard rather than a broader cross-tab design system.
- The current backend abstractions are reusable, but there is no storage layer yet for shared historical data.

## Planned / Missing / To Be Changed

- Extract reusable analytics calculations into a shared core instead of keeping them in the chart component.
- Add local storage and historical snapshot retention so multiple tabs can consume common time-series data.
- Introduce shared control patterns for filters, toggles, and chart framing across tabs.
- Keep gas, price, spread, and event concepts isolated so later tabs can reuse whichever parts fit their domain.

## Notes / Design Considerations

- Shared systems should be kept protocol-agnostic where possible and product-surface-agnostic wherever reasonable.
- The future TUI should consume the same core market and analytics outputs as the GUI rather than reimplementing them.
- A shared system should only be documented here if it is genuinely cross-cutting rather than just temporarily reused.

## Discarded / Obsolete / No Longer Relevant

- There is currently no discarded shared-core system beyond the removed starter scaffold.
