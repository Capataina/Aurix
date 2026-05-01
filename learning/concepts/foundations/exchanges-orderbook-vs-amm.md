# Exchanges: Order Book vs AMM

## Why This Matters Here

Every venue Aurix watches is an AMM. To understand WHY DeFi went with AMMs (and what the alternative would have been), you need to know what an order book is, why it works for centralised exchanges, and what specifically about Ethereum makes order books impractical there. This file explains the two paradigms and the trade-offs that shaped the modern DeFi landscape.

## Prerequisites

- `concepts/foundations/markets-and-prices.md` (you should know what bids, asks, and spreads are)

## The Two Paradigms

Every exchange in the world uses one of two models for matching buyers and sellers:

| | Order Book | AMM (Automated Market Maker) |
|---|---|---|
| **Used by** | NYSE, NASDAQ, Coinbase, Binance, every TradFi exchange, Nyquestro | Uniswap, SushiSwap, Curve, Balancer, every major DEX |
| **Matching** | Central matching engine pairs bids and asks at crossing prices | No matching — pool of two tokens with a math formula |
| **Liquidity providers** | Professional market makers posting bids and asks | Anyone who deposits both tokens into the pool |
| **Price source** | Most recent trade | Implicit in the pool's reserve ratio |
| **Quote update cost** | Per-update message (cheap) | Per-update transaction (expensive on Ethereum) |
| **Native fit for** | High-frequency, professional venues | On-chain, gas-constrained venues |

These aren't rivals — they're solutions to different sets of constraints. Order books dominate where matching engines are practical; AMMs dominate where they're not.

## Order Book Mechanics

In an order book exchange, all buyer interest and seller interest is collected and matched centrally:

```
       BIDS (buyers)              ASKS (sellers)
   ┌──────────────────┐       ┌──────────────────┐
   │  $3,002 × 5 WETH │       │  $3,005 × 3 WETH │  ← lowest ask
   │  $3,001 × 8 WETH │       │  $3,006 × 7 WETH │
   │  $3,000 × 12 WETH│       │  $3,007 × 4 WETH │
   │  $2,999 × 6 WETH │       │  $3,008 × 9 WETH │
   └──────────────────┘       └──────────────────┘
                ↑                     ↑
        highest bid              lowest ask
        ($3,002)                  ($3,005)
                ←─── $3 spread ────→
```

A trade happens when bids and asks cross. If a new buyer comes in willing to pay $3,005, they match the lowest ask of $3,005 × 3 WETH and execute that trade. The ask gets removed (or partially filled, if the buyer wanted less than 3 WETH). The matching engine processes this in microseconds.

Three properties of order books that make them powerful:

1. **Price discovery is precise.** The bid/ask gives you exact information about marginal supply and demand at every price level.
2. **Liquidity providers can react.** A market maker observing volatility can pull their quotes, adjust their spread, or change their sizes — all with cheap message updates.
3. **Matching is fast.** Modern exchanges process millions of orders per second.

Order books are how every traditional financial exchange operates. They're also how Caner's Nyquestro project is structured — a from-scratch matching engine in Rust with a lock-free order book. If you want to understand order books deeply, the Nyquestro README is a good cross-reference.

## Why Order Books Failed in DeFi

Several teams tried to build order-book DEXes in the early days of Ethereum. The model worked technically but failed economically. The reason: **gas**.

Every order placement on Ethereum is a transaction. Every cancellation is a transaction. Every quote update is a transaction. A market maker who naturally updates their quote 10 times per second (normal in TradFi) would pay thousands of dollars per minute just to participate. Worse, the time between submitting a quote and it being included in a block is up to 12 seconds — long enough that the market maker's quote is stale by the time it's live.

The economics simply don't work. Professional market makers stayed on CEXes (Coinbase, Binance) where their per-update cost is a fraction of a cent. On-chain order books became thin to the point of unusability.

This is the gap AMMs solved.

## AMM Mechanics

An AMM replaces the order book with a **pool** of two tokens and a **pricing formula**:

```
   ┌─────────────────────────────────────┐
   │     Uniswap V2 WETH/USDC pool       │
   │                                     │
   │     ┌─────────┐    ┌──────────┐     │
   │     │ 100 WETH│    │ 300k USDC│     │
   │     └─────────┘    └──────────┘     │
   │                                     │
   │     formula: x × y = k              │
   │     k = 100 × 300,000 = 30,000,000 │
   │     implied price: $3,000 / WETH   │
   │                                     │
   │     fee: 0.30% per swap             │
   └─────────────────────────────────────┘
```

There's no order book. There's no matching engine. There's no "buyer" and "seller" matched as counterparties. Instead:

- The **pool** holds two tokens
- The **formula** determines pricing as a function of the reserves
- A **swap** is anyone putting one token in and taking the other out, with the formula determining the exchange ratio
- **LPs** deposit both tokens into the pool to earn fees from every swap

Critical observation: in an AMM, **the price isn't quoted by anyone**. There's no market maker saying "I'll sell WETH for $3,000." There's just a pool with 100 WETH and 300,000 USDC, and the implied price (what a swap would actually execute at, for some size) is determined by the formula.

The genius of the AMM model is that it's **passive**. LPs deposit capital once. The pool handles all subsequent pricing automatically. No per-quote gas. No matching engine to operate. The whole exchange runs as a single immutable smart contract that anyone can interact with.

## What AMMs Trade Off

AMMs win on gas economics. They lose on:

1. **Pricing flexibility.** AMM prices are mechanical — they follow the formula. They can't react to information the way a market maker can. A market maker who sees volatility incoming can widen their spread or pull their quotes. An AMM just keeps pricing according to the formula, which means LPs absorb the entire cost of adverse selection.

2. **Capital efficiency.** A V2 pool spreads capital across all possible prices ($0 to ∞). Most of that capital is never used (nobody trades at $30,000 per WETH or $3 per WETH). V3 partially fixes this with concentrated liquidity, but it's still less efficient than an order book where market makers can concentrate capital exactly where the action is.

3. **Information leakage.** Every AMM swap is on-chain and visible. A large swap reveals trader intent. Order book exchanges have private order types (iceberg orders, hidden orders) that AMMs can't match. This is one driver of MEV — the public, visible nature of AMM swaps makes them inherently sandwichable.

4. **Maximum trade size.** A V2 pool with 100 WETH can't execute a 50 WETH trade with reasonable slippage — your trade itself moves the price too much. Order book exchanges with deeper liquidity can absorb larger trades without comparable price impact.

These trade-offs explain why **professional traders typically use CEXes for execution** and **AMMs are used by retail or for tokens that aren't listed on CEXes**.

## The Hybrid Future

Several projects are trying to bring order book mechanics back to DeFi via L2 (Layer 2 scaling) where gas is cheap enough to make order book updates affordable:

- **dYdX v4** runs an order book on its own appchain
- **Hyperliquid** runs an order book on its own L1
- **Vertex** combines order book and AMM liquidity
- **Various L2 projects** (Arbitrum, Optimism) host hybrid order book DEXes

These exist outside Aurix's current scope (Ethereum mainnet AMMs only) but are worth knowing as the trajectory of the field. The "AMM dominates DeFi" assumption is being eroded as scaling solutions make order books economically viable again.

## How This Appears in Aurix

Aurix's four venues are all AMMs:

- **Uniswap V3 (5bps and 30bps)** — concentrated liquidity AMMs, formula is more complex than V2 but conceptually still pool-based pricing
- **Uniswap V2** — the canonical `x * y = k` constant product AMM
- **SushiSwap** — a fork of Uniswap V2 with the same math, different liquidity, different governance

Aurix's `dex/uniswap_v3.rs` reads `slot0()` (the V3 pool's primary state function), decodes `sqrtPriceX96`, and derives the implied price. There's no order book to read because there's no order book to read — the price IS the pool state, derived via formula.

If Aurix watched a CEX (e.g. Coinbase WETH/USDC), the architecture would be fundamentally different: it would subscribe to the CEX's WebSocket order book feed and read the best bid/ask, rather than reading on-chain pool state and computing an implied price.

## Common Misunderstandings

❌ **"AMMs are simpler than order books."** AMMs *look* simpler from a system architecture view (no matching engine!) but the math is more complex (V3 tick math is genuinely difficult), and the LP economics introduce impermanent loss as a new concept that doesn't exist in order books.

❌ **"Order books are obsolete."** Order books dominate every TradFi exchange and every major CEX. They're "obsolete in DeFi" only in the specific sense that on-chain Ethereum gas costs make them economically infeasible — that's a constraint of the venue, not a property of the model.

❌ **"AMMs always have worse pricing than order books."** AMMs and order books at scale produce comparable pricing (arbitrage keeps them aligned). The differences are in capital efficiency, slippage curves, and the costs LPs vs market makers face — not in the marquee price.

❌ **"AMMs are decentralised, order books are centralised."** This conflates exchange model with operational structure. dYdX runs an order book on its own decentralised L2. There are decentralised order book DEXes. The order-book-vs-AMM distinction is about *matching mechanism*, not about who operates the venue.

## Related Files

- `concepts/core/amm-mechanics-v2-and-v3.md` — how AMM math actually works
- `concepts/core/traders-and-slippage.md` — slippage in AMM context
- `concepts/foundations/markets-and-prices.md` — the universal market primitive
- `materials/amm-foundational-resources.md` — Uniswap V2 and V3 whitepapers
