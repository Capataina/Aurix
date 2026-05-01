# Foundations Path

## Who This Path Is For

You're a software engineer (any background) who is new to DeFi, Ethereum, AMMs, and crypto markets. You may have heard terms like "swap," "liquidity pool," and "gas fees" but you don't have a working mental model of what they actually mean or how the pieces fit together.

By the end of this path you'll be able to read the Aurix codebase, understand what the dashboard is showing, and follow conversations about DeFi without losing the thread.

## What This Path Assumes

- Comfort reading Rust and TypeScript (you don't need to write either fluently)
- Comfort with basic algebra (you'll see formulas, but nothing beyond high-school level in this path)
- No prior crypto knowledge required

## Recommended Sequence

### Stage 1 — What money even is in this domain

- [ ] `concepts/foundations/tokens-and-the-defi-money-stack.md`

You'll learn what an ERC-20 token is, why WETH and USDC are the assets Aurix watches, and the difference between ETH and WETH (the latter is wrapped because ETH itself isn't ERC-20-compatible). About 15 minutes.

### Stage 2 — How markets work, conceptually

- [ ] `concepts/foundations/markets-and-prices.md`
- [ ] `concepts/foundations/exchanges-orderbook-vs-amm.md`

You'll learn what a market does fundamentally (matches buyers and sellers, produces a price, provides liquidity), the two paradigms for running an exchange (order book vs AMM), and why DeFi went with AMMs. About 30 minutes.

### Stage 3 — How AMMs actually compute prices

- [ ] `concepts/core/amm-mechanics-v2-and-v3.md`

The single most important file in this path. You'll work through the constant-product formula `x * y = k` with real numbers, see how slippage emerges from the math itself, and get an intuition for what concentrated liquidity (V3) adds on top. About 45 minutes — read slowly, work the example.

### Stage 4 — The cast of characters

- [ ] `concepts/core/liquidity-providers-and-impermanent-loss.md`
- [ ] `concepts/core/traders-and-slippage.md`
- [ ] `concepts/core/arbitrage-and-cross-venue-equilibrium.md`

Three short files covering the three roles in any AMM market: LPs (who provide capital and earn fees), traders (who pay slippage), and arbitrageurs (who close price gaps across venues). The third file is where Aurix itself enters the picture. About 60 minutes total.

### Stage 5 — The cost layer

- [ ] `concepts/domain-patterns/gas-and-execution-costs.md`

You'll learn what gas is, how it's denominated (gwei, wei, ETH), and why this is the reason most "free money" arbitrage opportunities are actually unprofitable. About 20 minutes.

### Stage 6 — The dark side

- [ ] `concepts/domain-patterns/mev-and-transaction-ordering.md`
- [ ] `concepts/domain-patterns/the-mempool-public-vs-private.md`

MEV (Maximal Extractable Value) and the mempool — the pieces that make Ethereum's economy more complex than "users send transactions, transactions execute." You'll learn what a sandwich attack is, why Flashbots exists, and how this relates to Aurix's read-only design. About 30 minutes.

### Stage 7 — Tying it back to Aurix

- [ ] `project/systems/what-aurix-observes.md`

The synthesis file. Now that you know what tokens, markets, AMMs, LPs, slippage, arbitrage, gas, and MEV all are, this file walks through what Aurix's 1 Hz polling loop is actually doing in those terms. About 20 minutes.

## What You Should Understand By The End

- What a token is, what WETH and USDC specifically are, and why Aurix uses those
- The difference between an order-book exchange and an AMM
- How `x * y = k` produces a price and why slippage is built into the math
- What an LP does, what fees they earn, and what impermanent loss is
- Why arbitrage exists and why most of what Aurix shows isn't profitable to act on
- What gas is and why it's the dominant cost layer for retail arbitrage
- What MEV is, what a sandwich attack looks like, and why the mempool matters
- What Aurix's polling loop actually does in domain terms

## What To Do Next

| Goal | Next path |
|---|---|
| Go deeper on the math | `domain-theory-path.md` |
| Understand the codebase | `project-systems-path.md` |
| Prepare for an interview about Aurix | `interview-fluency-path.md` |
| Start shipping a vector | `vector-prep-path.md` |
