# Statistical Primitives for Risk Modelling

## Why This Matters Here

Tab 5 (the README's "Token Correlation & Risk Dashboard") and Vector C (ML signal) both require a working understanding of standard quantitative finance metrics. This file is a focused introduction to the primitives — not academic depth, but enough to use them correctly.

## Prerequisites

- High-school statistics (mean, standard deviation, normal distribution)
- `concepts/core/arbitrage-and-cross-venue-equilibrium.md` (general market vocabulary)

## Status

Foundational domain knowledge. Not yet implemented in Aurix — Tab 5 is unbuilt; Vector C will use these.

## Returns vs Prices

Almost all of quantitative finance works with **returns**, not prices. There are two flavours:

**Simple return** between t-1 and t:

> **r_t = (P_t - P_{t-1}) / P_{t-1}**

A simple return of 0.05 means "the price went up 5%."

**Log return** between t-1 and t:

> **r_t = ln(P_t / P_{t-1})**

Log returns are slightly different (log of 1.05 ≈ 0.0488, not 0.05) but have nicer mathematical properties — they're additive over time (the 30-day log return = sum of 30 daily log returns), and they're more symmetric (a 50% gain followed by a 50% loss returns you to the same place in log return space).

Almost all quant finance uses log returns. Stick with that convention.

## Volatility

The standard deviation of returns over a period, typically **annualised** so different observation frequencies are comparable.

**Daily standard deviation**:

> **σ_daily = √( Σ(r_t - mean_r)² / (N - 1) )**

Where r_t is the daily log return and N is the number of days.

**Annualised**: multiply by √252 (252 trading days per year):

> **σ_annual = σ_daily × √252**

For crypto specifically, you might use √365 (every day is a trading day) — the choice depends on your data. Be consistent.

### Interpretation

WETH has annualised vol of roughly 60-100% in normal periods. Bitcoin: 50-80%. S&P 500: 15-20%. A "stable" stock: 20-30%. A high-vol meme coin: 200%+.

Vol of 80% means: in a typical year, the asset's price moves by about 80% of its starting value (one standard deviation). Big moves (2σ, 3σ) are dramatically larger.

### Vol Clustering

Returns are NOT independently distributed across time. High-vol periods cluster (volatility persists for hours/days), then return to low vol. This is "volatility clustering" — a stylised fact that motivates models like GARCH that explicitly capture vol's autocorrelation structure.

For Aurix's Tab 5 purposes, **rolling annualised volatility** computed over a sliding window (e.g. 30-day rolling window) captures regime shifts naturally without needing GARCH.

## Correlation

Measures how two assets move together. Range: -1 (perfectly opposite) to +1 (perfectly together), with 0 meaning no linear relationship.

**Pearson correlation** of returns r_A and r_B over a period:

> **ρ = Σ((r_A - mean_A)(r_B - mean_B)) / √(Σ(r_A - mean_A)² × Σ(r_B - mean_B)²)**

In code, every stats library has this built in.

### Interpretation

ETH and BTC have correlation ~0.85 in most periods (they move together strongly). USDC and USDT have correlation ~1.0 (both pegged to USD). ETH and traditional equities: 0.3-0.5 (some correlation, varies by regime). ETH and gold: usually 0.0-0.2 (essentially uncorrelated).

### What Correlation Misses

Correlation captures LINEAR relationships. Two assets that move together in normal times but decouple violently during crises ("tail correlation") would have moderate Pearson correlation but be misleading. For risk, **tail correlation matters more than average correlation** — assets you thought were diversified often turn out to be highly correlated during crashes.

For Tab 5 purposes, also compute **rolling correlation** over time. A constant 1-year correlation hides that the correlation might have been 0.3 for 11 months and 0.95 for 1 month (the crisis month).

## Sharpe Ratio

The classic risk-adjusted return metric:

> **Sharpe = (mean_return - risk_free_rate) / std_dev_of_returns**

The numerator is "excess return over the risk-free rate." The denominator is volatility.

### Interpretation

Sharpe lets you compare strategies fairly:
- "10% return at 5% vol" → Sharpe ~2.0
- "20% return at 20% vol" → Sharpe ~1.0

The first is more efficient even though it has lower raw returns. You could leverage 2× the first strategy and get "20% return at 10% vol" (Sharpe 2.0 still), which dominates the second.

### Practical Sharpe Levels

- Sharpe 1.0: decent strategy, beats long-only buy-and-hold
- Sharpe 1.5: strong active strategy
- Sharpe 2.0: very strong (institutional quant target)
- Sharpe 3.0+: rare, often a sign of overfitting

A backtested Sharpe of 4 is almost certainly overfitting unless validated on multiple years of held-out data.

### Risk-Free Rate Choice

For traditional finance: use US Treasury yields (3-month T-bill). For crypto: there's no clean "risk-free rate." Common choices:
- 0% (treats all return as excess return)
- DAI/USDC lending rate on Aave (~3-7% depending on conditions)
- US T-bill rate (~5% in 2024)

Be explicit about which choice you used.

## Maximum Drawdown

The largest peak-to-trough decline in equity over a period.

**Procedure**:
1. Track the running maximum of equity
2. At each point, compute drawdown = (current_equity - running_max) / running_max
3. Drawdown is always ≤ 0 (negative or zero)
4. Max drawdown = the most negative drawdown observed

### Interpretation

Max drawdown of -20% means: at some point during the period, the strategy lost 20% from its peak. This is more important to many investors than average return — a strategy with Sharpe 2.0 and -50% max drawdown is harder to live with than a Sharpe 1.0 with -10% max drawdown.

### Calmar Ratio

A drawdown-aware Sharpe alternative:

> **Calmar = annualised_return / |max_drawdown|**

Trades are evaluated on "return per unit of pain." High Calmar = consistent gains with small drawdowns.

## Value-at-Risk (VaR)

A risk metric: the maximum loss expected at a given confidence level over a given period.

**1-day VaR at 95% confidence = $X** means: "I expect to lose less than $X on 95% of days."

Equivalently: "On 5% of days (about 13 days per year), I expect losses larger than $X."

### Computation Methods

**Historical VaR**: take the 5th percentile of historical daily returns (the 5%-worst day). Multiply by portfolio size. Easiest method, sample-size dependent.

**Parametric (Gaussian) VaR**: assume returns are normally distributed. 95% VaR = 1.645 × σ × portfolio_size. Easy but underestimates tail risk because real returns have fat tails.

**Monte Carlo VaR**: simulate many possible price paths using assumed distributions, take the percentile of simulated losses. Most flexible but most computationally expensive.

### Limitations

VaR captures the threshold but not the tail. A portfolio might have 95% VaR of $10K but average loss CONDITIONAL on exceeding VaR could be $50K (the "bad days are really bad" case). **Conditional VaR (CVaR / Expected Shortfall)** addresses this:

> **CVaR_95% = mean of returns in the worst 5% of cases**

CVaR is more robust than VaR but harder to estimate from limited data.

### Coherence Issues

VaR is not a "coherent" risk measure in the technical sense — it can fail subadditivity (the VaR of a portfolio can exceed the sum of individual VaRs). CVaR is coherent. For serious risk management, use CVaR; for regulatory and reporting, VaR remains the standard.

## Beta

The sensitivity of an asset's returns to market returns. From the regression:

> **r_asset = α + β × r_market + ε**

A beta of 1.5 means the asset moves 1.5× as much as the market. A beta of 0.5 means half. A beta of 0 is "market neutral."

For crypto: most tokens have BTC as the "market." A token's beta to BTC tells you how much of its movement is just BTC movement.

## Information Ratio

Sharpe-like metric for active management:

> **IR = (strategy_return - benchmark_return) / std_dev(strategy_return - benchmark_return)**

Measures "excess return per unit of tracking error." Used to evaluate strategies that aim to outperform a benchmark (e.g. "WETH-strategy vs hold WETH").

## Putting It Together: A Risk Dashboard

For Tab 5, the user defines a portfolio (e.g. 60% WETH, 30% WBTC, 10% USDC). The dashboard computes:

| Metric | What it tells you |
|---|---|
| Annualised return | Average yearly performance |
| Annualised volatility | Risk of bouncing around |
| Sharpe ratio | Return per unit of risk |
| Maximum drawdown | Worst peak-to-trough loss |
| Pairwise correlation matrix | Which holdings are diversifying vs duplicating |
| 1-day VaR at 95% | Expected daily loss threshold |
| 1-day CVaR at 95% | Expected loss given a bad day |
| Beta to BTC | Crypto-market sensitivity |

A good risk dashboard surfaces all of these in a way that's actionable. "Your portfolio is 95% concentrated in BTC-correlated assets; you have less diversification than you think" is the kind of insight Tab 5 should produce.

## Common Misunderstandings

❌ **"Sharpe ratio measures return."** It measures risk-adjusted return. A Sharpe of 0 with 10% return is worse than a Sharpe of 1 with 5% return.

❌ **"Correlation captures all relationship."** It captures LINEAR relationships. Tail dependence (assets coupling during crashes) is invisible to standard correlation.

❌ **"Vol = risk."** Vol is one measure of risk. Tail risk (rare large losses) is a different dimension. A strategy can have low vol but high tail risk (e.g. selling deep out-of-the-money puts — small daily P&L until the catastrophic day).

❌ **"VaR tells you the worst case."** VaR tells you the threshold at a confidence level. The worst case is unknown — VaR doesn't bound it. CVaR partially addresses this by reporting the AVERAGE in the tail.

❌ **"Backtested Sharpe of 3 means the strategy is strong."** Sharpe of 3 in a backtest is almost always overfitting. Real ex-post Sharpe of 1.5 is strong; 2.0 is excellent; anything higher is suspicious unless validated on out-of-sample multi-year data.

## Related Files

- `concepts/advanced/ml-for-market-microstructure.md` — these primitives feed Vector C's feature engineering
- `materials/quant-finance-resources.md` — for going deeper (Wilmott, Lopez de Prado)
- `context/plans/vector-c-ml-arbitrage-survival.md` — calibration is partly a stats problem
