# The 1 Hz LoadSnapshot Tick

## Why This Matters

Every Aurix observation flows through one path: the 1 Hz polling loop driven by `setInterval` in `ArbitragePage.tsx`. Understanding this tick end-to-end is understanding what Aurix actually does as a running program.

## The Tick

Every 1,000 ms (`REFRESH_INTERVAL_MS = 1_000`), the React frontend triggers a `loadSnapshot()` call. The full round-trip is:

```
                React Frontend                        Rust Backend                   Ethereum RPC
                ──────────────                        ────────────                   ────────────
  T=0ms  │      setInterval fires                                                                
         │           │                                                                            
         │           v                                                                            
         │      requestInFlight check ─── true → bail                                             
         │           │ false                                                                      
         │           v                                                                            
         │      requestInFlight = true                                                            
         │      setLoading(true)                                                                  
         │      setErrorMessage(null)                                                             
         │           │                                                                            
         │           v                                                                            
         │      fetchMarketOverview()                                                             
         │      (Tauri invoke)                                                                    
         │           │                                                                            
         │           v                                                                            
         │      [Tauri IPC bridge]   ──────►  fetch_market_overview()                            
         │                                          │                                             
         │                                          v                                             
         │                                    AppConfig::from_environment()                       
         │                                    EthereumRpcClient::new(...)                         
         │                                          │                                             
         │                                          v                                             
         │                                    tokio::join!(                                       
         │                                      uniswap_v3_5bps_fetch,    ────►    eth_call slot0()
         │                                      uniswap_v3_30bps_fetch,   ────►    eth_call slot0()
         │                                      uniswap_v2_fetch,         ────►    eth_call x3
         │                                      sushiswap_fetch,          ────►    eth_call x3
         │                                      gas_price                 ────►    eth_gasPrice
         │                                    )                                                   
         │                                          │                                             
         │                                          v                                             
         │                                    MarketOverview { ... }                              
         │                                          │                                             
         │                                          v                                             
         │      [Tauri IPC bridge]   ◄──────  Ok(MarketOverview)                                 
         │           │                                                                            
         │           v                                                                            
         │      setOverview(nextOverview)                                                         
         │      setSnapshot(venues[0])                                                            
         │      setHistory([...prev, nextOverview].slice(-100))                                  
         │           │                                                                            
         │           v                                                                            
         │      insights.ts: deriveInsightsView(history)                                          
         │           │                                                                            
         │           v                                                                            
         │      React re-renders                                                                  
         │      requestInFlight = false                                                           
         │      setLoading(false)                                                                 
         │                                                                                        
         │           ─────────────────────────────────────────────────                           
  T=1000ms        next tick fires                                                                
```

End-to-end latency: typically 200-800ms depending on RPC provider response time. The 200ms variability is dominated by network round-trip; the local computation (BigUint decode, JSON serialisation) is sub-millisecond.

## Stage-by-Stage Detail

### Stage 1: Tick fires (frontend)

In `ArbitragePage.tsx`:

```typescript
useEffect(() => {
    void loadSnapshot();  // Initial fire
    const intervalId = window.setInterval(() => {
        void loadSnapshot();
    }, REFRESH_INTERVAL_MS);  // = 1000
    return () => window.clearInterval(intervalId);
}, []);
```

The interval is set on component mount and cleaned up on unmount. The initial `loadSnapshot()` fires immediately so the user doesn't wait a full second for first data.

### Stage 2: Concurrency guard

```typescript
async function loadSnapshot() {
    if (requestInFlight.current) {
        return;  // Bail if previous tick hasn't returned
    }
    requestInFlight.current = true;
    // ...
}
```

The `requestInFlight` ref prevents overlapping requests. If a previous tick is still executing (because the RPC was slow), the new tick is dropped. This is intentional — we'd rather skip a tick than queue requests, since queued requests would compound latency.

### Stage 3: Tauri command invocation

```typescript
const nextOverview = await fetchMarketOverview();
```

In `src/features/arbitrage/api.ts`, `fetchMarketOverview()` invokes the Tauri command:

```typescript
import { invoke } from "@tauri-apps/api/core";

export async function fetchMarketOverview(): Promise<MarketOverview> {
    return invoke<MarketOverview>("fetch_market_overview");
}
```

`invoke` serialises the command name and arguments, sends them through Tauri's IPC channel, and awaits the JSON-serialised response. The TypeScript generic `<MarketOverview>` provides typing — but no runtime validation; if the Rust side returns a different shape, TypeScript happily believes the lie until something accesses a missing field.

### Stage 4: Backend dispatch

In `src-tauri/src/lib.rs`, `fetch_market_overview` is registered as a Tauri command. Tauri receives the call, deserialises arguments (none in this case), and invokes the async function.

In `src-tauri/src/commands/market.rs`:

```rust
#[tauri::command]
pub async fn fetch_market_overview() -> Result<MarketOverview, String> {
    let configuration = AppConfig::from_environment().map_err(|error| error.to_string())?;
    let rpc_client = EthereumRpcClient::new(configuration.ethereum_mainnet_rpc_url());
    // ...
}
```

`AppConfig::from_environment()` is called every tick — the dotenv layer is initialised once via a `Once`-guarded bootstrap, so this is just a hash-map lookup, not file I/O.

### Stage 5: Concurrent fan-out

```rust
let uniswap_v3_5bps = uniswap_v3::fetch_weth_usdc_snapshot(&rpc_client);
let uniswap_v3_30bps = uniswap_v3::fetch_weth_usdc_30bps_snapshot(&rpc_client);
let uniswap_v2 = uniswap_v2::fetch_uniswap_v2_snapshot(&rpc_client);
let sushiswap = uniswap_v2::fetch_sushiswap_snapshot(&rpc_client);
let gas_price_gwei = rpc_client.gas_price_gwei();
let (uniswap_v3_5bps, uniswap_v3_30bps, uniswap_v2, sushiswap, gas_price_gwei) =
    tokio::join!(
        uniswap_v3_5bps,
        uniswap_v3_30bps,
        uniswap_v2,
        sushiswap,
        gas_price_gwei
    );
```

Five futures, run concurrently via `tokio::join!`. The futures don't block each other — they all start in parallel and the `join!` macro waits for all five to complete.

### Stage 6: Per-venue decoding

Each of the 5 futures runs independently:

**V3 5bps and V3 30bps** (`dex/uniswap_v3.rs`):
- Single `eth_call` to `slot0()` on the pool address
- Decode the first 32-byte word as `BigUint` → `sqrtPriceX96`
- Compute price: `(2^192 × 10^12) / sqrtPriceX96²`
- Wrap as `PriceSnapshot`

**V2 and Sushi** (`dex/uniswap_v2.rs`):
- Three `eth_call`s sequentially: `getPair(USDC, WETH)` → `token0()` → `getReserves()`
- Compute price: `(reserve_usdc / reserve_weth) × 10^12` (or inverse based on which token is token0)
- Wrap as `PriceSnapshot`

V2 takes longer than V3 (3 RPC calls vs 1) — this is one source of timing skew between venues, called out in Gap 10 (per-adapter timestamps).

**Gas price** (`ethereum/client.rs`):
- Single `eth_gasPrice` call
- Convert wei to gwei

### Stage 7: Failure semantics

```rust
let uniswap_v3_5bps = uniswap_v3_5bps.map_err(|error| error.to_string())?;
// ... and so on for each
```

The `?` operator on each result means **any single failure rejects the whole tick**. This is fail-fast (Gap 2). Pros: simple. Cons: a sushi RPC blip kills the tick even though V3 data is healthy.

A future fix (per Gap 2) would aggregate — collect successes and failures into a per-venue result enum, surface partials to the frontend.

### Stage 8: MarketOverview assembly

```rust
Ok(MarketOverview {
    chain: "Ethereum Mainnet".to_string(),
    pair_label: "WETH / USDC".to_string(),
    fetched_at_unix_ms: uniswap_v3_5bps.fetched_at_unix_ms,
    gas_price_gwei,
    venues: vec![uniswap_v3_5bps, uniswap_v3_30bps, uniswap_v2, sushiswap],
})
```

The `fetched_at_unix_ms` is taken from the V3 5bps snapshot — not a command-level orchestration timestamp. This is Gap 10. The venues vec ordering is implicitly contractual (frontend's VENUES array assumes this order).

### Stage 9: Serialisation back to frontend

Tauri serialises `MarketOverview` via Serde. The `#[serde(rename_all = "camelCase")]` attribute converts every field to camelCase on the wire:

```json
{
    "chain": "Ethereum Mainnet",
    "pairLabel": "WETH / USDC",
    "fetchedAtUnixMs": 1761845321000,
    "gasPriceGwei": 18.4,
    "venues": [
        {
            "chain": "Ethereum Mainnet",
            "dexName": "Uniswap V3 5bps",
            "pairLabel": "WETH / USDC",
            "priceUsd": 3047.23,
            "poolAddress": "0x88e6...5640",
            "feeTierBps": 5,
            "priceSourceLabel": "slot0() spot price",
            "fetchedAtUnixMs": 1761845321000
        },
        ...
    ]
}
```

### Stage 10: Frontend state update

```typescript
setOverview(nextOverview);
setSnapshot(nextOverview.venues[0] ?? null);
setHistory((currentHistory) => {
    const nextHistory = [...currentHistory, nextOverview];
    return nextHistory.slice(-HISTORY_LIMIT);  // = 100
});
```

Three state updates: current overview, "primary" snapshot (which is just `venues[0]`), and the rolling 100-sample history.

The `venues[0]` choice is implicit — it's the V3 5bps snapshot, which is the "main market line" by convention. Gap-related: there's no semantic contract beyond array order. A backend reorder of venues would silently change the hero view.

### Stage 11: Insights derivation

In the same render cycle:

```typescript
const insights = history.length > 0 ? deriveInsightsView(history) : null;
```

`deriveInsightsView` walks the entire history (up to 100 samples) and produces:

- One **primary insight card** (the most-prominent observation)
- Up to four **secondary insight cards** (venue ranking, spread regime, deviation leader, gas-adjusted view)
- Up to four **events** (recent transitions worth flagging)

The primary insight surfaces "Positive setup holding" when the cheapest-to-richest gas-adjusted route has stayed positive for `PERSISTENCE_WINDOW = 4` consecutive samples.

Detailed coverage of the insight engine in `project/systems/insight-engine-anatomy.md`.

### Stage 12: React re-render

State updates trigger React's reconciliation. The venue cards, chart, and insights panel all re-render with the new data. The chart's SVG paths are recomputed from the rolling history. Animations (if any) interpolate between old and new values.

The whole render cycle completes in single-digit milliseconds — well under the 1000ms tick budget.

## What This Misses

This walkthrough covers the happy path. Real ticks face:

- **RPC errors** — 503 from Alchemy, network timeout, malformed response. Caught via the error handling chain, surfaced as `setErrorMessage`.
- **Slow ticks** — when the round-trip exceeds 1000ms, the next tick is dropped (the `requestInFlight` guard).
- **Component unmount** — the `useEffect` cleanup function clears the interval; in-flight requests resolve into a stale state setter (technically a small memory leak but bounded).

## Pressure Points

1. **No partial-success handling** — Gap 2. One venue's failure kills the tick.
2. **No persistence** — Gap 1. Every tick re-queries; history evaporates on close.
3. **Polling is wasteful** — most ticks return identical data because the chain only updates every 12 seconds. Aurix polls 12× per block.
4. **The 100-sample window** is a soft constraint that limits insight quality. With persistence, insights could draw on much longer baselines.

## Related Files

- `project/architecture/two-runtime-tauri-rust-react.md` — the broader architecture
- `project/architecture/cross-runtime-contract.md` — IPC contract details
- `project/systems/insight-engine-anatomy.md` — what `deriveInsightsView` does
- `context/architecture.md` — implementation-facing reference
