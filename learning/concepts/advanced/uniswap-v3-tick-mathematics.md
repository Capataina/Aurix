# Uniswap V3 Tick Mathematics

## Why This Matters Here

Vector A (V3 LP backtester) requires implementing V3 tick math correctly. This is the file that prepares you for that. The math is non-trivial but well-defined; getting it wrong silently produces fee/IL numbers that look plausible but are wrong by 5-30%. Validation against on-chain reference positions is the only way to know your implementation is correct.

This file goes deeper than `concepts/core/amm-mechanics-v2-and-v3.md` — that file gives you the intuition, this one gives you the formulas you'd implement.

## Prerequisites

- `concepts/core/amm-mechanics-v2-and-v3.md` (you should already know V3 conceptually)
- `concepts/core/liquidity-providers-and-impermanent-loss.md` (you should understand the LP economics)
- Comfort with fixed-point arithmetic and notation

## Status

Foundational domain knowledge. Not yet implemented in Aurix — Vector A's plan is to implement this.

## Notation

| Symbol | Meaning | Range |
|---|---|---|
| `tick i` | Discrete price level | typically -887,272 to +887,272 |
| `P(i)` | Price at tick i | `P(i) = 1.0001^i` |
| `√P(i)` | Square root of price at tick i | `(1.0001)^(i/2)` |
| `√P_X96` | √P scaled by 2^96 | `√P × 2^96`, stored as Q64.96 |
| `L` | Liquidity of a position | `L = √(x × y)` |
| `tick_spacing` | Distance between selectable ticks | 10 (5bps), 60 (30bps), 200 (1%) |

## Q64.96 Fixed-Point Representation

V3 stores prices in Q64.96 fixed-point format. "Q64.96" means:
- 64 bits for the integer portion
- 96 bits for the fractional portion
- Total: 160 bits (which fits in a `uint160`)

To convert from Q64.96 to a "real" floating-point number:
- Q64.96_value / 2^96 = real_number

To convert from a real number to Q64.96:
- real_number × 2^96 = Q64.96_value

This is the same idea as scaling integers by 2^N to get fractional precision without using floats.

`sqrtPriceX96` is `√P` stored in Q64.96 format. To extract `√P`:
- `√P = sqrtPriceX96 / 2^96`

To extract `P`:
- `P = (sqrtPriceX96 / 2^96)² = sqrtPriceX96² / 2^192`

## Aurix's V3 Decode Walkthrough

`src-tauri/src/dex/uniswap_v3.rs` does this:

```rust
let numerator: BigUint = (BigUint::from(1u8) << 192) * BigUint::from(10u64).pow(TOKEN1_DECIMALS - TOKEN0_DECIMALS);
let denominator: BigUint = sqrt_price_x96.pow(2u32);
let numerator_f64 = numerator.to_f64()?;
let denominator_f64 = denominator.to_f64()?;
Ok(numerator_f64 / denominator_f64)
```

Let me unpack this. We have `sqrtPriceX96` as a `BigUint`. We want the WETH/USDC price (USDC per WETH) as `f64`.

The raw price (token1/token0 in their respective decimal-adjusted units):

> **P_raw = sqrtPriceX96² / 2^192**

But token0 is USDC (6 decimals) and token1 is WETH (18 decimals). The raw integer ratio doesn't account for this. To get the "human" price (USDC per WETH at 18-decimal scaling for both):

> **P_human = P_raw × 10^(token1_decimals - token0_decimals) = P_raw × 10^12**

Combining:

> **P_human = (sqrtPriceX96² × 10^12) / 2^192**

Wait, that's not what the code does. The code computes `numerator / denominator` where:
- `numerator = 2^192 × 10^12`
- `denominator = sqrtPriceX96²`

So the code computes:

> **(2^192 × 10^12) / sqrtPriceX96²**

Which is the inverse of what we said! `sqrtPriceX96² / 2^192` is the raw token1/token0 ratio. Aurix's code computes the inverse (`token0/token1`), which is USDC per WETH (because token0 is USDC and token1 is WETH).

This is correct — we want USD per WETH, and USDC IS the USD denominator. The formula gives you "USDC units per WETH unit," which after the 10^12 scaling is "USDC dollars per WETH" because USDC has 6 decimals and WETH has 18 decimals (so you need to scale up by 10^12 to express in equivalent decimal units).

The `BigUint` arithmetic is necessary because `sqrtPriceX96` is up to 160 bits, and `2^192` doesn't fit in any native integer. Once we've done the integer math, we convert to `f64` for the division (accepting some precision loss because the result fits comfortably in `f64`'s ~15-significant-digit representation).

## Tick-to-Price and Price-to-Tick Conversions

The V3 spec defines:

> **P(tick) = 1.0001^tick**

Or equivalently:

> **√P(tick) = (1.0001)^(tick/2) = 1.00005^tick** (approximately — see whitepaper for exact)

To convert tick → sqrtPrice (in Q64.96):

> **sqrtPriceX96(tick) = (1.0001)^(tick/2) × 2^96**

Computing this exactly requires careful fixed-point arithmetic. The OpenZeppelin or Uniswap V3 SDK implementations do it via a precomputed lookup table for powers of 2 (since 1.0001 ≈ 2^(1/13955.6...), exponentiation can be decomposed into bit-shifts for the integer portion).

To convert sqrtPrice → tick:

> **tick = floor(log_1.0001(P)) = floor(log_1.0001(sqrtPrice² / 2^192))**

Or more practically, using `log2` and the change-of-base:

> **tick = floor(2 × log2(sqrtPrice / 2^96) / log2(1.0001))**

The Uniswap V3 SDK provides `TickMath.getTickAtSqrtRatio(sqrtPriceX96)` which does this in one step.

## Liquidity, Reserves, and the L Variable

V3's `L` (liquidity) is the geometric mean of the two reserves:

> **L = √(x × y)**

where x and y are the reserves *as if the position were active everywhere* (the so-called "virtual reserves"). For a V3 position with a tight range, the virtual reserves are much larger than the actual deposited tokens — that's the source of capital efficiency.

For an LP whose range is `[tick_lower, tick_upper]` and current price is INSIDE the range:

> **x = L × (1/√P_current - 1/√P_upper)**
> **y = L × (√P_current - √P_lower)**

For an LP whose price has moved BELOW the range (current < lower):
- Position is 100% token0 (e.g. all USDC for a WETH/USDC pool where token0 is USDC)
- `x = L × (1/√P_lower - 1/√P_upper)`, `y = 0`

For an LP whose price has moved ABOVE the range (current > upper):
- Position is 100% token1 (e.g. all WETH)
- `x = 0`, `y = L × (√P_upper - √P_lower)`

These three cases are the entire V3 position composition logic. Vector A's simulation engine needs to handle all three correctly per swap.

## Liquidity For Amounts (Inverse)

To compute L from desired amounts:

For a position to be created with `amount0` and `amount1` at `√P_current` within `[√P_lower, √P_upper]`:

> **L_from_amount0 = amount0 × (√P_current × √P_upper) / (√P_upper - √P_current)**
> **L_from_amount1 = amount1 / (√P_current - √P_lower)**

> **L = min(L_from_amount0, L_from_amount1)**

(The min is because you're constrained by whichever side has less.)

## Fee Distribution Per Swap

This is where most V3 backtester implementations fail. The correct mechanism:

1. A swap of size Δ enters the pool at current price P
2. The pool walks through tick boundaries until either Δ is exhausted or a liquidity boundary is crossed
3. Within each "active range" (between tick boundaries), the swap pays the fee proportional to the amount swapped in that segment
4. The fee is distributed to LPs whose ranges include that segment, proportional to their share of the total liquidity active in that segment

For an LP simulation:

```
for each swap in history:
    if my_position.lower_tick <= current_tick <= my_position.upper_tick:
        # Position is active for this swap (or part of it)
        my_liquidity_share = my_position.liquidity / total_active_liquidity
        my_fee_earned = swap_fee_paid × my_liquidity_share
    
    # Update tick (the swap may have moved the price)
    current_tick = swap.tick_after
```

The naive mistake is treating fee distribution at the BLOCK or PERIOD level (averaging) — which produces "approximately right" numbers that are wrong by a few percent. Per-swap distribution, walking ticks, is the correct approach.

This is why Vector A's validation harness is essential: pick known on-chain LP positions, replay history, compare engine fees to actual collected fees. Per-swap implementations match within rounding; per-period averages don't.

## Impermanent Loss in V3

The V3 IL formula generalises V2's. For a V3 position with range `[√P_lower, √P_upper]`, entered at `√P_entry`:

When current `√P` is INSIDE the range, the formula reduces to V2's IL formula scaled by the "concentration factor":

> **concentration_factor = (√P_upper × √P_lower) / (√P_upper × √P_lower - √P_entry × (√P_upper - √P_lower))**

Higher concentration → higher IL per unit price move. A "wide" V3 range approximates V2's IL; a "tight" V3 range can have IL many times higher.

When current `√P` is OUTSIDE the range, the position is 100% one token and the IL is the maximum (the LP missed all upside on the side they're not holding).

## Worked Example — A V3 Position Over a Price Move

Let's track a V3 position through a price move:

**Setup:**
- LP enters at price $3,000 (so √P_entry = ~54.77)
- Range: $2,800 to $3,200 (so √P_lower = ~52.92, √P_upper = ~56.57)
- Deposit: 1 WETH + 3,000 USDC at entry (total: $6,000)

Computing `L`:
- L_from_amount0 = 3000 × (54.77 × 56.57) / (56.57 - 54.77) ≈ 3000 × 3098 / 1.80 ≈ 5,163,000
- L_from_amount1 = 1 / (54.77 - 52.92) ≈ 1 / 1.85 ≈ 0.541

Wait, the units don't match — these need to be in the same scale. Real V3 math uses Q64.96 throughout; the example here is illustrative. Let's just track the qualitative result.

**Price moves to $3,100 (inside the range):**

The pool's math automatically rebalanced your position. You now have less WETH and more USDC than at entry (because the AMM "sold" some of your WETH as the price rose).

You earn fees from every swap that touched the pool while in your range. If 100,000 USDC of volume happened with you holding 1% of in-range liquidity, you earned roughly 100,000 × 0.0005 (5bps fee) × 0.01 = $0.50 in fees over that period.

**Price moves to $3,250 (above the range):**

Your position is now 100% USDC. You stopped earning fees the moment the price exited your range. Whatever fees you earned while in-range are yours; you accumulate no more.

If the price stays above $3,200, you have all USDC and have missed the WETH appreciation. Your value: ~$3,400 (from the original $6,000, after various rebalancing). Hold-only baseline: 1 × $3,250 + $3,000 = $6,250. IL = $6,250 - $3,400 = $2,850.

**Price returns to $3,000:**

Your position re-enters your range. You start earning fees again. Your composition shifts back toward the entry mix. If the price returns exactly to $3,000, your position composition returns to the entry composition, and the IL goes back to (approximately) zero — minus the fees you earned during the journey.

## Validation Strategy for Vector A

To trust your V3 implementation, you need to validate against real positions. Procedure:

1. Pick 5 known LP positions from on-chain
2. For each, find the mint tx and burn tx (publicly visible)
3. From the mint tx: extract entry parameters (amounts, tick range, block)
4. From the burn tx: extract exit composition and total collected fees
5. Replay the position through your engine using the actual block range
6. Compare: did your engine's collected fees match the on-chain truth within tolerance?

Tolerance: 0.5% of fees, or $1, whichever is larger. Closer is better. If you're off by more than 1%, you have a bug — most likely in fee distribution, tick crossing, or boundary handling.

## Resources for Going Deeper

- **Uniswap V3 Whitepaper** (`materials/amm-foundational-resources.md`) — the primary source. Section 6 has the math.
- **Uniswap V3 SDK** (`@uniswap/v3-sdk` on npm) — has reference implementations of all the tick math
- **OpenZeppelin V3 contracts** — production Solidity implementations
- **Uniswap V3 Math by Atis E** — a free PDF with derivations

## Common Misunderstandings

❌ **"Tick math is easy."** It's not. The combination of fixed-point arithmetic, tick boundary handling, fee growth tracking, and edge cases (zero liquidity, concentrated positions at the current tick) is genuinely hard. Plan for multiple iterations of debugging against the validation harness.

❌ **"f64 is fine for V3 math."** For DISPLAY, sure. For computation that touches ticks, fee growth tracking, or position composition over time, f64 introduces drift that compounds. Use BigUint or u256 throughout the computation; convert to f64 only for the final display.

❌ **"My implementation is right because the test cases pass."** Your test cases are right because they came from your implementation. The only meaningful validation is against external truth — on-chain positions whose collected fees you can verify via Etherscan.

❌ **"Per-swap fee distribution is approximately equal to per-period averaging."** It's not. Distribution shape matters. A swap at the edge of a tight range distributes fees very differently than one in the middle. The approximation can be off by 5-30%.

## Related Files

- `concepts/core/amm-mechanics-v2-and-v3.md` — the conceptual foundation
- `concepts/core/liquidity-providers-and-impermanent-loss.md` — IL economics
- `project/comparisons/v2-vs-v3-amm-math.md` — side-by-side
- `materials/amm-foundational-resources.md` — V3 whitepaper, SDK references
- `context/plans/vector-a-v3-lp-backtester.md` — the implementation plan
- `exercises/core/decode-sqrtpricex96.rs` — practice decoding sqrtPriceX96 by hand
