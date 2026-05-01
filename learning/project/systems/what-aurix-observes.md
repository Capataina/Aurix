# What Aurix Observes

## What This System Does

Aurix observes **four parallel WETH/USDC markets** on Ethereum mainnet, once per second, and surfaces interpretive insights about cross-venue price relationships. This is the synthesis file that connects domain theory (markets, AMMs, arbitrage, MEV) to the specific code that does the observing.

## Where It Fits

This file is the bridge between `concepts/` (domain theory) and `context/systems/` (implementation truth). After reading this, you should be able to look at the dashboard and translate every visible element into terms from the concept files.

## What's Being Observed (in domain terms)

Aurix watches:

| Quantity | What it represents (domain) | Where it comes from (code) |
|---|---|---|
| 4 venue prices | Implied marquee price for WETH/USDC at each AMM pool | `dex/uniswap_v3.rs` for V3 pools, `dex/uniswap_v2.rs` for V2-style pools |
| 1 gas price | Current Ethereum mainnet gas price in gwei | `ethereum/client.rs::gas_price_gwei()` |
| 6 pairwise spreads | The cross-venue gaps that arbitrageurs would observe | `insights.ts` derives these from venues |
| Persistence (in samples) | How long the cheapest-to-richest direction has held | `insights.ts::trailingRunLength()` |
| Gas-adjusted profitability | The arbitrage spread minus assumed gas cost | `insights.ts::deriveSample()` |

These are observations of market microstructure in real time.

## What's NOT Being Observed

To set the boundary clearly, Aurix does NOT observe:

- The Ethereum mempool (no pending tx visibility) — Vector B's territory
- LP-side activity (no minting/burning of LP positions) — Vector A's territory
- Other token pairs (only WETH/USDC) — Gap 3
- Other chains (only Ethereum mainnet) — out of scope
- Order books (no CEX or order-book DEX data)
- Wallet-level activity (no specific address tracking) — Tab 3 territory
- Historical data (no persistence) — Gap 1

What Aurix observes is *the right-now state of four AMM pools and the gas price*. Everything else is derived from that observation plus a 100-sample rolling window.

## How to Read the Dashboard in Domain Terms

When you see "Uniswap V3 5bps · $3,047.23":
- Translation: "The current implied marquee price of WETH/USDC at the V3 0.05% fee pool, derived from `sqrtPriceX96` in that pool's `slot0()` state, is $3,047.23 per WETH."
- Domain meaning: this is the price you'd pay for an infinitesimally small WETH purchase on this venue right now. A larger purchase would face slippage.

When you see "Spread: $7.42":
- Translation: "The gap between the highest and lowest venue prices in this tick is $7.42."
- Domain meaning: arbitrage opportunity equal to $7.42 per WETH gross, before gas, slippage on both legs, and competition from MEV bots.

When you see "Positive setup holding for 6 samples · est. +$2.40":
- Translation: "The cheapest-to-richest route has stayed positive for 6 consecutive 1-second polls. Currently the spread minus assumed gas cost (`220,000 × gas_price_gwei × WETH_price / 10^9`) is +$2.40 in theoretical room."
- Domain meaning: a real spread direction has persisted, and at the current gas price the gross-minus-gas math is positive — but this doesn't account for slippage at meaningful trade sizes, MEV bot competition, or the fact that "+$2.40 of room" is very thin margin that almost certainly evaporates in execution.

When you see "Venue order: SushiSwap is currently richest, V3-30bps is cheapest":
- Translation: "Of the four venues this tick, SushiSwap quotes the highest WETH price; V3-30bps the lowest."
- Domain meaning: the implicit arbitrage direction would be buy on V3-30bps, sell on Sushi.

When you see "Spread regime: elevated above baseline":
- Translation: "The current spread is at least 15% above the recent 20-sample average."
- Domain meaning: you're in a higher-than-typical-recent disagreement regime, possibly because of volatility, possibly because of an arb bot just executed and pushed prices apart.

## The Severity Hierarchy

The insight engine surfaces observations at four severity levels (from `insights.ts`):

| Severity | When it fires | What it means in domain terms |
|---|---|---|
| `info` | Default state | Normal market activity, nothing notable |
| `watch` | Spread elevated OR positive gas-adjusted spread | Worth keeping an eye on but not actionable |
| `notable` | Persistent elevated spread (≥4 consecutive samples above 1.15× baseline) | Pattern worth thinking about |
| `actionable` | Positive gas-adjusted spread persisting ≥4 consecutive samples | Theoretically actionable IF you ignore everything else |

The "actionable" label is interpretive, not a recommendation. The dashboard does not say "trade now"; it says "this configuration would be theoretically profitable if you ignore slippage, MEV, and the fact that bots have been watching this for 4 seconds and not acted on it."

## What Aurix Tells You (Truthfully)

Aurix is honest about its limits. The dashboard:

- Shows real prices from real pools at real timestamps
- Computes real spreads with real gas-adjusted estimates
- Surfaces real persistence patterns

The dashboard does NOT:

- Recommend trades
- Account for slippage at meaningful trade sizes
- See MEV happening privately
- Predict future spreads
- Consider competition with bots

This honesty is intentional. Aurix is an analytics surface, not a trading tool. The read-only design (see `project/decisions/read-only-by-design.md`) is the architectural enforcement of this honesty.

## What Aurix Tells You (Implicitly)

By observing what's observable, Aurix shows you the **structure of the on-chain DEX market**:

- How efficient (or inefficient) AMM price alignment is across venues
- What the typical magnitude of dead-zone arbitrage gaps looks like
- How spread regimes shift across the day (US open, Asia open, weekend lull)
- How gas regimes affect what's theoretically actionable
- How venues differ in their typical pricing behaviour (V3 5bps tracks the "true" price most tightly because it has the deepest liquidity; V2 and Sushi drift further)

This is **market microstructure observation in real time**. The educational value is in seeing the dynamics, not in capturing any specific opportunity.

## How a Hiring Manager Should Read This

If a hiring manager looks at Aurix and asks "what does it actually do?", the right answer is layered:

**One sentence**: It's a 1 Hz cross-DEX price scanner that computes gas-adjusted arbitrage signals across four Ethereum mainnet WETH/USDC pools.

**One paragraph**: Aurix observes four AMM venues (Uniswap V3 at two fee tiers, Uniswap V2, and SushiSwap) once per second, decoding pool state via raw JSON-RPC calls (no ethers-rs — handcrafted ABI encoding) into normalised `PriceSnapshot` structs. A TypeScript insight engine derives interpretive observations: cross-venue spreads, persistence of arbitrage direction, gas-adjusted profitability, and severity-graded notifications. The architecture is read-only by design — Aurix never executes — and local-first — no data leaves the user's machine.

**The technical depth available behind the project**: V3 sqrtPriceX96 decoding via num-bigint, V2 reserve-ratio derivation with decimal-aware scaling, async Rust with tokio::join!, hand-rolled SVG charting, the rule-based insight engine with rolling-window persistence detection. Plus three vector plans (V3 LP backtester, MEV mempool detector, ML arbitrage-survival classifier) authored as natural extensions.

If they ask "but does it actually make money?", the right answer is: by design, no — Aurix is an analytics surface and a learning vehicle, not a trading tool. The hiring signal is the engineering, not the trading P&L.

## Related Files

- `concepts/core/arbitrage-and-cross-venue-equilibrium.md` — the domain theory of what Aurix observes
- `concepts/core/amm-mechanics-v2-and-v3.md` — the math behind the prices
- `project/architecture/the-1hz-loadsnapshot-tick.md` — the implementation of the polling loop
- `project/systems/insight-engine-anatomy.md` — how the insights are derived
- `context/systems/arbitrage-market-data.md` — implementation truth
- `context/systems/arbitrage-analytics.md` — implementation truth for the analytics layer
