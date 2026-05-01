# MEV and Transaction Ordering

## Why This Matters Here

MEV (Maximal Extractable Value) is one of the most consequential phenomena in modern Ethereum. It explains why arbitrage on the public mempool doesn't work for retail, why pro traders use Flashbots, why some swaps cost users hundreds of dollars in invisible "fees," and why Vector B (mempool MEV detector) would be a strong portfolio piece. This file builds the MEV mental model from scratch.

## Prerequisites

- `concepts/core/arbitrage-and-cross-venue-equilibrium.md` (MEV is partially the institutionalised version of arbitrage)
- `concepts/domain-patterns/gas-and-execution-costs.md` (you should understand priority fees and the EIP-1559 mechanics)

## The Core Insight

When a block builder constructs the next Ethereum block, they choose:

1. **Which transactions to include** from the mempool (out of potentially thousands waiting)
2. **What order to put them in**
3. **What transactions to exclude** (intentionally not include, even if they pay enough gas)

Different choices produce different financial outcomes. The maximum profit a builder can extract by choosing optimal ordering, inclusion, and exclusion is called **MEV**.

This is not a bug. It's a structural property of any system where (a) transactions are batched and (b) ordering matters. Ethereum has both: blocks contain ~150-300 transactions, and DeFi operations are highly order-sensitive (a swap's output depends on the pool's state at the moment of execution, which depends on what other transactions came before in the same block).

## A Worked Example — The Sandwich Attack

This is the canonical MEV strategy. The mechanics:

**Setup:** Alice wants to swap 50 WETH for USDC on Uniswap V3. The pool's current price is $3,000. Alice expects to receive ~$150,000 USDC. To allow for some slippage, she submits with `amount_out_min = 145,000 USDC` (about 3% slippage tolerance).

**The bot watches the mempool.** Alice's pending transaction is visible to anyone watching the public mempool (which is most arbitrage bots).

**The bot computes:** "If Alice's swap executes against the current pool, the price will move from $3,000 to $2,985 (because she's adding 50 WETH to the pool). She'll get ~$148,500 USDC. But if I push the price up first to $3,015 by buying WETH myself, then her swap will move it from $3,015 to $3,000, and she'll get only ~$145,500 USDC. The $3,000 USDC she 'gives up' to me, I capture by selling my WETH back at the price her swap created."

**Bot's three transactions:**

1. **Front-run**: Bot buys ~10 WETH from the same pool. Pool's WETH reserves drop, price moves up to $3,015.
2. **Alice's transaction executes**: at the new price. She gets $145,500 USDC instead of the $148,500 she would have gotten if she'd executed first.
3. **Back-run**: Bot sells the 10 WETH it just bought, capturing the price boost Alice's swap created.

**Net result:**
- Alice received $3,000 less than she would have without the bot
- Bot profit ≈ $3,000 − (gas cost of two bot transactions)
- Pool LPs earn fees on all three swaps (a tiny consolation prize)

The bot doesn't even need to risk anything — both the front-run and back-run are computed conditional on Alice's swap actually executing. If anything fails, the bot's bundle reverts and the bot pays only the gas.

This is MEV. The bot extracted $3,000 from Alice purely by virtue of being able to choose the order: front-run → Alice → back-run.

## How Sandwich Attacks Are Submitted

The bot doesn't just submit three normal transactions and hope for the right ordering — that would require beating other bots in the public mempool gas auction (and risk Alice's tx getting included before the front-run). Instead, the bot submits all three as a **bundle** via a private channel like Flashbots.

A Flashbots bundle is an atomic group of transactions submitted to block builders directly (bypassing the public mempool) with the guarantee that they'll be included in the specified order or not at all. The block builder is paid via a tip from the bundle's profit.

This means:
- The bundle's three transactions are guaranteed to land in the right order
- The bundle is invisible to other bots (no front-running the front-run)
- If the simulation shows a loss, the bundle isn't submitted

Sandwich attacks are highly profitable for bots because the worst case is a small gas loss and the best case is multi-thousand-dollar profit per opportunity.

## Other MEV Categories

Sandwich attacks are the most famous but not the only kind:

### Frontrunning (without back-running)

A bot sees an opportunity in the mempool — say, an oracle update that will trigger a profitable liquidation — and submits its own transaction with a higher priority fee to land first. No back-run; just being first to the opportunity.

### Just-in-Time (JIT) Liquidity

A sophisticated bot sees a large pending swap on Uniswap V3. The bot mints a tightly-concentrated LP position right before the swap (at the current price), captures most of the swap's fee (because the bot is now the dominant in-range liquidity), then burns the position immediately after.

This is "fee extraction" without taking IL risk — the bot's position is in the pool for only one block (~12 seconds). The legitimate LPs who were going to earn that fee instead get diluted.

### Liquidation Sniping

When a borrower's collateral drops below the liquidation threshold on Aave/Compound/Maker, anyone can call the liquidation function and claim the liquidation bonus (typically 5-10% of the position). Bots monitor borrower health continuously and race to be first when a position becomes liquidatable.

### Arbitrage as MEV

The arbitrage we discussed in `arbitrage-and-cross-venue-equilibrium.md` is itself a form of MEV — the bot extracts value by being the first to close a cross-venue gap. The "value extracted" is the gross spread minus the bot's costs.

### Backrunning (alone)

A bot sees a transaction that creates an opportunity (e.g. a large swap that moves a price) and submits its own transaction immediately after to capture the resulting opportunity. Less invasive than sandwich attacks because no front-run is involved — the bot only profits from cleaning up after.

## Builder/Searcher/Validator Architecture

MEV has produced a sophisticated ecosystem post-merge:

```
                ┌──────────────────────────────┐
                │  SEARCHERS                   │
                │  · Look for opportunities    │
                │  · Construct bundles         │
                │  · Bid for inclusion         │
                └──────────────┬───────────────┘
                               │
                               v
                ┌──────────────────────────────┐
                │  BUILDERS                    │
                │  · Receive bundles           │
                │  · Construct full blocks     │
                │  · Maximise total value      │
                └──────────────┬───────────────┘
                               │
                               v
                ┌──────────────────────────────┐
                │  RELAYS                      │
                │  · Forward blocks            │
                │  · Validate consensus rules  │
                └──────────────┬───────────────┘
                               │
                               v
                ┌──────────────────────────────┐
                │  VALIDATORS (proposers)      │
                │  · Choose which block to use │
                │  · Sign and propose          │
                └──────────────────────────────┘
```

- **Searchers** find MEV opportunities (sandwich, JIT, arb, liquidations) and submit bundles
- **Builders** assemble blocks from many searchers' bundles plus regular mempool transactions, optimising for total profit
- **Relays** (e.g. Flashbots Relay) connect builders to validators
- **Validators** are the proposers who actually attest to the block

Each layer extracts value. Searchers earn from the opportunities they find. Builders earn from being able to construct profitable blocks (they keep some value, pass some to validators). Validators earn from priority fees and the value passed by builders.

## Estimated Scale

Rough estimates for Ethereum mainnet MEV (post-merge):

- ~$300-700 million per year in extracted MEV
- ~$100-300 million per year in user "harm" (from sandwich attacks specifically)
- Top-10 MEV searchers do hundreds of thousands of transactions per month
- The biggest single MEV extraction in history was over $30 million in a single block

The scale is not small. MEV has become a substantial revenue stream for a small set of sophisticated actors.

## How This Affects Aurix

Aurix doesn't currently watch the mempool, so it doesn't see MEV directly. But:

1. **Most arbitrage opportunities Aurix shows are NOT actionable** because pro arbitrage bots (which DO watch the mempool, via private orderflow) capture the profitable ones in milliseconds. What Aurix sees on the public mempool is the leftovers — gaps too small or too risky for pros to bother with.

2. **The "Positive setup holding" insight** is partially MEV-exposed in reverse: if a real, large gap appeared, MEV bots would close it via private bundles. The fact that Aurix's surfaced gaps persist for many seconds is itself proof that they're not worth bot attention.

3. **Vector B (MEV detector)** is the dedicated piece that would detect MEV in the mempool — sandwich attempts, JIT liquidity, liquidations — and surface them with extractable-value calculations. Aurix would still never execute (read-only by design), just observe the MEV ecosystem.

## Common Misunderstandings

❌ **"MEV is theft."** It's not theft in the legal sense — every transaction is voluntarily submitted with a slippage tolerance, and the bot exploits the tolerance the user explicitly allowed. But it IS a form of value extraction enabled by information asymmetry. "Predatory" or "extractive" is more accurate than "theft."

❌ **"You can avoid MEV by using a tight slippage tolerance."** Tighter slippage tolerance reduces sandwich profit, but too tight a tolerance means your swap reverts from normal market noise. Practical defence: use private orderflow (Flashbots, MEV-Share) for any swap where the slippage tolerance × trade size is meaningful (>$10K).

❌ **"MEV will go away with proof-of-stake / new protocols."** MEV is structural to any system with batched ordered transactions. Proof-of-stake didn't eliminate it; it just changed the actors. Even on L2s and alt-L1s with shorter block times, MEV exists. The only way to eliminate it is to encrypt orders before they enter the mempool (an active research area: "encrypted mempools," "PBS auctions," "fair ordering protocols").

❌ **"MEV is bad for Ethereum."** It's complicated. MEV extraction is bad for the specific users being extracted from. But the existence of MEV creates a market for sophisticated infrastructure (Flashbots, MEV-Boost) that has improved Ethereum's overall efficiency. Some MEV (like arbitrage) is *necessary* — without it, prices wouldn't stay aligned across venues. The discourse is contested.

❌ **"Aurix could detect and warn about MEV."** Vector B's plan is exactly this. Currently Aurix doesn't have mempool visibility, so it can't see MEV in flight. After Vector B, it could surface "this swap pattern looks like a sandwich victim" or "this opportunity is being targeted by JIT liquidity bots."

## Related Files

- `concepts/domain-patterns/the-mempool-public-vs-private.md` — where MEV opportunities live
- `concepts/advanced/mempool-mev-detection-mechanics.md` — the technical depth for Vector B
- `concepts/core/arbitrage-and-cross-venue-equilibrium.md` — arbitrage as a form of MEV
- `concepts/core/traders-and-slippage.md` — slippage tolerance as the mechanism that enables/limits sandwich attacks
- `materials/mev-resources.md` — Flashbots, libMEV, Eigenphi for going deeper
- `context/plans/vector-b-mev-detector.md` — the implementation plan for the detector
