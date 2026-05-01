# AMM Foundational Resources

Curated resources for understanding Automated Market Makers — the underlying technology of every venue Aurix watches.

## Why It Matters For This Repo

Aurix decodes V2 reserve ratios and V3 sqrtPriceX96 to produce the prices on the dashboard. Understanding AMM math at the source level is essential for shipping Vector A (V3 LP backtester) honestly.

## How To Use This List

The Uniswap whitepapers are the primary sources. Read those first; everything else is supplementary. Don't try to absorb everything in one pass — the V3 whitepaper especially benefits from multiple read-throughs.

## Primary Sources

### Uniswap V2 Whitepaper

- URL: `https://uniswap.org/whitepaper.pdf`
- Length: ~10 pages
- Difficulty: Moderate (some math, no advanced PDE/algebra)
- What you'll learn: the constant-product formula, the price oracle mechanism (TWAP), token routing across pairs

Read sections 2 (formal model) and 3.2 (token routing) carefully. Section 2.2 (price oracle) explains how V2 produces TWAP price feeds — useful background for understanding why V3 introduced sqrtPriceX96.

After reading: you should be able to derive the swap math (`amount_out = (amount_in_with_fee × reserve_out) / (reserve_in + amount_in_with_fee)`) yourself from the constant-product invariant.

### Uniswap V3 Whitepaper

- URL: `https://uniswap.org/whitepaper-v3.pdf`
- Length: ~30 pages
- Difficulty: High (significant fixed-point math and notation)
- What you'll learn: concentrated liquidity, Q64.96 representation, tick math, liquidity-as-virtual-reserves model, fee growth tracking

Section 6 is the critical one — it covers the math in full. Make multiple passes. The first pass is for shape; subsequent passes for precision. By the third pass, you should be able to translate the equations to code.

The notation can be unfamiliar — `√P` and `L = √(x×y)` both represent design choices that have specific reasons (covered in `concepts/advanced/uniswap-v3-tick-mathematics.md`).

After reading: you should understand why V3 stores `√P × 2^96` rather than `P` directly, and you should be able to convert ticks to prices and back without looking up the formula.

### Uniswap V3 SDK

- Repo: `https://github.com/Uniswap/v3-sdk`
- Language: TypeScript
- What you'll learn: a reference implementation of all the V3 math primitives

If your Vector A implementation produces results that disagree with the V3 SDK's, the SDK is almost certainly right. Use it as ground truth for tick math, position calculations, and quote estimation.

The TypeScript code is readable Rust-like. Look at `TickMath.ts`, `LiquidityMath.ts`, `SqrtPriceMath.ts` for the math; `Pool.ts` for the high-level state machine.

## Supplementary Reading

### "Uniswap V3: A New Era of Liquidity" — Hayden Adams

- URL: Uniswap blog
- Length: ~5 pages
- Difficulty: Light
- Good for the high-level intuition before diving into the whitepaper

### "Uniswap V3 Math Primer" — Atis E

- URL: searchable as a PDF on arXiv-style aggregators
- Length: ~50 pages
- Difficulty: Moderate-to-high
- A reader-friendly walkthrough of V3's math with worked examples

This is the best secondary source if the whitepaper feels too dense. It walks through the same material more pedagogically.

### Uniswap V2 Source Code

- Repo: `https://github.com/Uniswap/v2-core`
- Language: Solidity
- What you'll learn: how the math actually gets implemented in production

The `UniswapV2Pair.sol` contract is short and readable. The `swap` function is a worked example of the math from the whitepaper.

### Uniswap V3 Source Code

- Repo: `https://github.com/Uniswap/v3-core`
- Language: Solidity
- What you'll learn: the production implementation of tick math, fee growth, position management

`UniswapV3Pool.sol` is significantly more complex than V2. Don't try to read end-to-end on first pass — focus on `swap()` first, then `mint()`/`burn()` for LP mechanics.

## When To Read What

**Before reading any Aurix code**: V2 whitepaper sections 2 and 3.

**Before starting Vector A**: V3 whitepaper sections 2-6, with section 6 being mandatory. V3 SDK as reference.

**For deep understanding**: V2 + V3 source code, plus Atis E's primer.

**For interview prep**: high-level concepts from V2 whitepaper + V3 whitepaper section 1-2 (motivations) is usually sufficient.

## Adjacent AMM Designs (Optional)

Other notable AMM designs worth knowing for breadth:

- **Curve (StableSwap)** — designed for stablecoin pairs, uses a different math curve to minimise slippage
- **Balancer** — multi-asset pools with weighted reserves
- **Kyber Network** — DEX aggregator with multiple pricing sources
- **CowSwap (Coincidence of Wants)** — batch auction model

None of these directly apply to Aurix today, but they're good for understanding the design space. Curve is particularly interesting for Tab 5 considerations (correlated stablecoin pairs).

## Related Files

- `concepts/core/amm-mechanics-v2-and-v3.md` — concept treatment
- `concepts/advanced/uniswap-v3-tick-mathematics.md` — V3 deep math
- `project/comparisons/v2-vs-v3-amm-math.md` — side-by-side
- `context/plans/vector-a-v3-lp-backtester.md` — implementation plan
