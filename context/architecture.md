# Architecture

## Scope / Purpose

- This document is the top-down structural map of Aurix as implemented today, covering repository shape, subsystem boundaries, dependency direction, and the runtime path that turns Ethereum reads into the current desktop dashboard.

## Repository Overview

- Aurix is a local Tauri desktop application with a React 19 frontend and a Rust backend.
- The implemented product surface covers **Tab 1 (live arbitrage monitor)** and **Tab 2 (LP backtester — Vector A, code-complete 2026-05-03)**. Tabs 3-5 remain README intent.
- Tab 1 reads multi-pair, multi-venue prices live (`WETH/USDC` + `WBTC/USDC` are the registered pairs as of 2026-05-02 multi-pair refactor). Tab 2 ingests historical V3 swap streams, runs position simulations + strategy grids + the M2.8 capital-allocation headline.
- The repository has two runtime layers: a Vite-served frontend in `src/` and a Tauri-hosted Rust backend in `src-tauri/`.
- The implemented backend now ships persistence (SQLite via rusqlite + WAL), automated tests (130+ unit/integration tests), and the full Vector A pipeline. Wallet (Tab 3), gas intelligence (Tab 4), and risk modelling (Tab 5) remain unimplemented.

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
| Desktop shell and runtime entrypoints | Application startup, IPC wiring, build handshake, window metadata, shared styling bootstrap, app-data DB resolution | `src/main.tsx`, `src/App.tsx`, `index.html`, `src/styles/`, `src-tauri/src/lib.rs`, `src-tauri/tauri.conf.json` |
| Tab 1 — arbitrage frontend | Poll cadence, in-memory session history, chart modes, infographic block grid, venue/detail panels | `src/features/arbitrage/`, `src/components/blocks/`, `src/components/primitives/`, `src/components/shell/` |
| Tab 1 — backend market pipeline | Configuration, RPC transport, DEX-specific reads, gas-price reads, market overview assembly | `src-tauri/src/config/`, `src-tauri/src/ethereum/`, `src-tauri/src/dex/`, `src-tauri/src/commands/market.rs` |
| Cross-boundary market contract | Normalised payload shape shared between Rust and TypeScript | `src-tauri/src/market/types.rs`, `src/features/arbitrage/types.ts` |
| **Persistence (M2.0)** | SQLite read/write topology, schema migrations, idempotent writes for swaps / pool events / runs / strategies / benchmarks / headline | `src-tauri/src/storage/` |
| **V3 math primitives (M2.2)** | Q64.96 fixed-point, tick ↔ sqrtPriceX96, liquidity ↔ amounts, per-swap fees, IL closed forms | `src-tauri/src/math/` |
| **Archive ingestion (M2.1)** | `eth_getLogs` batched fetcher, ABI decoder for Swap/Mint/Burn/Collect, Alchemy live + mock test source, idempotent persistence + checkpoint | `src-tauri/src/ingest/` |
| **Position simulation engine (M2.3)** | Per-swap fee distribution, in-range tracking, LVR (Milionis), management gas at block-historical prices, equity curve emission | `src-tauri/src/backtest/` |
| **Validation harness (M2.4)** | Replay LP-position fixtures, compute fees/gas/value diffs vs ground truth, pass/fail rolled into a `ValidationReport` | `src-tauri/src/validation/` |
| **Strategy comparison grid (M2.5)** | Grid search over `range × rule × deposit × period`, persists per-cell metrics (Sharpe / Sortino / Deflated Sharpe / max DD / etc) | `src-tauri/src/strategies/` |
| **Multi-asset benchmark module (M2.7)** | DefiLlama no-key (Aave/Compound/Lido), FRED + Stooq no-key chain (T-bills, S&P, gold), beaconcha.in KEY_REQUIRED, V2 LP constant-product, HODL, alpha decomposition (period + rolling 30/60/90) | `src-tauri/src/benchmarks/` |
| **Capital allocation headline (M2.8)** | Adaptive-tercile vol regime classifier, per-month best/naive/median LP vs Aave/Lido/HODL, verdict prose synthesis | `src-tauri/src/headline/` |
| **Tab 2 — LP Backtester frontend** | Strategy controls, equity curve chart, headline verdict block, sortable strategy grid, regime panel | `src/features/lp-backtest/` |
| **Vector A IPC layer** | 10 Tauri commands wrapping ingestion / backtest / grid / headline / benchmark fetch + cache; KEY_REQUIRED surfaced via CommandError.keyRequired | `src-tauri/src/commands/lp.rs` |

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

End-to-end trace of one refresh tick, from the 1 Hz frontend timer to the rendered dashboard:

```text
ArbitragePage mounts
  -> useEffect schedules setInterval(loadSnapshot, 1000 ms)
  -> loadSnapshot guards against overlap via requestInFlight ref
  -> fetchMarketOverview() in api.ts calls invoke("fetch_market_overview")
     ====== IPC boundary (Tauri <-> React) ======
  -> commands::market::fetch_market_overview (src-tauri/src/commands/market.rs)
       -> AppConfig::from_environment           (config.rs)
            -> loads .env once via ENVIRONMENT_BOOTSTRAP (Once)
            -> resolves MAINNET_RPC_URL or constructs Alchemy URL from ALCHEMY_API_KEY
       -> EthereumRpcClient::new                (ethereum/client.rs)
       -> tokio::join! of five concurrent futures:
            |-- uniswap_v3::fetch_weth_usdc_snapshot       (V3 5bps pool slot0)
            |-- uniswap_v3::fetch_weth_usdc_30bps_snapshot (V3 30bps pool slot0)
            |-- uniswap_v2::fetch_uniswap_v2_snapshot      (factory getPair -> reserves)
            |-- uniswap_v2::fetch_sushiswap_snapshot       (factory getPair -> reserves)
            `-- rpc_client.gas_price_gwei()
       -> any one .map_err(...)? fails the whole command (no partial success)
       -> assembles MarketOverview { chain, pair_label, fetched_at_unix_ms, gas_price_gwei, venues[4] }
       -> fetched_at_unix_ms is copied from the V3 5bps snapshot timestamp, not command-local
     ====== IPC boundary (Rust -> JSON, camelCase) ======
  -> api.ts receives MarketOverview typed payload
  -> setOverview(nextOverview)
  -> setSnapshot(nextOverview.venues[0])    // array-order contract: V3 5bps is hero
  -> setHistory((h) => [...h, nextOverview].slice(-100))   // rolling window
  -> deriveInsightsView(history) runs only after state commits
     -> computes median, spread, gas-adjusted estimate per sample
     -> emits primary card, up to 4 secondary cards, up to 4 events
  -> render pass fans out:
       |-- PriceCard       ( snapshot, gasPriceGwei )
       |-- MarketChart     ( history, chartMode, showEvents )
       |-- InsightsPanel   ( InsightsViewModel )
       `-- venue lanes     ( VENUES metadata joined to overview.venues by dexName )
```

- Refresh cadence is fixed at one second in the frontend rather than being driven by a backend scheduler.
- The `requestInFlight` ref in `ArbitragePage` prevents overlapping ticks when a read takes longer than 1 s; when it triggers, a tick is silently dropped.
- History is session-only and disappears on restart because no persistence layer exists.
- Failure handling is still coarse-grained: any venue read failure rejects the whole command and surfaces an error banner in the GUI while leaving history intact.
- The hero card implicitly contracts with the backend that `venues[0]` is the canonical primary venue; reordering the `vec![uniswap_v3_5bps, ...]` in `commands/market.rs` would silently change which venue the user sees as "the price".

## Inter-System Relationships

The repository has roughly fourteen system-level surfaces (Tab 1 stack + the Vector A subsystems shipped 2026-05-03). The most consequential cross-boundary relationships are listed here; per-system outward connections live in the owning `systems/*.md` file.

### Tab 1 (Arbitrage) — runtime-foundation, market-data, analytics, gui

| Upstream | Downstream | Mechanism | Boundary shape | What breaks if the link fails |
| --- | --- | --- | --- | --- |
| runtime-foundation | arbitrage-market-data | Rust intra-crate imports of `EthereumRpcClient`, `AppConfig`, `MarketOverview`, `PriceSnapshot` | Synchronous function calls inside one Tokio runtime | The `fetch_market_overview` command cannot construct its transport; the IPC call returns an error string, UI shows the error banner |
| arbitrage-market-data | arbitrage-analytics | `MarketOverview` payload over Tauri IPC (`invoke("fetch_market_overview")`), serialised as JSON with camelCase field names | Async request-response, fail-fast on any venue error | `deriveInsightsView` is never called; the UI keeps the last successful history, the error banner surfaces, the 1 Hz loop keeps retrying |
| arbitrage-analytics | arbitrage-gui | `InsightsViewModel` pure-data handoff in the same React tree (no IPC) | In-process prop passing | Only the insights panel would fail; the rest of the dashboard continues — but in practice `deriveInsightsView` is called unconditionally when history is non-empty, so a throw propagates a full React error boundary failure |
| runtime-foundation | arbitrage-gui | `MarketOverview`/`PriceSnapshot` TypeScript types in `src/features/arbitrage/types.ts` mirrored from `src-tauri/src/market/types.rs`; shell-level metadata via `index.html` + `tauri.conf.json` | Type-level contract bridged by Serde's `rename_all = "camelCase"` on the Rust side | A Rust field rename without the TS mirror being updated compiles cleanly on both sides but produces `undefined` reads at runtime — there is no automated contract check |
| arbitrage-market-data | arbitrage-gui | Implicit ordering contract: `venues[0]` is treated as the hero price; `dexName` string values are used as identity keys by `VENUES` (ArbitragePage) and `SERIES_META` (MarketChart) | Position-based + string-based implicit contract | Reordering `vec![...]` in `commands/market.rs` silently changes the hero; renaming a DEX label silently breaks both the venue card's price lookup and the chart's per-line colour/legend mapping |

### Tab 2 (LP Backtest) — Vector A subsystems

| Upstream | Downstream | Mechanism | Boundary shape | What breaks if the link fails |
| --- | --- | --- | --- | --- |
| ingest | storage | `Storage::insert_swap_events_batch(Vec<SwapEventRow>)` + `insert_pool_events_batch` | Async write through writer-thread; idempotent via `INSERT OR IGNORE` keyed on `(pool_address, block_number, log_index)` | `Ingester::backfill` returns `IngestError`; the LP page error-banners. Idempotency means partial failures are recoverable on next run |
| storage | backtest | `Storage::query_swaps_for_pool_range(pool, from, to)` returns `Vec<SwapEventRow>` | Sync read via reader pool + `spawn_blocking`; rows ordered by `(block_number, log_index)` | `Engine::simulate` returns `BacktestError::EmptyData` when zero rows; pipeline halts. Backtest never blocks on a writer transaction |
| math | backtest | Pure-function imports of `tick_to_sqrt_price_x96`, `liquidity_for_amounts`, `amounts_for_liquidity`, `fee_share_token0/1` | Synchronous function calls within one Tokio task | A `V3MathError` propagates as `BacktestError::MathError`; engine halts that simulation run |
| backtest | strategies | `GridRunner::run_grid` invokes `Engine::simulate(config, rule)` per cell sequentially | Sequential per-cell loop; results aggregated into `StrategyResultRow` | Per-cell `BacktestError` halts that cell; grid continues to next cell. (The Pass 2 audit recommends a parallel variant — out of scope today) |
| backtest | headline | `HeadlineRunner::run` invokes `Engine::simulate` per month per LP variant (best/naive/median) | Sequential per-(month, variant); 3 × N month sims | Per-month `BacktestError` halts that month; verdict still synthesises from remaining months |
| strategies + benchmarks | headline | Headline reads strategy grid + benchmark series from storage and composes the verdict | Composition via two storage reads | Stale strategy grid → stale verdict; stale benchmarks → missing benchmark column in verdict |
| commands/lp.rs | every Vector A backend | Tauri IPC dispatch — fan-out 16 (`Engine`, `Ingester`, `GridRunner`, `HeadlineRunner`, `DefiLlamaProvider`, `TradFiProvider`, `Storage`, etc.) | Async Tauri command surface; `CommandError { message, key_required }` shape | Backend error → `CommandError` returned to frontend; frontend renders error banner. The `key_required` field lets the frontend prompt for API-key configuration distinctly from generic errors |
| commands/lp.rs | lp-backtest-gui | Per-IPC typed wrappers in `src/features/lp-backtest/api.ts` mirror Rust DTOs | JSON over Tauri IPC; Rust `serde(rename_all = "camelCase")` + hand-kept TS types in `types.ts` | Field rename in Rust without TS update → `undefined` at runtime; same risk class as Tab 1's wire-convention. Documented in `notes/wire-convention.md` |
| telemetry | every page + every IPC | `telemetry.record(eventName, payload)` (frontend) + `telemetry_persist` IPC (backend writes to `~/Library/Logs/com.ataca.aurix/last-session.json`) | Cross-cutting event recorder; not in the dependency graph but used by every active flow | Telemetry buffer growth is bounded by interval flush; missing flush = recent events lost from `last-session.json` but the application continues to function |

## Critical Paths and Blast Radius

### Tab 2 — User clicks Re-run on the LP backtester

This trace covers the dominant Vector A flow. It crosses 8 system boundaries.

```
LpBacktestPage useEffect fires (settings JSON-key changed OR rerunNonce bumped)
  ├─ telemetry.record("lp.pipeline.start", {...})
  ├─ lpPoolMetadata(...) ─IPC─→ commands::lp::lp_pool_metadata
  │     └─ UniswapV3SubgraphSource::pool_metadata(addr) ─HTTP─→ hosted subgraph
  │     ↑ on failure → CommandError; pipeline aborts with banner
  │
  ├─ lpGetChainHead(chain) ─IPC─→ commands::lp::lp_get_chain_head
  │     ├─ AlchemyArchiveSource::from_environment().latest_finalized_block()
  │     │     ↑ on failure (key missing or 400) → fall through
  │     └─ public RPC → eth_getBlockByNumber("finalized")
  │     ↑ on terminal failure → CommandError ("Could not reach chain head")
  │
  ├─ runLpIngestion(pool, head-N, head, chain, proto) ─IPC─→ commands::lp::run_lp_ingestion
  │     ├─ Tier 1: UniswapV3SubgraphSource::for_protocol(...).fetch_logs ─→ Ingester::backfill
  │     │     └─ decoder::decode_swap → SwapEventRow → Storage::insert_swap_events_batch
  │     │     ↑ on failure → fall through to Tier 2
  │     ├─ Tier 2: AlchemyArchiveSource::from_environment() (mainnet only) → Ingester::backfill
  │     │     ↑ on failure → fall through to Tier 3
  │     └─ Tier 3: AlchemyArchiveSource::with_rpc_url(public_rpc) → Ingester::backfill
  │     ↑ on terminal failure → CommandError; no synthetic fallback (notes/no-synthetic-in-user-facing.md)
  │
  ├─ lpQueryFirstSwapPrice(pool, from, to, t0d, t1d) ─IPC─→ Storage::query_swaps_for_pool_range first row
  │     └─ math::sqrt_price_x96_to_tick + math::price helpers → FirstSwapInfo { tick, price }
  │
  ├─ lpTokenUsdPrices(chain, [token0, token1]) ─IPC─→ commands::lp::lp_token_usd_prices
  │     └─ DefiLlama coins API → TokenPricesDto
  │
  ├─ runLpBacktest(config, rule) ─IPC─→ commands::lp::run_lp_backtest
  │     └─ Engine::simulate(config, rule)
  │           ├─ Storage::query_swaps_for_pool_range → ~1k rows (TEXT decimal big-ints)
  │           ├─ math::liquidity::liquidity_for_amounts (initial L from deposit + entry price)
  │           ├─ per-swap loop (the dominant cost):
  │           │     ├─ parse_sqrt / parse_signed / parse_liquidity (3-4 BigUint::parse_bytes per swap)
  │           │     ├─ math::fees::fee_share_token0/1 (in-range check + share)
  │           │     ├─ LVR discrete approximation (f64-cast Δsqrt²·L/sqrt)
  │           │     ├─ rebalance trigger via RebalanceContext + RebalanceRule
  │           │     └─ value_usd via position_usd_value(_explicit) — 3 calls per swap
  │           └─ Storage::persist_position_run (idempotent on config_hash)
  │
  ├─ runLpGrid(grid_config) ─IPC─→ GridRunner::run_grid
  │     └─ Engine::simulate per cell × 81 cells (3×3×3×3 default grid)
  │
  ├─ runLpHeadline(headline_config) ─IPC─→ HeadlineRunner::run
  │     ├─ per-month sub-backtests × 3 LP variants × N months
  │     ├─ benchmark series reads from Storage (per-asset)
  │     ├─ adaptive-tercile vol-regime classifier
  │     └─ verdict prose synthesis
  │
  ├─ lpFetchBenchmarkSeries(series_key) × N ─IPC─→ DefiLlamaProvider / TradFiProvider
  │     └─ Storage::insert_benchmark_points_batch (replace-on-duplicate)
  │
  └─ React state updates → block components render
```

**Critical-path observations:**

- **8 systems touched:** lp-backtest-gui, telemetry, IPC commands, ingest, storage, backtest, math, strategies, headline, benchmarks (10 if you count the cross-cutting telemetry recorder).
- **Idempotency holds at every storage write.** Re-running the pipeline is a cache-hit at every step; React StrictMode's double-mount is structurally safe (per `notes/idempotent-runs.md`).
- **Failure terminals at every step** are explicit `CommandError` returns; no path falls through to synthetic data (per `notes/no-synthetic-in-user-facing.md`).
- **No paid-API path** in the chain: Subgraph → user-Alchemy → public RPC → empty-state (per `notes/free-data-fallback-chain.md`).

### Tab 1 — 1 Hz `fetch_market_overview` tick

This trace was previously the dominant flow before the Vector A sprint. It is unchanged by the sprint.

```
ArbitragePage mounts → setInterval(loadSnapshot, 1000)
  └─ fetchMarketOverview() → invoke("fetch_market_overview")
        ↓ IPC boundary
        commands::market::fetch_market_overview
        ├─ AppConfig::from_environment (config.rs)
        ├─ EthereumRpcClient::new
        ├─ tokio::join! of 5 futures (V3 5bps, V3 30bps, V2, Sushi, gas)
        │     └─ any one .map_err? → whole command fails
        └─ MarketOverview { venues: [...4...], gas_price_gwei, fetched_at_unix_ms }
        ↓ IPC boundary (Rust → JSON camelCase)
  └─ React state → 100-sample rolling history → derive insights → render PriceCard / Chart / Insights
```

**Critical-path observations** (unchanged from prior architecture pass):
- **Single external dependency:** the Ethereum mainnet JSON-RPC endpoint (one URL backs every venue read + the gas read).
- **Fail-fast error model:** any one venue's failure rejects the whole command (documented in `Aurix/Gaps.md` Gap 2 as a known limitation).
- **`venues[0]` is the implicit hero contract;** reordering the vec silently switches which DEX the user sees as the price.

## Critical Paths and Blast Radius

One critical operation dominates the system: the `fetch_market_overview` IPC call. Because it is polled at 1 Hz and drives every rendered value, its blast radius is the whole product surface.

| Change target | Blast radius |
| --- | --- |
| Renaming a field on `MarketOverview`/`PriceSnapshot` in Rust | Frontend reads become `undefined` on that field; no compile-time signal; the dashboard renders with silent holes until runtime. The TS mirror in `types.ts` must be updated in lockstep. |
| Changing the order of the `venues` vec in `commands/market.rs` | The hero price (`PriceCard`) silently switches to a different DEX; downstream analytics are unaffected because they use `dexName` lookups, but the chart and the venue lanes would be unaffected too. |
| Renaming a `dex_name` value in a V2/V3 adapter | `VENUES` (price lookup, accent colour) and `SERIES_META` (chart line colour, legend) fall back to default or crash on undefined lookup. Both live in frontend code. |
| Switching RPC provider | Single touch point in `config.rs`; no other module knows the URL. The transport is provider-agnostic (generic JSON-RPC). |
| Increasing venue count to five | Backend adds a future to `tokio::join!`; frontend must be extended in two places — `VENUES` in `ArbitragePage.tsx` and `SERIES_META` in `MarketChart.tsx` — or the new venue renders without colour and without a lane. |
| Changing the fee-tier mapping or adding a new pool | Localised to one DEX adapter in `src-tauri/src/dex/`; no frontend change required as long as `dex_name` is stable. |

**Single external dependency**: the Ethereum mainnet JSON-RPC endpoint. All four venue reads and the gas-price read go through the one `EthereumRpcClient` pointing at one URL. Provider rate-limiting, outage, or rewrite of the free Alchemy tier silently fails every 1 Hz tick simultaneously — the product has no alternate path.

## State Ownership

| State | Owner | Lifetime | Notes |
| --- | --- | --- | --- |
| RPC URL | `AppConfig` (`src-tauri/src/config.rs`) | Resolved once on first call, reloaded on every command invocation | `ENVIRONMENT_BOOTSTRAP: Once` ensures `.env` files are parsed exactly once per process lifetime |
| HTTP client | `EthereumRpcClient` | Constructed once per IPC call (no pooling across ticks) | `reqwest::Client` is itself internally pooled, so this is acceptable but not optimal |
| Market history | `ArbitragePage` React state | Session-only; wiped on reload | Fixed 100-sample rolling window; no persistence, no replay |
| Chart mode / events toggle | `ArbitragePage` React state | Session-only | Not persisted across reloads |
| Venue metadata (`VENUES`) | Frontend module constant | Source-level | Joined to the backend payload by `dexName` string equality |
| Chart series metadata (`SERIES_META`) | `MarketChart` module constant | Source-level | Same `dexName` keyspace as `VENUES` — separate copy |

## Structural Notes / Current Reality

- The repository still contains starter scaffolding residue in `index.html` (title `Tauri + React + Typescript`, favicon `/vite.svg`), `public/`, `src/assets/`, and metadata fields in `Cargo.toml` (`description = "A Tauri App"`, `authors = ["you"]`) and `tauri.conf.json` (lower-case `productName: "aurix"` and window title `aurix`).
- The backend returns four venue snapshots, but stale copy saying "three live Ethereum venue reads" still appears in three places — `src/features/arbitrage/components/PriceCard.tsx:48`, the rustdoc on `commands::market::fetch_market_overview` at `src-tauri/src/commands/market.rs:11`, and the fallback string `"3"` for the Active-venues detail row at `src/features/arbitrage/ArbitragePage.tsx:222`.
- Tauri capability and icon folders are present because the desktop shell is already bundle-ready even though the product shell polish is incomplete.
- There is no test directory or automated verification layer in the repository; current correctness depends on manual inspection and build checks.
- No source file carries `WHY`/`NOTE`/`HACK`/`IMPORTANT`/`TODO`/`SAFETY` annotations — design rationale lives entirely in `context/` documents and in rustdoc on public functions, not inline at decision points. Future sessions should continue to capture rationale in `context/notes/` rather than scatter annotations through code.
- Tabs 2 to 5 remain README intent only and should not be described elsewhere in `context/` as implemented systems.

## Coverage

Files inspected end-to-end during this upkeep pass (content read, not just listed):

- `src-tauri/src/`: `main.rs`, `lib.rs`, `config.rs`, `commands/mod.rs`, `commands/market.rs`, `ethereum/mod.rs`, `ethereum/client.rs`, `dex/mod.rs`, `dex/uniswap_v2.rs`, `dex/uniswap_v3.rs`, `market/mod.rs`, `market/types.rs`
- `src-tauri/`: `Cargo.toml`, `tauri.conf.json`, `capabilities/default.json`
- `src/`: `main.tsx`, `App.tsx`, `features/arbitrage/api.ts`, `features/arbitrage/types.ts`, `features/arbitrage/ArbitragePage.tsx`, `features/arbitrage/insights.ts`, `features/arbitrage/components/PriceCard.tsx`, `features/arbitrage/components/MarketChart.tsx`, `features/arbitrage/components/InsightsPanel.tsx`
- Repo root: `index.html`, `package.json`

Noted but not read during this pass (no change pressure surfaced):

- `src/styles/theme.css`, `src/styles/dashboard.css` — pure presentation, summarised in `systems/arbitrage-gui.md` without field-level verification
- `vite.config.ts`, `tsconfig.json`, `tsconfig.node.json`, `src-tauri/build.rs`
- `src-tauri/Cargo.lock`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`
- `public/`, `src-tauri/icons/`, `src-tauri/gen/` (generated Tauri schemas), `src-tauri/target/`, `dist/`, `node_modules/`, `src/assets/`, `.vscode/`

Nothing in the repository was described from file-structure inference alone — every claim in `context/` maps to a file that was either read this pass or read during the prior upkeep and re-verified against the import graph this pass.
