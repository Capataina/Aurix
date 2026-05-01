# Arbitrage and Cross-Venue Equilibrium

## Why This Matters Here

Arbitrage is what Aurix is fundamentally about. The dashboard exists to observe cross-venue price discrepancies, surface them with persistence and gas-adjusted profitability, and help the user understand the dynamics of the multi-venue DEX market. This file explains what arbitrage is, the mechanism by which it closes gaps, and crucially — why so many "free money" gaps Aurix shows are actually unprofitable to act on.

## Prerequisites

- `concepts/core/amm-mechanics-v2-and-v3.md` (you should understand how AMM prices form)
- `concepts/core/traders-and-slippage.md` (you should understand slippage)

## The Mechanism

Suppose Pool A and Pool B are both WETH/USDC pools. At the same moment:
- Pool A has reserves giving an implied price of $3,000 per WETH
- Pool B has reserves giving an implied price of $3,008 per WETH

WETH is "cheaper" on Pool A and "richer" on Pool B. An arbitrageur:

1. **Buys WETH cheap on A**: swaps USDC for WETH on Pool A. This ADDS USDC to A and REMOVES WETH, pushing A's price UP.
2. **Sells WETH expensive on B**: swaps WETH for USDC on Pool B. This ADDS WETH to B and REMOVES USDC, pushing B's price DOWN.
3. **Pockets the difference**: the WETH bought cheap on A is sold expensive on B; the gap is the gross profit.

Both swaps happen in a **single atomic transaction** — either both succeed or both revert. This is critical: an arbitrageur cannot afford to do leg 1 and have leg 2 fail (they'd be left holding WETH at a worse price than expected).

After the arbitrage:
- A's price is HIGHER than before (less WETH, more USDC)
- B's price is LOWER than before (more WETH, less USDC)
- The gap between A and B has SHRUNK

The very act of arbitraging closes the gap that triggered it. This is the mechanism by which prices stay aligned across venues without any central coordinator.

## The Equilibrium

In the absence of execution costs, arbitrageurs would close every gap to exactly zero (any gap = profit). But execution costs exist:

- **Gas** for both swaps (~$10-$50 in real money for a typical gas price)
- **Slippage** on both legs (your own swaps move the pool prices)
- **MEV competition** — other arbitrageurs are trying to capture the same opportunity

The minimum profitable gap = gross spread > (gas + slippage + competition risk premium). Below this, no arbitrageur enters. The result: prices across venues stay within a "no-arbitrage band" defined by execution costs.

This band isn't a hard line — it's stochastic. Small gaps (under the cost floor) persist for many seconds because nobody profits from closing them. Large gaps (well above the cost floor) vanish in milliseconds because every arb bot races to capture them.

## What Aurix Sees

Most of what Aurix's dashboard shows is **the dead zone** — gaps that are visible but uneconomical to act on. When you see "Sushi → V3-5bps positive setup holding 12 samples · est. +$6 route," that's:

- A real spread between SushiSwap (currently cheaper) and V3-5bps (currently richer)
- That has stayed in the same direction for 12 consecutive 1-second polls
- Worth ~$6 gross at the assumed trade size, before any execution cost

For this to be profitable, you'd need:
- Gas cost of both swaps < $6 — possible only at very low gas (single-digit gwei)
- Slippage on both legs at the chosen size < the remainder
- No bot beating you to it (which is unlikely if it's been visible for 12 seconds)

The fact that the gap is still there after 12 seconds is itself proof that it's not profitable — otherwise a bot would have captured it long ago.

## The "Why Doesn't It Vanish?" Question

A naive observer might ask: if the spread is real, why don't the arbitrageurs close it?

Three reasons:

### 1. The gap is below the cost floor

Most spreads Aurix shows are sub-$10. With typical gas costs of $15-$50 per swap and meaningful slippage on any non-trivial size, a $6 spread is just below break-even for retail-tier arbitrageurs. The bots ignore it because they'd lose money executing it.

### 2. The gap is in a difficult-to-capture pool

Some pools have shallow liquidity that means even a small arbitrage attempt would suffer high slippage. The Sushi pool, in particular, is shallower than V3 — arbitrage from V3 to Sushi (or vice versa) at any meaningful size faces the slippage curve on the Sushi side disproportionately.

### 3. Pro arbitrageurs use private orderflow

The professional arbitrage market doesn't use the public mempool. Arbitrage bots run colocated near block builders, submit via Flashbots or similar private channels, and bid for inclusion priority. Their cost structure is different from retail's — they pay less in gas competition, they execute faster, and they have specialised infrastructure. The opportunities they find disappear in milliseconds. The opportunities Aurix sees on the public mempool are leftovers.

## What an "Actionable" Spread Looks Like

For a retail-tier arbitrage to be theoretically profitable, you'd typically need:

- Gross spread > $40-$100 at meaningful size
- Both venues with sufficient depth that your trade doesn't move the price much
- Low gas environment (under 30 gwei)
- A pool pair the major bots aren't actively monitoring (less common for popular pairs)

These conditions occur but are rare for popular pairs like WETH/USDC. They're more common in:
- Newly-listed token pools (bots haven't deployed coverage yet)
- Long-tail pairs (low volume, wider price drift)
- During chain congestion (when normal arb bots are gas-priced out)

Aurix watches WETH/USDC specifically because it's one of the MOST competed-for pairs — which is intellectually interesting (you're observing the most efficient arbitrage market in DeFi) but means there's almost never a retail-actionable opportunity.

## How This Appears in Aurix

The arbitrage observation logic lives in `src/features/arbitrage/insights.ts`. Specifically:

```typescript
const positiveRunLength = trailingRunLength(
  derivedHistory,
  (sample) => sample.gasAdjustedUsd > 0,
);
```

This counts how many consecutive recent samples have a positive gas-adjusted spread (meaning: spread × WETH amount > assumed gas cost). The "Positive setup holding" insight fires when `positiveRunLength >= PERSISTENCE_WINDOW` (4 samples).

The key word is "persistence." A single positive sample is noise. Four consecutive positive samples in the same direction (cheapest-to-richest route stable) suggests something structural — but the very fact that it's persisting for 4+ seconds means no bot is capturing it, which means it's almost certainly not actually profitable for execution.

The dashboard surfaces this honestly: the insight body says "stayed positive for [N] samples, with gas still leaving an estimated [X] of room" — leaving "of room" deliberately ambiguous. It's gross theoretical room before any other costs.

## Common Misunderstandings

❌ **"If Aurix shows a +$6 spread, I could make $6 by arbitraging it."** No. The +$6 is gross at the marquee price, before gas (~$15-$50 for a round-trip), slippage on both legs, and competition from bots that have been watching this exact spread for the last 12 seconds. Net is almost always negative.

❌ **"The arbitrage market is broken because gaps persist."** The gaps persist precisely because the market is in equilibrium — any gap small enough to ignore stays open. Closing those gaps would lose money. The market is working as designed; the equilibrium just isn't at zero spread.

❌ **"More venues = more arbitrage opportunities for me."** More venues = more potential gaps, but they're all subject to the same cost floor. Adding a 10th venue when you have 9 doesn't materially expand the actionable opportunity surface — it just gives you more visibility into a bigger dead zone.

❌ **"Arbitrage closes gaps perfectly."** Arbitrage closes gaps to the cost floor. Below that, gaps persist indefinitely. The "law of one price" holds only modulo execution costs.

❌ **"Aurix's data could be turned into a trading bot."** Aurix's data is too slow (1 Hz polling, 100-sample window) and too late (public RPC, no mempool watching) for any competitive arbitrage. To trade real money on this signal you'd need: WebSocket subscriptions instead of polling, mempool watching, private orderflow (Flashbots or similar), and capital deployed to both sides of every venue pair. None of that is in Aurix's scope by design.

## How Arbitrage Differs Across Venue Types

Aurix watches four AMMs all on Ethereum mainnet. Real arbitrage opportunities span more venue types:

| Arbitrage type | Description | Difficulty |
|---|---|---|
| **DEX-DEX** (what Aurix watches) | Same pair across multiple DEXes on the same chain | High competition, narrow margins |
| **CEX-DEX** | A CEX (Coinbase, Binance) priced differently than a DEX | Slow (CEX withdrawals/deposits), some opportunities exist |
| **Cross-chain** | Same asset on different chains (Ethereum WETH vs Arbitrum WETH) | Slow (bridges add latency), specialised infrastructure |
| **Triangular** | A→B→C→A across pairs where the cycle is profitable | Combinatorial explosion of opportunities, mostly gone |
| **Statistical arbitrage** | Pairs of correlated assets diverging from their normal relationship | Quant-fund territory, requires modelling |

DEX-DEX is the most-competed-for and has the narrowest margins. Aurix is watching the toughest segment of the arbitrage market.

## Related Files

- `concepts/core/amm-mechanics-v2-and-v3.md` — the math that produces venue-specific prices
- `concepts/core/traders-and-slippage.md` — slippage on each leg of the arb
- `concepts/domain-patterns/gas-and-execution-costs.md` — the dominant cost layer
- `concepts/domain-patterns/mev-and-transaction-ordering.md` — why pros use private orderflow
- `project/systems/insight-engine-anatomy.md` — how Aurix specifically computes and surfaces these signals
