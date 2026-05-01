# Cross-Runtime Contract

## Why This Matters

Aurix's two runtimes (Rust + TypeScript) communicate via Tauri IPC. The contract that defines what crosses the boundary — field names, types, encoding — is the most fragile piece of the architecture because it has no automated enforcement. A field rename in Rust without a matching TypeScript change compiles cleanly on both sides and fails silently at runtime. This file documents the contract, the convention that holds it together, and the failure modes when it drifts.

## The Contract Surface

Two struct families currently cross the boundary:

```rust
// src-tauri/src/market/types.rs

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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketOverview {
    pub chain: String,
    pub pair_label: String,
    pub fetched_at_unix_ms: u64,
    pub gas_price_gwei: f64,
    pub venues: Vec<PriceSnapshot>,
}
```

And their TypeScript mirrors:

```typescript
// src/features/arbitrage/types.ts (mirror — manually maintained)

export interface PriceSnapshot {
    chain: string;
    dexName: string;
    pairLabel: string;
    priceUsd: number;
    poolAddress: string;
    feeTierBps: number;
    priceSourceLabel: string;
    fetchedAtUnixMs: number;
}

export interface MarketOverview {
    chain: string;
    pairLabel: string;
    fetchedAtUnixMs: number;
    gasPriceGwei: number;
    venues: PriceSnapshot[];
}
```

That's the entire contract surface today. One Tauri command (`fetch_market_overview`) returns one struct family.

## The camelCase Convention

The `#[serde(rename_all = "camelCase")]` attribute is what bridges Rust's snake_case convention (idiomatic Rust) to JavaScript's camelCase convention (idiomatic JS). Without it, the Rust side would emit `dex_name` and the JavaScript side would expect `dexName` — every field would silently be `undefined`.

The convention has to be applied to EVERY struct that crosses the boundary. Forgetting it on a new struct produces silent failures.

This is documented as a project convention in `context/notes/wire-convention.md`.

## Type Mapping

| Rust type | JSON wire format | TypeScript type | Notes |
|---|---|---|---|
| `String` | string | `string` | UTF-8 |
| `f64` | number | `number` | JS numbers are f64, exact match |
| `u16`, `u32`, `u64` | number | `number` | u64 > 2^53 risks JS precision loss |
| `bool` | boolean | `boolean` | direct |
| `Vec<T>` | array | `T[]` | direct |
| `Option<T>` | T or null | `T \| null` | Serde defaults to `null` for None |
| `enum` | tagged object or string | union type | Depends on Serde tag attribute |

The `u64` precision issue: JavaScript numbers are IEEE 754 doubles, which can represent integers exactly up to 2^53. Aurix's `fetched_at_unix_ms: u64` fits well within this range (even billions of years from now), but a 256-bit value crossing as `u64` would silently truncate.

## Why No Automated Check

There are several options for enforcing the contract:

| Approach | Description | Trade-off |
|---|---|---|
| **`ts-rs` codegen** | Rust crate that emits TypeScript types from Rust structs | Adds build step, requires struct annotations |
| **Specta** | Type-safe Tauri command bindings | Requires using a specific Tauri pattern |
| **Runtime validator (zod)** | TypeScript validation at the boundary | Adds runtime overhead, partial protection |
| **Contract test** | Round-trip a known payload, verify equivalence | Catches major changes, misses subtle ones |
| **Manual mirror** (current) | Convention discipline only | Zero overhead, zero protection |

Aurix uses the last (manual mirror) for now. This is documented as Gap 11 in the gap inventory — a known fragility.

When Vector A or Vector B ships, the contract surface will expand significantly (new commands, new struct families). At that point, automation becomes more valuable. The likely path: introduce `ts-rs` for new struct families, leave existing ones alone.

## What Crossing The Boundary Looks Like

The actual JSON for one tick:

```json
{
    "chain": "Ethereum Mainnet",
    "pairLabel": "WETH / USDC",
    "fetchedAtUnixMs": 1761845321000,
    "gasPriceGwei": 18.43,
    "venues": [
        {
            "chain": "Ethereum Mainnet",
            "dexName": "Uniswap V3 5bps",
            "pairLabel": "WETH / USDC",
            "priceUsd": 3047.234567890,
            "poolAddress": "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640",
            "feeTierBps": 5,
            "priceSourceLabel": "slot0() spot price",
            "fetchedAtUnixMs": 1761845321001
        },
        {
            "chain": "Ethereum Mainnet",
            "dexName": "Uniswap V3 30bps",
            ...
        },
        {
            "chain": "Ethereum Mainnet",
            "dexName": "Uniswap V2",
            ...
        },
        {
            "chain": "Ethereum Mainnet",
            "dexName": "SushiSwap",
            ...
        }
    ]
}
```

About 1-2 KB per tick. At 1 Hz polling = ~7 MB/hour of cross-runtime traffic. Negligible for desktop, but worth noting if Aurix ever scaled to higher polling rates.

## Failure Modes

The contract drifts in predictable ways:

### Field Rename in Rust Without TypeScript Update

Rust:
```rust
pub price_usd: f64,  →  pub price_usdc: f64,
```

Wire format silently changes from `priceUsd` to `priceUsdc`. TypeScript still expects `priceUsd`, accesses `overview.venues[0].priceUsd`, gets `undefined`, renders `$NaN` or breaks downstream calculations. No type error, no runtime exception, just silently-wrong output.

### Type Change in Rust Without TypeScript Update

Rust:
```rust
pub price_usd: f64,  →  pub price_usd: String,
```

Wire format becomes `"priceUsd": "3047.23"` (string) instead of `"priceUsd": 3047.23` (number). TypeScript's `priceUsd: number` type system believes it's a number; runtime arithmetic on a string produces `NaN` or string concatenation. Sometimes silently wrong, sometimes obviously broken.

### Field Removed in Rust

The TypeScript side accesses a field that no longer exists, gets `undefined`. Same silent failure as rename.

### Field Added in Rust

The TypeScript side ignores it (extra fields don't break deserialisation). Less risky — the new field is just unused on the frontend.

### Convention Violation

Forgetting `#[serde(rename_all = "camelCase")]` on a new struct. The Rust side emits snake_case; TypeScript expects camelCase; every field is `undefined`. Catastrophic but obvious in development.

## The dex_name Identity Contract

Beyond the struct contract, there's an **implicit string-key contract**: the `dex_name` field's value (`"Uniswap V3 5bps"`, `"Uniswap V3 30bps"`, `"Uniswap V2"`, `"SushiSwap"`) is used as a lookup key in two frontend places:

- `src/features/arbitrage/ArbitragePage.tsx`'s `VENUES` array (for venue card metadata)
- `src/features/arbitrage/components/MarketChart.tsx`'s `SERIES_META` table (for chart line colours)

A rename like `"Uniswap V3 5bps"` → `"Uniswap V3 (5 bps)"` in Rust silently breaks both lookups. The venue card renders `$0.00` (from the `?? 0` fallback) and the chart crashes on undefined accent.

This is the most subtle contract — a string value, not a struct field. Documented in `context/notes/dex-name-contract.md`.

## How This Will Scale

When Vector A ships:
- New IPC commands: `start_backtest`, `query_backtest_results`, `query_strategy_grid`, etc.
- New struct families: `BacktestRun`, `LpStrategy`, `EquityPoint`, `StrategyComparison`
- The contract surface roughly triples

When Vector B ships:
- New WebSocket-driven IPC: continuous streams (Tauri supports events for this)
- New struct families: `MempoolTx`, `SandwichOpportunity`, `LatencyHistogram`

When Vector C ships:
- New commands: `query_prediction`, `query_calibration_diagnostics`
- New struct families: `Prediction`, `ReliabilityBin`, `FeatureImportance`

At that scale, manual mirror maintenance becomes untenable. The likely transition: introduce `ts-rs` for new struct families, deprecate manual mirrors gradually.

## Related Files

- `project/architecture/two-runtime-tauri-rust-react.md` — the broader two-runtime architecture
- `project/architecture/the-1hz-loadsnapshot-tick.md` — what crosses the boundary every second
- `context/notes/wire-convention.md` — the project's official convention
- `context/notes/dex-name-contract.md` — the string identity contract
- `context/architecture.md` §Critical Paths and Blast Radius — what breaks when this drifts
