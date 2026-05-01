# AMM Mechanics: V2 and V3

## Why This Matters Here

This is the single most important concept file in the archive. Every venue Aurix watches is an AMM, every price Aurix decodes comes from AMM math, and every Aurix vector (especially Vector A — V3 LP backtester) requires understanding this material. If you read only one concept file, read this one — and re-read until the worked examples land.

## Prerequisites

- `concepts/foundations/markets-and-prices.md` (you should know what a price and slippage are)
- `concepts/foundations/exchanges-orderbook-vs-amm.md` (you should know what an AMM is at the conceptual level)

## Notation

| Symbol | Meaning |
|---|---|
| `x` | Reserve of token0 in the pool |
| `y` | Reserve of token1 in the pool |
| `k` | Invariant constant: `x × y = k` (V2) |
| `Δx` | Amount of token0 added to the pool by a trader |
| `Δy` | Amount of token1 removed from the pool by a trader |
| `P` | Implied price (token1 per token0, e.g. USDC per WETH) |
| `√P` | Square root of price — V3 stores this rather than P directly |
| `L` | V3 pool's "liquidity" — the geometric mean of reserves |
| `tick` | Discrete price level in V3 (each tick = 1.0001× price change) |

## V2 — The Constant Product Formula

Uniswap V2 (and SushiSwap, and most "x*y=k" AMMs) pricing is governed by one equation:

> **x × y = k**

Where `x` is the amount of token0 in the pool, `y` is the amount of token1, and `k` is a constant that doesn't change across swaps (modulo fees, which we'll add in a moment). Any swap must preserve `k`.

The implied price (token1 per token0) at any moment is:

> **P = y / x**

That's it. Two equations and you have the entire V2 pricing model.

### Worked Example 1 — A simple swap

A WETH/USDC pool starts with 100 WETH (token0) and 300,000 USDC (token1). So:

- `k = 100 × 300,000 = 30,000,000`
- `P = 300,000 / 100 = $3,000` per WETH

You want to swap 1 WETH for USDC. After the swap:

- The pool has 101 WETH (you added 1)
- For `k` to stay constant: `101 × y_new = 30,000,000`, so `y_new = 297,029.70`
- The pool LOST `300,000 - 297,029.70 = 2,970.30 USDC`
- That's what you receive: **2,970.30 USDC for your 1 WETH**

Wait — the implied price was $3,000, but you got $2,970.30. Where did the missing $29.70 go?

It didn't go anywhere. It never existed. The "implied price" of $3,000 is the price for an *infinitesimally small* trade — the marginal price at the current reserves. The moment you add 1 WETH, the pool's reserves shift, the implied price drops, and your *average* execution price for the 1 WETH is lower than the starting marginal price.

This $29.70 difference is **slippage**. It's not a fee, not a cost paid to anyone — it's the inherent geometric consequence of a curve where price drops as you add more of one token.

After your swap, the pool is at (101 WETH, 297,029.70 USDC). New implied price: `297,029.70 / 101 = $2,941.88`. The next person to swap WETH→USDC will face this new (lower) marginal price.

### Worked Example 2 — A bigger swap

Same starting pool: 100 WETH, 300,000 USDC, k = 30 million, P = $3,000.

You want to swap 10 WETH for USDC. After:

- Pool has 110 WETH
- `y_new = 30,000,000 / 110 = 272,727.27`
- USDC removed: `300,000 - 272,727.27 = 27,272.73`
- Your average execution price: `27,272.73 / 10 = $2,727.27` per WETH

A 10× larger swap got 9.18% worse execution ($2,727 vs $3,000). That's not linear — slippage scales worse than proportionally with trade size. The function is convex: doubling your trade size more than doubles your slippage cost.

This is why DEX aggregators like 1inch and Uniswap's Universal Router split large orders across multiple pools — splitting reduces total slippage even when each leg pays its own gas.

### Worked Example 3 — Adding the fee

V2's fee is 0.30% of the input. The mechanics: when you put 1 WETH in, only 0.997 WETH actually enters the formula; 0.003 WETH stays in the pool as fees, distributed to LPs.

Same starting pool, same 1 WETH swap:

- Effective input: 0.997 WETH
- `(100 + 0.997) × y_new = 30,000,000`
- `y_new = 30,000,000 / 100.997 = 297,038.43`
- USDC out: `300,000 - 297,038.43 = 2,961.57`

Compared to no-fee scenario (2,970.30 USDC), the fee cost the trader 8.73 USDC. That 8.73 stays in the pool, increasing `k` slightly, and is distributed to LPs proportional to their pool share.

After many swaps, k grows over time (purely from accumulated fees). LPs withdraw at any time and get their share of the (now larger) pool.

### V2 Fee Tiers

V2 has one fee tier: 0.30%. SushiSwap (V2-fork) is also 0.30%. There's no "choose your fee" in V2 — every swap pays the same.

This is a constraint V3 lifts.

## V3 — Concentrated Liquidity

Uniswap V3's innovation: liquidity providers choose a **price range** within which their capital is active.

```
   V2: liquidity spread uniformly across all prices ($0 to ∞)
   ████████████████████████████████████████████████████████  ← LP's capital
   $0   $1k  $2k  $3k  $4k  $5k  $6k  $7k  $8k  $9k  ...
   
   V3: liquidity concentrated in a chosen range (e.g. $2,800 - $3,200)
   ░░░░░░░░░░░░████████████████████░░░░░░░░░░░░░░░░░░░░░░░  ← LP's capital
   $0   $1k  $2k  $3k  $4k  $5k  $6k  $7k  $8k  $9k  ...
```

Same total dollars deposited, but in V3 they're concentrated in the range you predict the price will spend most of its time. Inside the range you earn fees from every swap that touches it; outside, you earn zero.

Inside the active range, V3's pricing math is identical to V2 — the same `x * y = k` curve, just with virtual reserves that make the LP's actual capital behave AS IF it were a much larger V2 position. Outside the range, your position is entirely one token (whichever side the price exited toward).

### Why "Square Root of Price"?

V3 stores price as `√P` rather than P directly. Two reasons:

1. **Computation efficiency.** Many of the swap math operations involve square roots of price. Storing `√P` directly skips the per-swap square root calculation.

2. **Liquidity unit consistency.** The "liquidity" `L` of a V3 position has the property that `L = √(x × y)` — it's the geometric mean of reserves. Storing prices in `√P` units makes the liquidity math clean.

The encoding `sqrtPriceX96` is `√P × 2^96`, stored as a 160-bit unsigned integer. The `2^96` scaling is to give precision (since we can't store fractional values in an integer); we just shift right by 96 (or, equivalently, divide by 2^96) when we want the actual `√P`.

To get the actual price `P` from `sqrtPriceX96`:

> **P = (sqrtPriceX96 / 2^96)² = sqrtPriceX96² / 2^192**

For a WETH/USDC pool where the result is "USDC per WETH at the raw decimal level," you then need to multiply by `10^(WETH_decimals - USDC_decimals) = 10^12` to get a human-readable price.

This is exactly what Aurix's `dex/uniswap_v3.rs` does:

```rust
let numerator: BigUint = (BigUint::from(1u8) << 192) * BigUint::from(10u64).pow(TOKEN1_DECIMALS - TOKEN0_DECIMALS);
let denominator: BigUint = sqrt_price_x96.pow(2u32);
// P = numerator / denominator
```

The numerator is `2^192 × 10^12`, the denominator is `sqrtPriceX96²`. The ratio is the WETH/USDC price.

### Ticks

V3 doesn't let LPs choose arbitrary price ranges — only ranges aligned to discrete **ticks**. A tick is a price level where:

> **price(tick) = 1.0001^tick**

So tick 0 corresponds to price 1.0, tick 1 to 1.0001, tick 100 to ~1.01, tick -1000 to ~0.9048, and so on. Each tick is a 1 basis point (0.01%) price change from the next.

The tick spacing depends on the fee tier:
- 5bps fee tier: tick spacing 10 (every 10 ticks selectable)
- 30bps fee tier: tick spacing 60
- 100bps fee tier: tick spacing 200

This is why V3 LP positions are described by `(lower_tick, upper_tick)` rather than `(lower_price, upper_price)` — ticks are the actual primitive.

### V3 Fee Tiers

V3 supports multiple fee tiers per pool. For WETH/USDC, three pools exist:
- **5bps** (0.05%) — the main pool, deepest liquidity
- **30bps** (0.30%) — for higher-volatility periods
- **100bps** (1.00%) — exists but tiny liquidity for major pairs

Aurix watches both 5bps and 30bps because the spread between them is itself an interesting signal (when they diverge, it suggests fee tier preferences are shifting due to recent activity).

## Comparing V2 and V3

| Property | V2 | V3 |
|---|---|---|
| Math | `x * y = k` | `x * y = k` within active range, modified by tick boundaries |
| LP capital coverage | $0 to ∞ | LP-chosen `[lower_tick, upper_tick]` |
| Capital efficiency | Low | Up to ~4000× higher in tight ranges |
| Fee tiers | 0.30% only | 0.05%, 0.30%, 1.00% |
| Position fungibility | LP positions are fungible (ERC-20 LP tokens) | Positions are NFTs (each is unique) |
| Impermanent loss | Symmetric, predictable | Higher in narrow ranges (more sensitive to price moves) |
| Implementation complexity | Simple | Significantly more complex |

The trade-off: V3 LPs earn more fees per dollar deposited (because their capital is concentrated where trades happen) BUT they bear more impermanent loss risk (because their range can be exited entirely, leaving them holding only one side). It's higher reward AND higher risk.

This trade-off is the central topic of `concepts/core/liquidity-providers-and-impermanent-loss.md`.

## How This Appears in Aurix

Aurix decodes V3 prices from `sqrtPriceX96` and V2 prices from `getReserves()`. The two adapters live in separate files because their decoding paths are different:

**`src-tauri/src/dex/uniswap_v3.rs`:**
1. Call `slot0()` on the pool contract via `eth_call`
2. Take the first 32-byte word of the response (`sqrtPriceX96`)
3. Decode as `BigUint::from_bytes_be`
4. Compute `(2^192 × 10^12) / sqrt²` to get the price as `f64`

**`src-tauri/src/dex/uniswap_v2.rs`:**
1. Call `getPair(USDC, WETH)` on the factory contract to find the pair address
2. Call `getReserves()` on the pair contract to get `(reserve0, reserve1)`
3. Call `token0()` to determine which reserve is which token
4. Compute `(reserve_usdc / reserve_weth) × 10^12` to get the price as `f64`

Different protocols, different decoding paths, same concept: read the pool state, derive the implied price.

## Common Misunderstandings

❌ **"V3 is just V2 with extras."** V3 is mathematically a generalisation of V2 — set V3's range to `[-∞, +∞]` and you recover V2 behaviour. But the implementation, the position management, and the LP economics are all genuinely different. Treating V3 as "V2 with bells" leads to wrong intuitions about IL and capital allocation.

❌ **"AMMs price tokens at their 'true' value."** AMMs price tokens at *whatever the most recent swap left the reserves at*. The "true" value is whatever traders are willing to swap them for. There's no anchor to fundamental value — only to recent trading activity, kept in line with other venues by arbitrage.

❌ **"Slippage is a fee."** Slippage is a geometric consequence of the curve, not a fee. No one collects your slippage. It "goes" to the pool's new reserve composition, where it's effectively returned to LPs over time as the pool gets re-balanced by future trades.

❌ **"V3 LPs earn more, period."** V3 LPs earn more PER DOLLAR DEPLOYED in active ranges, but their range can be exited entirely (in which case they earn nothing AND face the maximum IL). Average V3 LP returns aren't necessarily higher than V2 — they're more variance-y.

## Related Files

- `concepts/core/liquidity-providers-and-impermanent-loss.md` — the LP side of these mechanics
- `concepts/core/traders-and-slippage.md` — the trader side
- `concepts/core/arbitrage-and-cross-venue-equilibrium.md` — what keeps AMM prices aligned across venues
- `concepts/advanced/uniswap-v3-tick-mathematics.md` — the deep V3 math (Q64.96, tick crossings, fee growth tracking)
- `project/comparisons/v2-vs-v3-amm-math.md` — side-by-side
- `project/systems/insight-engine-anatomy.md` — how Aurix decodes both
- `materials/amm-foundational-resources.md` — V2 and V3 whitepapers
