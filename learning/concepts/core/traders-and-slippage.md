# Traders and Slippage

## Why This Matters Here

Slippage is the difference between the marquee price and the price you actually get when you swap. It's the consequence of AMM math meeting real trade sizes. For Aurix specifically, slippage is one of the three reasons the "free money" arbitrage opportunities the dashboard surfaces are usually unprofitable to act on (the others being gas and bot competition). Understanding slippage at the math level is essential to reasoning about whether any displayed opportunity could actually be captured.

## Prerequisites

- `concepts/core/amm-mechanics-v2-and-v3.md` (you must understand `x * y = k` and how it produces an implied price)

## The Three Costs a Trader Faces

When you swap on an AMM, you pay three costs:

1. **The swap fee** — the AMM's explicit fee (0.05% / 0.30% / 1.00% of input depending on pool)
2. **Slippage** — your trade moves the pool's price, so your average execution is worse than the marquee price
3. **Gas** — Ethereum transaction cost in ETH-denominated units

Slippage is the one that scales with trade size. The fee is a fixed percentage; gas is a roughly fixed unit cost (varies with network congestion but not with trade size). Slippage grows as a function of (your trade size) / (pool depth).

## Where Slippage Comes From

Recall the V2 swap formula. Pool starts at (x, y) with k = x*y and P = y/x. You add Δx to the pool. New state:

- `(x + Δx) × y_new = k`, so `y_new = k / (x + Δx)`
- You receive: `Δy = y - y_new = y - k/(x+Δx)`

Your **average execution price** is `Δy / Δx` (the USDC you got per WETH you paid).

The marquee price was `P = y/x`. Your average price is `Δy/Δx`. The gap is slippage.

Doing the algebra:

> **Δy/Δx = y / (x + Δx)**

Compare to marquee: `P = y/x`. The ratio:

> **(Δy/Δx) / P = x / (x + Δx)**

So your average execution as a fraction of marquee = `x / (x + Δx)`. This is always less than 1 (you always pay slippage). The size of the gap depends on `Δx / x` — your trade size as a fraction of the pool's reserves.

### Worked Example 1 — Tiny trade (negligible slippage)

Pool: 100 WETH, 300,000 USDC, P = $3,000.

You swap 0.01 WETH (i.e. Δx = 0.01, x = 100):

- Average execution / marquee = 100 / 100.01 ≈ 0.9999
- Slippage: ~0.01% of marquee
- You receive ~$30.00 - $0.003 = effectively $30

Tiny trades have effectively zero slippage on this pool depth.

### Worked Example 2 — Moderate trade (0.1% slippage)

Same pool. You swap 1 WETH (Δx = 1, x = 100):

- Average execution / marquee = 100 / 101 ≈ 0.9901
- Slippage: ~1% of marquee
- Marquee value of 1 WETH: $3,000
- You receive: 1% less = ~$2,970

This matches the worked example in `amm-mechanics-v2-and-v3.md` (you got 2,970.30 USDC for 1 WETH).

### Worked Example 3 — Large trade (10% slippage)

Same pool. You swap 10 WETH (Δx = 10, x = 100):

- Average execution / marquee = 100 / 110 ≈ 0.909
- Slippage: ~9.1% of marquee
- Marquee value: $30,000
- You receive: ~$27,272

A 10× larger swap got 9× worse execution — slippage is **convex**, growing faster than linearly with trade size.

### Worked Example 4 — Slippage on a deeper pool

Suppose the same trade hit a pool 10× deeper (1,000 WETH, 3,000,000 USDC):

- Marquee P still $3,000
- Swap 10 WETH: average execution / marquee = 1,000 / 1,010 ≈ 0.990
- Slippage: ~1% (vs ~9% on the shallower pool)

This is why deep pools matter. The same trade on a 10× deeper pool has 10× less slippage.

## The Convexity Property

Slippage scales worse than proportionally with trade size:

| Trade size as % of pool | Slippage % |
|---|---|
| 0.1% | 0.10% |
| 1% | 0.99% |
| 10% | 9.09% |
| 50% | 33.3% |
| 100% | 50% |

This convexity has two practical implications:

1. **DEX aggregators split large orders.** 1inch, Paraswap, Uniswap's Universal Router — they all route a single trade across multiple pools to minimise total slippage. Even paying gas for multiple swaps, the aggregate slippage is lower than putting everything through one pool.

2. **Large LPs absorb the convexity.** When a 10% trade happens on a pool, the LPs collectively give up 9% to slippage (which is then partially earned back via the fee on that swap). Large LPs facing a "shock" trade can lose meaningful value.

## Slippage Tolerance and MEV

When you submit a swap to an AMM, you specify a **slippage tolerance** — the worst-case execution price you're willing to accept. If the actual execution would be worse than this, the swap reverts.

For example, you might say "swap 1 WETH for at least 2,950 USDC." If the pool's state shifted between your submission and execution (because someone else's swap landed first), and the new state would only give you 2,940 USDC, your swap reverts and you pay only the gas.

This is a critical defence against **sandwich attacks**:

1. Bot sees your pending swap with slippage tolerance "minimum 2,950 USDC"
2. Bot front-runs you with a swap that pushes the price up
3. Your swap executes at the new (worse) price, but is still ≥ 2,950 USDC, so it doesn't revert
4. Bot back-runs you with a reverse swap, capturing your slippage as profit

The lower your slippage tolerance, the harder you are to sandwich (a tight tolerance reverts before the bot can extract much). But too tight a tolerance means your swap reverts even from normal market noise. The standard recommendation is 0.5%-1% slippage tolerance for routine trades.

This connects directly to MEV — see `concepts/domain-patterns/mev-and-transaction-ordering.md`.

## How This Appears in Aurix

Aurix doesn't model trader slippage explicitly. The dashboard shows the marquee price for each venue (the implied price at the current pool state for an infinitesimal trade). When the "Positive setup holding" insight surfaces a +$6 spread, that's the gross theoretical arbitrage opportunity at zero size — the moment anyone tried to capture it at meaningful size, slippage on both legs would eat into the gap.

A more sophisticated version of Aurix (e.g. as part of Vector A's backtester) would compute a "slippage curve" for each pool — what trade size would be required to extract the full spread, and what the actual net would be after slippage on both sides. This isn't currently surfaced because Aurix is observational rather than execution-grade.

The `GAS_UNITS_ESTIMATE = 220_000` in `insights.ts` is a partial proxy for this — it estimates execution cost at gas + slippage at some implicit small trade size. A "real" net-profitability calculation would require explicit modelling of optimal trade size given slippage curves on both venues, which is non-trivial.

## Common Misunderstandings

❌ **"Slippage is the AMM's fee."** Slippage and fees are different. Fees go to LPs as compensation for providing liquidity. Slippage doesn't go anywhere — it's a geometric consequence of the curve, returned to LPs over time as the pool re-balances.

❌ **"If I trade in tiny chunks, I avoid slippage."** Each tiny chunk faces tiny slippage but each pays full gas. Below a certain chunk size, the gas cost dominates and you're worse off than a single larger swap. The optimal chunk size depends on pool depth, gas price, and your total intended size.

❌ **"Slippage is the same direction as the trade."** It's worse for the trader regardless of direction. Buying or selling, you always face slippage; you always get less than the marquee price would have suggested.

❌ **"Setting slippage tolerance to 0% protects me perfectly."** A 0% tolerance means your swap reverts unless execution is exactly at the marquee price — which is essentially impossible because there's natural noise. Practical slippage tolerances are 0.5%-1% for routine trades.

## Related Files

- `concepts/core/amm-mechanics-v2-and-v3.md` — the underlying math (slippage falls out of `x*y=k` directly)
- `concepts/core/arbitrage-and-cross-venue-equilibrium.md` — how arbitrage closes price gaps and produces the slippage curves we observe
- `concepts/domain-patterns/mev-and-transaction-ordering.md` — sandwich attacks and how slippage tolerance defends against them
- `concepts/domain-patterns/gas-and-execution-costs.md` — the other major cost layer for traders
