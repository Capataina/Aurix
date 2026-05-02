# Round-Trip Pool-Fee Math (`lib/arbitrage.ts`)

## What this is

`computeRoute()` and `findBestRoute()` in `src/lib/arbitrage.ts` compute net P/L per round-trip when `pnlMode === "gas-and-fees"`. The math applies the per-leg pool fee to **both** the buy leg and the sell leg, not once. This note exists because the convention is non-obvious from the code alone and a future change could silently get it wrong.

## The math

Each leg of a round-trip arbitrage carries the venue's pool fee (5 bps for Uniswap V3 0.05%, 30 bps for V3 0.3%, etc.). The fee is applied as:

```
buyEffective  = buyPrice  / (1 - buyFeeRate)        // you pay slightly more than spot
sellEffective = sellPrice * (1 - sellFeeRate)       // you receive slightly less than spot
netPerUnit    = sellEffective - buyEffective - gasCostUsd / units
```

The `1 - feeRate` factor appears on **both** legs because:

- **Buy leg:** to acquire 1 unit of the base asset at price `P` you must put in `P / (1 - fee)` of the quote — the pool keeps `fee × deposited` and you receive the rest.
- **Sell leg:** when you sell 1 unit at price `P`, you receive `P × (1 - fee)` of the quote — the pool keeps `fee × P` and you receive the rest.

The two `(1 - fee)` factors do not cancel because they apply to different prices (`buyPrice` vs `sellPrice`). On a balanced market they cost roughly `2 × fee` of the spread; on a thin spread they swallow the entire opportunity. This is exactly why the gas-only P/L mode looks profitable for a stretch of ticks but the gas+fees mode shows the same opportunities as net negative.

## Why per-leg, not once

A common cheap-substitute is to deduct `2 × fee × spread` once at the round-trip level. That's wrong when fee rates differ between the buy and sell venues — e.g. buying on a 5 bps V3 pool and selling on a 30 bps V3 pool. Per-leg with the correct rate per venue is the only general-case-correct form.

`fee_tier_bps` is carried on each `VenuePayload` from the Rust backend (`src-tauri/src/dex/...`) so the frontend math always has the right rate for each leg.

## Why N×N, not closed-form

`findBestRoute()` iterates every `(buyVenue, sellVenue)` pair with `buyIndex !== sellIndex` because pool fees can flip the optimum. The naive "buy at the lowest, sell at the highest" answer is only correct if all venues have the same fee rate; mixed fee tiers can make a slightly worse spot price on a 5 bps venue beat a slightly better spot price on a 30 bps venue. The N×N scan is cheap (≤ 4 venues today, will stay ≤ ~10 even with multi-pair) and worth the correctness guarantee.

## Where this sits in the wider plan

This is a Tab 1 / arbitrage-page concern; it does not propagate to Vector A's LP backtester (Tab 2), which has its own fee accounting via the V3 swap-step inner loop and `feeGrowthInside` accumulation. The round-trip math here only models execution against a pool's spot price; the LP-side math models being-the-pool, which is fundamentally different.

## Surfacing in the UI

The "Pool fees" toggle in the topbar (`src/components/shell/TopBar.tsx`) flips `PnlMode` between `"gas"` (gas only — what the original Tab 1 showed) and `"gas-and-fees"` (gas + per-leg pool fees — the more honest number). Both are computed from the same history; switching modes does not reset state. `DEFAULT_PNL_MODE = "gas-and-fees"` because the more honest number is the default.
