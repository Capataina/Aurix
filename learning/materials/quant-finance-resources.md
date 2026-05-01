# Quant Finance Resources

Resources for the statistical and quantitative finance background needed for Tab 5 (risk modelling) and Vector C (ML signal). Not academic-quant depth — applied-quant depth.

## Why It Matters For This Repo

Tab 5's job is to compute correlation, volatility, VaR, Sharpe, and drawdown for user-defined portfolios. Vector C uses calibrated probabilities, time-split validation, and feature engineering on time series. Both require the basics covered in these resources.

## Primary Sources

### "Paul Wilmott Introduces Quantitative Finance" — Paul Wilmott

- Format: textbook (~700 pages)
- Difficulty: Moderate-to-high (graduate-level math)
- What you'll learn: Black-Scholes, options pricing, stochastic calculus, term structure of interest rates

Wilmott is famously verbose and irreverent. Don't try to read end-to-end — use it as a reference for specific topics. Chapters 1-5 cover the foundations (returns, vol, normal distribution assumptions); Chapter 6 covers risk neutrality (skip if not doing options); the rest is options-specific and less relevant for Aurix.

For Aurix purposes, this is a "look up specific topics when needed" resource, not "read cover to cover."

### "Active Portfolio Management" — Grinold & Kahn

- Format: textbook (~600 pages)
- Difficulty: Moderate
- What you'll learn: Sharpe-style return analysis, Information Ratio, the math of skill, practical portfolio construction

The Sharpe / IR / breadth-skill framework here is foundational for thinking about active strategies. Tab 5's purpose dovetails with this material.

### "Advances in Financial Machine Learning" — Marcos López de Prado

- Format: textbook (~400 pages)
- Difficulty: Moderate-high
- What you'll learn: ML methodology for financial time series, including the time-split discipline Vector C needs

This is the canonical reference for ML in finance. Chapter 7 (cross-validation in finance) is mandatory reading before shipping Vector C — it explains exactly why random splits fail and how to do time-based splits correctly. Chapter 8 (feature importance) is also valuable.

López de Prado is a working quant and the book is opinionated; some recommendations are debated in the field. But the time-split methodology is well-grounded and broadly accepted.

## Online Courses

### "Stanford CS229" — Andrew Ng

- URL: searchable; lecture notes online
- Format: lecture series
- What you'll learn: the ML foundations Vector C uses (logistic regression, regularization, model selection, calibration)

Good general ML grounding. If you've done ML coursework before, you can skim — focus on classification and calibration sections.

### "QuantPy" YouTube channel

- Format: video tutorials
- What you'll learn: practical quant analysis in Python

Good for seeing how the abstract concepts get implemented. Not deep theory, but useful for "what does this look like in code."

## Domain-Specific

### "Quantitative Risk Management" — McNeil, Frey, Embrechts

- Format: textbook (~700 pages)
- Difficulty: High (theoretical)
- What you'll learn: rigorous treatment of VaR, CVaR, copulas, extreme value theory

Reference-grade. For Tab 5's purposes, you don't need this depth — but it's the canonical source if you want to go beyond "compute historical 95% VaR" into "model the tail correctly."

### "Statistical Methods for Financial Engineering" — Hodzic et al.

- Less famous but more accessible than McNeil for the same material

## Topic-Specific Reading

### Volatility

- "Volatility Modeling Methods for Financial Markets" — Glasserman and various
- GARCH and its variants (capture vol clustering)
- Realized volatility from high-frequency data

For Aurix Tab 5, "rolling annualised standard deviation of log returns" is sufficient. Don't reach for GARCH unless you have a specific reason.

### Correlation

- Pearson is the workhorse
- Spearman (rank-based) is more robust to outliers
- Tail correlation / dynamic conditional correlation for crisis modelling
- For crypto specifically: "On the Cross-Asset Diversification Benefits of Cryptocurrencies" — useful empirical work

### VaR / CVaR

- "Expected Shortfall: A Natural Coherent Alternative to VaR" — Acerbi & Tasche
- Foundational paper on CVaR's properties

### Sharpe and Friends

- The original Sharpe paper (1966) is short and worth reading
- "Why Sharpe Ratio is Often Misleading" — many short blog posts on this

## Light Reading

### "When Genius Failed" — Roger Lowenstein

- Story of Long-Term Capital Management's collapse
- Worth reading for the empirical lesson about correlation breakdown in crises

### "More Money Than God" — Sebastian Mallaby

- History of hedge funds; useful context for understanding the institutional quant world Aurix would interact with at the hiring level

## When To Read What

**For Tab 5 implementation**: Wilmott chapters 1-5 + López de Prado chapter 7 + topic-specific reading for VaR/correlation. ~10-20 hours total.

**For Vector C implementation**: López de Prado chapter 7 (time-split validation) is the critical read. Plus calibration material from sklearn docs. ~5-10 hours.

**For interview-level fluency**: foundational concepts (return, vol, Sharpe, correlation, VaR) from Wilmott + practical sense from QuantPy. ~5-8 hours.

## Related Files

- `concepts/advanced/statistical-primitives-for-risk-modelling.md` — concept treatment for these primitives
- `concepts/advanced/ml-for-market-microstructure.md` — Vector C's ML methodology
- `context/plans/vector-c-ml-arbitrage-survival.md` — implementation plan
