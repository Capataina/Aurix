# Liquidity Providers and Impermanent Loss

## Why This Matters Here

LPs are one of the three economic roles in any AMM market. Aurix doesn't currently model LP behaviour, but Vector A (V3 LP backtester) is entirely about answering "what would have happened to an LP in this position over this period?" To build Vector A correctly, you need to understand what LPs actually do, what fees they earn, and the impermanent loss they take on. This file covers all three with worked examples.

## Prerequisites

- `concepts/core/amm-mechanics-v2-and-v3.md` (you must understand `x * y = k` and how V3 ranges work)

## What an LP Does

A liquidity provider deposits two tokens into an AMM pool in equal value (priced at the current pool ratio). In return, they receive a "share" representing their fractional ownership of the pool. They can withdraw at any time by burning the share, receiving back their proportional slice of whatever the pool currently holds.

For example, in a V2 WETH/USDC pool with 100 WETH and 300,000 USDC at $3,000/WETH:
- You deposit 1 WETH + 3,000 USDC (total value: $6,000, current price)
- You receive an LP share representing 1/101 of the pool (you added 1% to each side, ~1% pool ownership)
- You can later burn the share to receive your share of the pool's current state — which will NOT be 1 WETH + 3,000 USDC unless the price hasn't moved

The pool's math will have rebalanced your position automatically as swaps happen against the pool. This is the source of impermanent loss.

## What LPs Earn — Fees

Every swap through the pool pays a fee (0.30% in V2, 0.05% / 0.30% / 1.00% in V3 depending on the pool). The fee stays in the pool and is distributed pro-rata to LPs based on their share of the pool's liquidity at the moment of the swap.

In V2, this is simple: every LP gets a fraction of every swap proportional to their pool share.

In V3, it's more complex: only LPs whose price range covers the current price are "active" and earn fees from a given swap. An LP whose range doesn't include the current price earns zero from swaps that don't touch their range.

This is the V3 trade-off: concentrated liquidity earns more fees per dollar IF the price stays in the chosen range, and earns ZERO if it doesn't.

## What LPs Risk — Impermanent Loss

Impermanent loss (IL) is the difference between an LP's position value and what they would have had if they'd just held the original tokens (without LPing).

The "loss" is "impermanent" because it only crystallises if you withdraw at a different price than you entered. If the price returns to the entry price, the IL goes back to zero. The moment you withdraw at a different price, the IL becomes permanent.

### Worked Example — Simple V2 IL

You enter a V2 WETH/USDC pool at price $3,000:
- Deposit: 1 WETH + 3,000 USDC
- Initial value: $6,000
- Pool composition: assume you're 1% of a 100/30,000 pool, so you own (1 WETH, 300 USDC) — but for simplicity let's pretend the pool is exactly your position. Then your reserves are (1 WETH, 3,000 USDC) and `k = 3,000`.

Wait — let me redo with the simpler "you're the entire pool" setup:

- Initial pool reserves: x = 1 WETH, y = 3,000 USDC, k = 3,000, P = $3,000
- Your initial value: $3,000 USDC + $3,000 WETH (at $3,000) = $6,000

Now suppose the price doubles to $6,000 (driven by external arbitrage). The pool's new state must satisfy `x × y = 3,000` AND `y / x = 6,000` (the new price). Solving:

- `x = √(3000/6000) = √0.5 ≈ 0.707 WETH`
- `y = 6,000 × 0.707 ≈ 4,243 USDC`
- Sanity check: `0.707 × 4,243 ≈ 3,000 ✓`

So the pool now has roughly 0.707 WETH and 4,243 USDC. Your withdrawal would give you those amounts, valued at the new price:

- WETH value: 0.707 × $6,000 = $4,243
- USDC value: $4,243
- Total: **$8,486**

Now compare to the hold-only baseline. If you'd just held your original 1 WETH + 3,000 USDC without LPing:
- WETH value: 1 × $6,000 = $6,000
- USDC value: $3,000
- Total: **$9,000**

You "lost" $9,000 - $8,486 = **$514** by being an LP instead of holding. That's IL.

Notice: you didn't lose money in absolute terms — you went from $6,000 to $8,486, a 41% gain. You just gained less than you would have by holding. IL is an opportunity cost, not a cash loss.

### The Symmetry of IL

If the price had HALVED to $1,500 instead of doubling to $6,000:

- New x = √(3000/1500) = √2 ≈ 1.414 WETH
- New y = 1500 × 1.414 ≈ 2,121 USDC
- Withdrawal value: 1.414 × $1,500 + $2,121 = $2,121 + $2,121 = $4,243

Hold-only baseline:
- WETH: 1 × $1,500 = $1,500
- USDC: $3,000
- Total: $4,500

Difference: $4,500 - $4,243 = $257.

The IL is symmetric in log-price space. Doubling and halving produce the same magnitude of IL relative to the hold-only baseline.

### IL Formula (V2)

The closed-form IL formula for V2:

> **IL(r) = (2√r) / (1 + r) − 1**

where `r = price_now / price_entry`.

For r = 2 (price doubled): IL = (2√2)/3 − 1 ≈ 0.943 − 1 = -5.7%  
For r = 1.5 (price up 50%): IL ≈ -2.0%  
For r = 1 (price unchanged): IL = 0%  
For r = 0.5 (price halved): IL ≈ -5.7%  
For r = 4 (price quadrupled): IL ≈ -20%  
For r = 10 (price 10×ed): IL ≈ -42.5%

This shows IL is non-linear: small price moves produce small IL, but large moves produce devastatingly large IL. A price 10× move costs you 42.5% relative to holding.

### V3 IL is Worse (Per Range)

V3 concentrates capital in a narrower range, which amplifies both fee earnings AND IL. The math is more complex (depends on the chosen range and the price path), but the intuition is:

- A V3 position in `[$2,800, $3,200]` with a small price move stays in range and earns more fees per dollar than V2
- The same V3 position with a price move OUTSIDE that range becomes 100% one token (whichever side the price crossed) and earns zero fees while suffering maximum IL

The V3 LP's problem is choosing the right range. Too tight: you exit it quickly and earn nothing. Too wide: you might as well be V2. The optimal range depends on volatility and your fee/IL trade-off appetite.

## Fees vs IL: The LP's Bet

An LP bets that **fees earned during the period > impermanent loss**. Whether this works out depends on:

| Variable | Effect on fees | Effect on IL |
|---|---|---|
| **Volatility** | More vol = more swap activity = more fees | More vol = more IL |
| **Trading volume** | More volume = more fees | No effect |
| **Price path (range-bound vs trending)** | No direct effect | Range-bound = low IL; trending = high IL |
| **Fee tier (V3)** | Higher tier = more per swap, but less swaps probably | No effect |
| **Range width (V3)** | Tighter range = higher fees per dollar in range | Tighter range = higher IL when in range |

The general rule: **range-bound, high-volume markets are good for LPs**. Trending markets with sustained directional moves are bad.

For Aurix's WETH/USDC pair: this is a relatively volatile pair (WETH is volatile against USDC), so V3 LPs in tight ranges earn meaningful fees but also face meaningful IL. Whether LPing beats holding depends on the specific period and range chosen — exactly what Vector A's backtester is designed to answer empirically.

## How This Appears in Aurix

Aurix doesn't currently model LP behaviour at all. Vector A (V3 LP backtester) is the entire feature for this:

- Replay every historical swap through a chosen LP position
- Compute exact fees earned per swap (LP's fraction of in-range liquidity at that moment)
- Track impermanent loss continuously via the V3 tick math
- Output: equity curve showing position value, fees, IL, and hold-only baseline overlaid

The validation harness compares engine output to known on-chain LP positions (mint and burn tx hashes are public, so collected fees are verifiable to within rounding).

When Vector A ships, it will live in `src-tauri/src/backtest/` with a Tauri command exposed to the frontend. The frontend will be a new tab in the app shell (which doesn't exist yet — the tab shell itself is Gap 9).

## Common Misunderstandings

❌ **"IL means I lose money."** IL is opportunity cost, not absolute loss. You can have positive IL (the worst case) and still have made money in absolute terms. The question is whether your fees exceeded your IL — that's the LP's actual bet.

❌ **"V3 LPs always earn more than V2 LPs."** V3 LPs earn more *per dollar deployed in active ranges* but face higher IL and can spend periods entirely out of range earning nothing. Average V3 returns vs V2 returns depend on the LP's range-management skill.

❌ **"You can avoid IL by exiting at the same price you entered."** True but not actionable. If you knew when the price would return to entry, you'd just trade on that knowledge instead of LPing. In practice, LPs face IL because they can't perfectly time entry/exit.

❌ **"Stablecoin pairs (USDC/USDT) have no IL."** True for tightly-pegged stablecoins. But Curve and similar stablecoin AMMs use modified curves that further reduce IL. For volatile pairs (WETH/USDC), IL is meaningful and unavoidable.

❌ **"More fees = better LP outcome."** Only if fees > IL. A high-volume volatile pair generates lots of fees AND lots of IL; a low-volume stable pair generates few fees but minimal IL. Net outcome depends on which side wins.

## Related Files

- `concepts/core/amm-mechanics-v2-and-v3.md` — the underlying math
- `concepts/advanced/uniswap-v3-tick-mathematics.md` — tick math for V3 position simulation
- `concepts/core/arbitrage-and-cross-venue-equilibrium.md` — arbitrage is what causes the price moves that drive IL
- `context/plans/vector-a-v3-lp-backtester.md` — the implementation plan
- `materials/amm-foundational-resources.md` — V3 whitepaper section 6 covers the math directly
