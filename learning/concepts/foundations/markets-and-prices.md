# Markets and Prices

## Why This Matters Here

Aurix's whole purpose is to observe a market. Before you can understand what it's observing, you need a working mental model of what a market actually IS — not in any specific technology (order books, AMMs, etc.) but as a universal economic primitive. This file builds that model and then connects it to Aurix's specific case.

## Prerequisites

- `concepts/foundations/tokens-and-the-defi-money-stack.md` (you should know what WETH and USDC are)

## What a Market Does, Fundamentally

Strip away the technology — order books, AMMs, brokers, exchanges, dark pools, whatever. A market does three things:

1. **Matches buyers with sellers** of the same asset
2. **Produces a price** through the act of matching
3. **Provides liquidity** — the ability for someone to trade *now*, in whatever size they want, without waiting

That's it. Every market — the NYSE, your local farmer's market, eBay, OpenSea, Uniswap — does these three things. The differences are in mechanism (how matching works), structure (centralised vs distributed), and scale (one buyer/seller pair vs millions per second).

For an asset with active trading, there's no such thing as "the price." There's a price *at this moment, in this market, for this size of trade*. The same WETH might cost $3,000 on one DEX and $3,008 on another at the same moment because they're separate markets. The same WETH might cost $3,000 if you buy 1 unit and $3,030 if you buy 100 units in the same market because of liquidity depth.

## The Three Roles

Every market has three economic roles, even if one entity plays multiple:

**Buyers** — want to acquire the asset. Pay the asking price.

**Sellers** — want to dispose of the asset. Accept the bidding price.

**Liquidity providers** — facilitate trades by being willing to buy or sell at any moment. They earn the **spread** (the gap between bid and ask) as compensation for taking inventory risk.

In a CEX (centralised exchange like Coinbase), liquidity providers are typically professional market makers — firms that quote both sides of the order book continuously. In an AMM (decentralised exchange like Uniswap), liquidity providers are anyone who deposits two tokens into a pool. The economic role is the same; the mechanism is different.

## Liquidity and Depth

Market quality is judged primarily by **liquidity** — the ability to trade at the marquee price without moving it.

A market with 100,000 WETH for sale at exactly $3,000 has deep liquidity at $3,000. You can buy 100 WETH and the price doesn't budge. A market with 1 WETH at $3,000 and the next sell order at $3,050 has thin liquidity. Buying 2 WETH means buying the first at $3,000 and the second at $3,050 — your average is $3,025, far above the marquee.

This effect — your trade moving the price as a function of its size — is **slippage**. Slippage is unavoidable in any market; it's just larger or smaller depending on liquidity depth. For Aurix, slippage matters because most of the "free money" arbitrage opportunities the dashboard surfaces would vanish if anyone actually tried to capture them at meaningful size — the act of trading at size moves the prices and eliminates the gap.

A useful mental model: **liquidity is the depth of the order book at each price level**, even when there's no literal order book. In an AMM, "depth" is implicit in the pool's reserves — a pool with 100 WETH and 300,000 USDC is much shallower than a pool with 10,000 WETH and 30,000,000 USDC, even though their marquee prices are identical.

## Price Discovery

When sources disagree about an asset's price, **price discovery** is the process by which the market settles on one number (or a tight range).

In a single venue, price discovery happens continuously through trades. Every trade is a data point: at time T, two parties agreed on price P for size S. The marquee price is essentially the most-recent trade price (with some smoothing).

Across venues, price discovery happens through **arbitrage**. If one venue has WETH at $3,000 and another at $3,008, an arbitrageur:

- Buys cheap on the first venue (which pushes its price up)
- Sells expensive on the second venue (which pushes its price down)
- Pockets the difference (minus costs)

This activity continues until the gap is smaller than execution costs. In equilibrium, prices across venues stay within an "execution cost band" — they never converge perfectly because closing the last few cents of gap costs more than the gap is worth.

Aurix observes this equilibrium directly. Most of what the dashboard shows is the persistent dead zone where small gaps exist but aren't profitable to close.

## Worked Example: A Tiny Market

Consider a hypothetical market for one asset (let's call it FOO) at one moment:

```
Buyers (bids)                  Sellers (asks)
─────────────                  ──────────────
$98 × 50 units                 $103 × 30 units
$97 × 80 units                 $104 × 60 units
$96 × 100 units                $105 × 100 units
$95 × 200 units                $107 × 200 units
```

Reading this:

- The **best bid** is $98 (highest a buyer is willing to pay) for 50 units
- The **best ask** is $103 (lowest a seller is willing to accept) for 30 units
- The **spread** is $103 - $98 = $5
- The **midpoint** is $100.50

If you want to buy 1 unit *right now*, you pay $103 (the best ask).

If you want to buy 100 units right now, you pay 30 × $103 + 60 × $104 + 10 × $105 = $3,090 + $6,240 + $1,050 = $10,380. Your average price is $103.80 — the second 70 units cost more than the first 30 because you exhausted the cheapest level and walked up the order book.

This walking-up-the-book is a literal version of slippage. In an AMM the same thing happens, but the "book" is implicit in the pool's math — the price moves continuously as the reserves shift.

## How This Appears in Aurix

Aurix's `MarketOverview` payload (defined in `src-tauri/src/market/types.rs`) is one tick's view of *four parallel markets* — each DEX is its own market for WETH/USDC, each producing its own price:

```rust
pub struct MarketOverview {
    pub chain: String,
    pub pair_label: String,
    pub fetched_at_unix_ms: u64,
    pub gas_price_gwei: f64,
    pub venues: Vec<PriceSnapshot>,  // <-- four prices, one per venue
}
```

The venues array is ordered: V3 5bps, V3 30bps, V2, SushiSwap. Each `PriceSnapshot` carries a `price_usd: f64` field — that venue's "current marquee price" at the moment of the fetch.

The 1 Hz polling loop in `src/features/arbitrage/ArbitragePage.tsx` calls `fetchMarketOverview()` every second and accumulates a 100-sample history (`HISTORY_LIMIT = 100`). The `insights.ts` engine derives interpretation from that history — the spread between the highest and lowest venue, persistence of which venue is currently cheapest, and so on.

What Aurix is observing, in the language of this file: **four parallel markets for the same asset, with their price discovery processes running independently, and the cross-market gaps that result from that independence.**

## Common Misunderstandings

❌ **"There's one true price for an asset."** There's a price per venue per moment per trade-size. "The price of WETH" is a useful approximation but never exactly true.

❌ **"More liquidity = better market."** Better, yes, but liquidity isn't free. LPs in an AMM face impermanent loss; market makers in a CEX face inventory risk. Liquidity is *paid for* somehow — usually by traders paying spread or fees. A "deep liquidity" claim should always invite the question "what does the LP earn for providing it?"

❌ **"Arbitrage is free money."** Arbitrage is paid work. The arbitrageur has to deploy capital, pay gas, manage execution risk, and compete with other arbitrageurs for the same opportunity. The "free money" framing is what naive observers see; the reality is that any persistent gap is paying *exactly* the marginal arbitrageur's costs (because if it paid more, more arbitrageurs would enter and close it).

❌ **"Price discovery is instant."** Price discovery happens at the speed of trades and information. In high-frequency CEX markets it's microseconds. In on-chain DeFi it's bounded by Ethereum's 12-second block time. That delay is exactly why MEV exists — the gap between "information available" and "transaction included in a block" is a 12-second window where ordering matters.

## Related Files

- `concepts/foundations/exchanges-orderbook-vs-amm.md` — the two technical paradigms for matching buyers and sellers
- `concepts/core/amm-mechanics-v2-and-v3.md` — how AMMs specifically produce prices
- `concepts/core/traders-and-slippage.md` — slippage in detail
- `concepts/core/arbitrage-and-cross-venue-equilibrium.md` — arbitrage as the cross-market price discovery mechanism
- `materials/quant-finance-resources.md` — for going deeper on market microstructure theory
