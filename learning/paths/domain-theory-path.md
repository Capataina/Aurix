# Domain Theory Path

## Who This Path Is For

You can already describe what an AMM is. You want to understand the *math*, the *trade-offs*, and the *full economic picture* of how Ethereum's DeFi ecosystem actually functions. You're treating Aurix as a vehicle for genuinely learning the field rather than just shipping a project.

This path overlaps with `foundations-path` at the start; skip the first stage if you've already done foundations.

## What This Path Assumes

- Foundations-path content (or equivalent prior knowledge)
- Comfort with mathematical notation (you'll see Σ, √, fixed-point representations)
- Willingness to read primary sources (Uniswap V2/V3 whitepapers are referenced as required reading)

## Recommended Sequence

### Stage 1 — Foundations refresh (skip if completed)

- [ ] `concepts/foundations/markets-and-prices.md`
- [ ] `concepts/foundations/exchanges-orderbook-vs-amm.md`
- [ ] `concepts/core/amm-mechanics-v2-and-v3.md`

### Stage 2 — Constant product math at depth

- [ ] `concepts/core/amm-mechanics-v2-and-v3.md` (re-read the V2 sections carefully)
- [ ] `concepts/core/liquidity-providers-and-impermanent-loss.md` (work through the IL example with multiple price moves)
- [ ] `concepts/core/traders-and-slippage.md` (verify the slippage formulas hold for varying trade sizes)
- [ ] `materials/amm-foundational-resources.md` → read the **Uniswap V2 whitepaper** end-to-end

The V2 whitepaper is short (~10 pages) and worth reading in full. Pay attention to section 2.2 (price oracle) and section 3.2 (token routing).

### Stage 3 — Concentrated liquidity (V3)

- [ ] `concepts/advanced/uniswap-v3-tick-mathematics.md`
- [ ] `materials/amm-foundational-resources.md` → read the **Uniswap V3 whitepaper**
- [ ] `project/comparisons/v2-vs-v3-amm-math.md`

The V3 whitepaper is denser (~30 pages with significant math). Don't try to read it once and absorb everything — make multiple passes. Section 6 (mathematics) is the load-bearing one for understanding tick math, Q64.96 representation, and per-tick liquidity accounting.

### Stage 4 — Cross-venue dynamics

- [ ] `concepts/core/arbitrage-and-cross-venue-equilibrium.md`
- [ ] `concepts/domain-patterns/gas-and-execution-costs.md`

Understand why arbitrage equilibrium isn't perfect — gas + slippage + competition leave a "dead zone" of persistent gaps that look profitable but aren't.

### Stage 5 — MEV and the broader ecosystem

- [ ] `concepts/domain-patterns/mev-and-transaction-ordering.md`
- [ ] `concepts/domain-patterns/the-mempool-public-vs-private.md`
- [ ] `concepts/advanced/mempool-mev-detection-mechanics.md`
- [ ] `materials/mev-resources.md` → read the **Flashbots research papers** (linked in materials)

MEV is its own subfield. The research literature is dense but the core ideas (sandwich attacks, JIT liquidity, frontrunning, builder economics) become clear with focused reading.

### Stage 6 — Statistical primitives

- [ ] `concepts/advanced/statistical-primitives-for-risk-modelling.md`
- [ ] `materials/quant-finance-resources.md`

For Tab 5 (risk modelling) and Vector C (ML signal), you'll need volatility, correlation, VaR, Sharpe ratio, and drawdown as working concepts. Not academic-quant depth — applied-quant depth.

### Stage 7 — ML for market microstructure (optional, for Vector C)

- [ ] `concepts/advanced/ml-for-market-microstructure.md`
- [ ] `materials/ml-for-finance-resources.md`

This stage is only relevant if you're planning to ship Vector C. It covers feature engineering for time series, time-split validation (vs random splits, which leak future into past), calibration, and the specific ML challenges that arise in financial data.

## What You Should Understand By The End

- The AMM math at a level where you could implement V2 from scratch
- V3's tick math, Q64.96 representation, sqrtPriceX96 encoding, and the per-tick liquidity model
- Why concentrated liquidity is more capital-efficient AND more vulnerable to IL than V2
- The relationship between gas costs, slippage, and arbitrage equilibrium
- The MEV ecosystem: sandwich attacks, JIT liquidity, frontrunning, builder economics
- Public vs private mempools and why pros use Flashbots
- Statistical primitives at applied-quant depth (vol, correlation, VaR, Sharpe, drawdown)
- (If completed) The specific ML challenges in financial time series

## Estimated Time

- Stages 1-3: 4-6 hours (the V2 and V3 whitepapers dominate)
- Stages 4-5: 3-5 hours
- Stage 6: 2-3 hours
- Stage 7: 4-8 hours

Total: 13-22 hours of focused reading, more if you go deep on the primary sources.

## What To Do Next

| Goal | Next path |
|---|---|
| Apply this knowledge to ship something | `vector-prep-path.md` (pick A for tick math, C for ML) |
| Understand the codebase concretely | `project-systems-path.md` |
| Develop interview talking points | `interview-fluency-path.md` |
