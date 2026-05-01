# ML for Finance Resources

Resources for the specific problem of applying machine learning to financial time series — the core methodology Vector C requires.

## Why It Matters For This Repo

Vector C (ML arbitrage-survival classifier) needs methodology that's fundamentally different from "standard" ML on i.i.d. data. The gotchas — time-split validation, calibration, regime change, fat tails — are the difference between a useful model and a useless one.

## Primary Sources

### "Advances in Financial Machine Learning" — Marcos López de Prado

- Format: textbook (~400 pages)
- Difficulty: Moderate-high
- The canonical reference for ML in finance

For Vector C specifically, mandatory reading:
- **Chapter 4**: Sample Weights — how to weight observations when overlapping data is correlated
- **Chapter 7**: Cross-Validation in Finance — why random splits fail and how to do time-based splits correctly
- **Chapter 8**: Feature Importance — methods that handle correlated features

Optional but useful:
- Chapter 11 (model backtesting) — connects ML evaluation to financial PnL evaluation
- Chapter 17 (machine learning asset allocation) — applied portfolio context

### Calibration Literature

The standard references on calibration:

- **Platt 1999** — "Probabilistic Outputs for Support Vector Machines" — original Platt scaling paper
- **Niculescu-Mizil & Caruana 2005** — "Predicting Good Probabilities with Supervised Learning" — empirical comparison of calibration methods
- **Guo et al. 2017** — "On Calibration of Modern Neural Networks" — focused on NN miscalibration

For Vector C, you don't need to read these in depth — sklearn's calibration documentation is sufficient. But if you're optimising the calibration stage carefully, the original references help.

### "Trading and Exchanges" — Larry Harris

- Format: textbook (~600 pages)
- Difficulty: Moderate
- What you'll learn: market microstructure, the institutional context for understanding what features matter in trading data

Not ML-specific, but essential context. Chapters on order flow, market makers, and liquidity are particularly relevant.

## Online Courses

### Hudson & Thames Quantitative Research Tutorials

- URL: `hudsonthames.org`
- Format: code-heavy blog posts and notebooks
- What you'll learn: practical applications of López de Prado's methodology

Good for seeing the methodology in code. Their MlFinLab Python library implements many of the López de Prado techniques.

### Stanford CS229 — "Machine Learning"

- Format: lecture series, free online
- What you'll learn: foundational supervised learning, including logistic regression, model selection, regularization

Standard ML primer. Useful if your coursework background is rusty. Skip the deep learning sections (they're not relevant to Vector C).

## Topic-Specific

### Time Series Cross-Validation

- López de Prado chapter 7
- Walk-forward validation methodology
- Purged k-fold (handles overlapping samples)
- The "leakage in finance" problem

### Calibration Diagnostics

- sklearn `CalibrationDisplay` documentation
- Reliability diagrams
- ECE (Expected Calibration Error)
- The conformal prediction framework (more advanced)

### Regime Detection

- Hidden Markov Models for regime identification
- Change-point detection methods
- "Beyond stationarity" papers

For Vector C's V1, you don't need explicit regime detection — but it's useful to know about for V2 if calibration drift becomes a problem.

### Feature Engineering for Time Series

- Rolling statistics (mean, std, skew, kurtosis)
- Lagged features (auto-regressive components)
- Cyclical encoding (time of day, day of week)
- Cross-feature interactions

Standard methodology; covered in any time-series-aware ML resource.

## ONNX-Specific Resources

### ONNX Documentation

- URL: `https://onnx.ai`
- What you'll learn: the Open Neural Network Exchange format Vector C uses to bridge Python training to Rust inference

For Aurix's deployment path:
- ONNX format spec (basic understanding)
- ONNX Python export from sklearn / XGBoost / PyTorch
- ONNX Runtime for inference

### tract (Rust ONNX runtime)

- Repo: `https://github.com/sonos/tract`
- What you'll learn: pure-Rust ONNX runtime, simpler dependency than `ort`
- Best fit for Aurix's "no heavy dependencies" preference

### ort (Rust binding to ONNX Runtime)

- Repo: `https://github.com/pykeio/ort`
- What you'll learn: FFI binding to Microsoft's ONNX Runtime, faster than tract
- Use if Vector C's latency benchmarks require it

## Adjacent Reading

### "Algorithmic Trading: Winning Strategies and Their Rationale" — Ernest Chan

- Format: ~250 pages
- For specific strategy types and how they're typically tested
- Useful background for understanding what "good" looks like in trading ML

### "Quantitative Trading" — Ernest Chan

- Earlier book, similar themes
- Practical and approachable

## When To Read What

**For Vector C V1**: López de Prado chapters 4 and 7 + sklearn calibration docs + Hudson & Thames tutorials. ~10-15 hours.

**For Vector C V2 (mempool features added)**: add Trading and Exchanges sections on order flow + regime detection background. ~5-10 hours.

**For interview-level fluency**: López de Prado chapter 7 alone is enough to discuss ML methodology in finance credibly. ~3 hours.

## Related Files

- `concepts/advanced/ml-for-market-microstructure.md` — methodology covered conceptually
- `concepts/advanced/statistical-primitives-for-risk-modelling.md` — the stats foundations
- `context/plans/vector-c-ml-arbitrage-survival.md` — implementation plan
