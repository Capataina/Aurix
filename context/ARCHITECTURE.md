# Architecture

## Scope / Purpose

- This document is the top-down structural map of Aurix as implemented today, covering repository shape, subsystem boundaries, dependency direction, and the runtime path that turns Ethereum reads into the current desktop dashboard.

## Repository Overview

- Aurix is a local Tauri desktop application with a React 19 frontend and a Rust backend.
- The implemented product surface is currently narrower than the root README roadmap and is limited to Tab 1's live arbitrage-monitoring slice for one hard-coded pair, `WETH / USDC`.
- The repository has two runtime layers: a Vite-served frontend in `src/` and a Tauri-hosted Rust backend in `src-tauri/`.
- The current product does not include persistence, routing, background jobs, automated tests, or implementation for Tabs 2 to 5.

## Repository Structure

```text
Aurix/
├── README.md                               # Immutable project intent and roadmap
├── agents.md                               # Session workflow rules for this repository
├── package.json                            # Frontend package manifest and build scripts
├── index.html                              # Browser shell metadata still using starter defaults
├── public/
│   ├── vite.svg                            # Starter Vite asset still referenced by index.html
│   └── tauri.svg                           # Starter Tauri asset retained from scaffolding
├── src/
│   ├── main.tsx                            # React entrypoint and global stylesheet loading
│   ├── App.tsx                             # Single-screen app root
│   ├── features/
│   │   └── arbitrage/
│   │       ├── ArbitragePage.tsx           # Polling loop, rolling session history, page composition
│   │       ├── api.ts                      # Tauri IPC client for market overview reads
│   │       ├── insights.ts                 # Derived insight cards and recent-event logic
│   │       ├── types.ts                    # Frontend market payload contracts
│   │       └── components/
│   │           ├── PriceCard.tsx           # Primary venue readout and refresh state
│   │           ├── MarketChart.tsx         # SVG chart modes and event markers
│   │           └── InsightsPanel.tsx       # Live interpretation and event feed rendering
│   └── styles/
│       ├── theme.css                       # Global tokens, typography, and page background
│       └── dashboard.css                   # Dashboard layout, panels, chart, and insight styling
├── src-tauri/
│   ├── Cargo.toml                          # Rust crate manifest and dependency set
│   ├── tauri.conf.json                     # Desktop build handshake and window configuration
│   ├── capabilities/default.json           # Granted permissions for the main window
│   ├── icons/                              # Bundled desktop icon assets
│   ├── gen/schemas/                        # Generated Tauri capability and config schemas
│   └── src/
│       ├── main.rs                         # Desktop binary entrypoint
│       ├── lib.rs                          # Tauri builder and command registration
│       ├── config.rs                       # Environment-backed RPC configuration
│       ├── commands/
│       │   ├── mod.rs                      # Command module registry
│       │   └── market.rs                   # `fetch_market_overview` IPC boundary
│       ├── ethereum/
│       │   ├── mod.rs                      # Ethereum transport module registry
│       │   └── client.rs                   # Read-only JSON-RPC transport and gas reads
│       ├── dex/
│       │   ├── mod.rs                      # DEX adapter module registry
│       │   ├── uniswap_v2.rs               # Uniswap V2 and SushiSwap reserve readers
│       │   └── uniswap_v3.rs               # Uniswap V3 `slot0()` price readers
│       └── market/
│           ├── mod.rs                      # Market model module registry
│           └── types.rs                    # Normalised backend payloads returned to the GUI
└── context/
    ├── architecture.md                     # Structural repository map
    └── systems/                            # Canonical subsystem reality documents
```

## Subsystem Responsibilities

| Subsystem | Owns | Primary modules |
| --- | --- | --- |
| Desktop shell and runtime entrypoints | Application startup, IPC wiring, build handshake, window metadata, shared styling bootstrap | `src/main.tsx`, `src/App.tsx`, `index.html`, `src/styles/`, `src-tauri/src/lib.rs`, `src-tauri/tauri.conf.json` |
| Arbitrage frontend surface | Poll cadence, in-memory session history, chart mode selection, insight rendering, venue/detail panels | `src/features/arbitrage/` |
| Backend market pipeline | Configuration, Ethereum RPC transport, DEX-specific reads, gas-price reads, market overview assembly | `src-tauri/src/config.rs`, `src-tauri/src/ethereum/`, `src-tauri/src/dex/`, `src-tauri/src/commands/market.rs` |
| Cross-boundary market contract | Normalised payload shape shared between Rust and TypeScript | `src-tauri/src/market/types.rs`, `src/features/arbitrage/types.ts` |

- The frontend owns presentation and in-session interpretation only; it does not talk to Ethereum directly.
- The Rust backend owns all chain access and protocol decoding; no frontend file embeds calldata or pool math.
- The current repository shape is feature-first rather than platform-first because only one implemented feature exists.

## Dependency Direction

- `src/main.tsx` depends on `src/App.tsx`, which mounts the arbitrage feature and shared styles.
- `src/features/arbitrage/ArbitragePage.tsx` depends on the IPC client, local analytics helpers, and presentational components.
- `PriceCard.tsx`, `MarketChart.tsx`, and `InsightsPanel.tsx` depend on already-derived props and do not invoke Tauri commands themselves.
- `src-tauri/src/lib.rs` depends on the market command module and the opener plugin for runtime wiring.
- `commands/market.rs` depends on configuration, the shared Ethereum RPC client, DEX adapters, and normalised market types.
- The DEX adapters depend on the shared Ethereum client and shared payload structs, but not on frontend concerns or on each other beyond module-level coexistence.
- The shared market contract sits between the backend command and the frontend feature as the only intentional cross-runtime data boundary.

## Core Execution / Data Flow

```text
React mount
  -> ArbitragePage requests `fetch_market_overview`
  -> Tauri command loads RPC configuration
  -> one EthereumRpcClient is constructed
  -> Uniswap V3 5bps + Uniswap V3 30bps + Uniswap V2 + SushiSwap + gas price are fetched concurrently
  -> DEX-specific readers decode protocol state into `PriceSnapshot` values
  -> command assembles one `MarketOverview`
  -> frontend appends the overview to a 100-sample in-memory history
  -> chart, summary metrics, and insight cards are re-derived from that session history
  -> UI renders the latest snapshot plus historical interpretation
```

- Refresh cadence is fixed at one second in the frontend rather than being driven by a backend scheduler.
- History is session-only and disappears on restart because no persistence layer exists.
- Failure handling is still coarse-grained: any venue read failure rejects the whole command and surfaces an error banner in the GUI.

## Structural Notes / Current Reality

- The repository still contains starter scaffolding residue in `index.html`, `public/`, `src/assets/`, and metadata fields in `Cargo.toml` and `tauri.conf.json`.
- The backend returns four venue snapshots, but some user-facing copy still says "three live Ethereum venue reads", so the code and copy are slightly out of step.
- Tauri capability and icon folders are present because the desktop shell is already bundle-ready even though the product shell polish is incomplete.
- There is no test directory or automated verification layer in the repository; current correctness depends on manual inspection and build checks.
- Tabs 2 to 5 remain README intent only and should not be described elsewhere in `context/` as implemented systems.
