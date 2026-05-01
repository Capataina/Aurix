# Two-Runtime Architecture: Tauri + Rust + React

## Why This Architecture Matters

Aurix runs as a **desktop application with two heterogeneous runtimes**: a Rust backend doing all the network I/O and on-chain decoding, and a React 19 + TypeScript frontend rendering the UI. They communicate via Tauri's IPC layer.

This is a deliberate choice with significant consequences. Understanding it is the foundation for understanding every other Aurix design decision — read-only by design, no ethers-rs, the IPC contract, the async fetch pattern, all of it traces back to "two runtimes, mediated by Tauri."

## High-Level Shape

```
┌──────────────────────────────────────────────────────────┐
│                     OS Process Boundary                   │
│                                                          │
│  ┌────────────────────┐         ┌─────────────────────┐  │
│  │    Rust Backend    │         │  React Frontend     │  │
│  │                    │         │                     │  │
│  │  · Async I/O       │         │  · UI rendering     │  │
│  │  · RPC client      │         │  · Local state      │  │
│  │  · DEX adapters    │         │  · Insights engine  │  │
│  │  · Crypto math     │         │  · Charts (SVG)     │  │
│  │  · BigUint         │         │  · Event handling   │  │
│  └─────────┬──────────┘         └──────────┬──────────┘  │
│            │                               │             │
│            │       Tauri IPC layer         │             │
│            └───────────────┬───────────────┘             │
│                            │                             │
│                       JSON over IPC                      │
│                       (Serde camelCase)                  │
│                                                          │
└──────────────────────────────────────────────────────────┘
                             │
                             │ JSON-RPC over HTTPS
                             v
                  ┌─────────────────────────┐
                  │   Ethereum Mainnet      │
                  │   (RPC provider)        │
                  └─────────────────────────┘
```

Both runtimes live in the same OS process. They communicate via Tauri's IPC channel — the frontend invokes typed Rust functions (`#[tauri::command]`) and receives serialised responses.

## The Two Runtimes

### The Rust Backend

Lives in `src-tauri/src/`. Responsibilities:

- **Network I/O** — talks to the Ethereum RPC endpoint via `reqwest`
- **Concurrent fetching** — uses `tokio` for async + `tokio::join!` to fan out venue reads
- **DEX-specific decoding** — `dex/uniswap_v3.rs` and `dex/uniswap_v2.rs` decode pool state from raw JSON-RPC responses
- **Big-integer arithmetic** — `num-bigint` for Ethereum's 256-bit values like `sqrtPriceX96`
- **Configuration** — env-based RPC URL resolution (Alchemy fallback)
- **Type definitions** — the `MarketOverview` and `PriceSnapshot` structs that cross to the frontend

The backend exposes exactly ONE Tauri command: `fetch_market_overview`. That's the entire surface area — one function the frontend can call.

### The React Frontend

Lives in `src/`. Responsibilities:

- **UI rendering** — React components for venue cards, charts, insight panels
- **Local state management** — useState/useRef for the rolling history, current snapshot, error states
- **Polling orchestration** — `setInterval` at 1 Hz drives `loadSnapshot()`
- **Analytics** — `insights.ts` derives the rule-based interpretation from the rolling history
- **Chart rendering** — `MarketChart.tsx` produces hand-rolled SVG (no charting library)
- **Visual system** — plain CSS (no Tailwind, no shadcn)

The frontend is intentionally a pure presentation + interpretation layer. All network I/O happens in the backend.

## Why Two Runtimes (Not One)

You could in principle build Aurix as:

1. **Pure TypeScript** (Electron + Node) — frontend and backend both JS
2. **Pure Rust** (egui + a Rust HTTP/UI loop) — no JS at all
3. **The chosen approach: heterogeneous** (Rust backend + JS frontend, mediated by Tauri)

The chosen approach wins on:

| Concern | Pure TS | Pure Rust | Tauri+Rust+React |
|---|---|---|---|
| Backend perf for big-integer math | Slow (BigInt) | Fast | Fast |
| Frontend ecosystem (charts, animations, dev tooling) | Excellent | Limited | Excellent |
| Binary size | Large (Electron bundles Chromium) | Small | Small (Tauri uses OS webview) |
| Cross-runtime contract overhead | None (same runtime) | None | Some (Serde + Tauri IPC) |
| Memory footprint | High | Low | Low |
| Hire-ability of someone to extend | Easy (JS devs everywhere) | Harder | Mid |

The decision rationale lives in `project/decisions/tauri-over-electron.md` and `project/decisions/rust-backend-over-pure-typescript.md`. Read both for the full reasoning.

## The IPC Contract

The Rust backend defines `MarketOverview` and `PriceSnapshot` in `src-tauri/src/market/types.rs`:

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceSnapshot {
    pub chain: String,
    pub dex_name: String,
    pub pair_label: String,
    pub price_usd: f64,
    pub pool_address: String,
    pub fee_tier_bps: u16,
    pub price_source_label: String,
    pub fetched_at_unix_ms: u64,
}
```

The `#[serde(rename_all = "camelCase")]` is critical — it converts Rust's snake_case fields to JS-friendly camelCase on the wire. The frontend's TypeScript mirror (in `src/features/arbitrage/types.ts`) declares matching camelCase fields.

This mirror is **manually maintained**. There's no automated codegen — a field rename in Rust without a matching TypeScript change compiles cleanly on both sides and fails silently at runtime. This is Gap 11 in the gap inventory.

Full coverage of the IPC contract lives in `project/architecture/cross-runtime-contract.md`.

## Data Flow Per Tick

Every second, this happens:

```
1. Frontend timer fires (setInterval, 1000ms)
2. ArbitragePage.tsx loadSnapshot() called
3. fetchMarketOverview() invokes Tauri command
4. Tauri serialises the call, hands to Rust
5. fetch_market_overview() runs:
   a. Build EthereumRpcClient
   b. tokio::join! over 5 futures (4 venue fetches + gas)
   c. Each venue future does eth_call against pool contract, decodes response
   d. Returns MarketOverview struct
6. Tauri serialises MarketOverview to camelCase JSON
7. Frontend receives JSON, hydrates as MarketOverview type
8. setHistory appends to 100-sample window
9. insights.ts derives the InsightsViewModel
10. React re-renders with new state
```

Detailed walkthrough in `project/architecture/the-1hz-loadsnapshot-tick.md`.

## Tauri's Role Specifically

Tauri is the framework that:

- Compiles the Rust backend into a native binary
- Embeds the React frontend as a webview (the OS's native webview, not bundled Chromium)
- Provides the IPC bridge with type checking on the Rust side
- Handles desktop concerns (window sizing, OS integration, file system access permissions)

What Tauri ISN'T:
- A web framework (it's a desktop framework)
- An alternative to Electron in the simple sense (it's smaller and more secure but the tradeoff is using OS webview which can vary by platform)
- A way to share code between runtimes (you still have two languages, two type systems, manual mirror maintenance)

## Major Boundaries

| Boundary | What changes across it |
|---|---|
| **Rust ↔ Tauri IPC** | Async function call → JSON-serialised return |
| **Tauri IPC ↔ JavaScript** | JSON deserialisation, camelCase mapping |
| **Rust ↔ Ethereum RPC** | HTTP request/response, hex-encoded calldata |
| **Frontend state ↔ Components** | React hooks, props, context |
| **DEX adapter ↔ Generic backend** | Protocol-specific decoding to common `PriceSnapshot` shape |

Each boundary is a place where bugs hide. The Rust ↔ TypeScript boundary is the riskiest because it has no compile-time check.

## Pressure Points

1. **The cross-runtime contract** has no automated check (Gap 11). Solutions: add `ts-rs` codegen, add a runtime validator, or add a contract test that exercises the wire format.

2. **The single Tauri command pattern** means any new feature needs a new command. This is fine for one feature but doesn't scale. When Vector A or Vector B ships, the backend will need command organisation (probably a `commands/` module per feature area).

3. **Polling is a chunky abstraction** — every tick is a full RPC round-trip. WebSocket subscriptions would be more efficient. Vector B brings WebSockets in for mempool watching; that infrastructure could be reused for live pool state too.

4. **Memory between ticks** lives in React state only. There's no backend cache, no persistence — every tick re-queries from scratch. Persistence (M2.0 of Vector A) is the architectural fix.

## Related Files

- `project/architecture/the-1hz-loadsnapshot-tick.md` — detailed tick walkthrough
- `project/architecture/cross-runtime-contract.md` — the IPC contract details
- `project/decisions/tauri-over-electron.md` — why Tauri specifically
- `project/decisions/rust-backend-over-pure-typescript.md` — why heterogeneous
- `context/architecture.md` — implementation-facing version
