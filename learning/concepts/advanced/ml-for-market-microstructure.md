# ML for Market Microstructure

## Why This Matters Here

Vector C (ML arbitrage-survival classifier) requires applying machine learning to financial time series — a domain with specific gotchas that don't apply to image classification or NLP. This file covers the ML methodology specifically for financial data: time-split validation, calibration, feature engineering for time series, and the regime-change problem.

## Prerequisites

- `concepts/core/arbitrage-and-cross-venue-equilibrium.md` (you must understand what we're predicting)
- General ML familiarity (you've trained models before — this file isn't an ML primer)

## Status

Foundational domain knowledge. Not yet implemented in Aurix — Vector C's plan is to implement this.

## The Prediction Problem

Aurix observes spread states once per second across 4 venues (so 6 pairwise spreads per tick). For each observation, we want to answer: **will this spread persist long enough to be (theoretically) executable?**

Concretely:

- **Input**: market state at time t (current spread, recent volatility, gas, time-of-day, ...)
- **Output**: probability that the spread will still be open with the same sign at time t+30s

This is a binary classification problem if we threshold ("will it last 30s? yes/no") or a regression problem if we predict the probability directly. We want probability — calibrated probability — because downstream we'll multiply by gross spread to compute expected value.

## The Data

After the persistence layer (M2.0 / M-ML.1) is in place, Aurix's collector accumulates per-tick snapshots:

```
timestamp, v3_5bps_price, v3_30bps_price, v2_price, sushi_price, gas_gwei
```

Each tick is a row. After 7 days of continuous collection, you have ~600,000 rows. After 30 days, ~2.6 million rows. After 90 days, ~7.8 million.

For a binary classifier, each row needs a label:

```
look forward 30s: did the spread (v3_5bps - sushi) keep the same sign for all of those 30 ticks?
```

Compute this for every row by walking 30 steps forward.

## The First Critical Mistake: Random Splits

In standard ML practice, you split data randomly into train/val/test. For time series, **this leaks future into past.**

Suppose you train on 80% randomly-sampled rows and test on the remaining 20%. The test rows are interleaved chronologically with training rows — for any test row at time t, there are training rows at times t-1, t+1, t-30, t+30, etc. Your model has implicitly seen the surrounding context.

The classifier looks great on this test set (AUC 0.95) but fails on truly unseen data (AUC 0.55). The "performance" was overfitting to information that wouldn't be available in real prediction.

The fix: **time-based splits.**

```
Train: oldest 60% of data (rows 1 to 1.56M out of 2.6M, say)
Validation: middle 20% (rows 1.56M to 2.08M)
Test: most recent 20% (rows 2.08M to 2.6M)
```

The model trains on the past, validates on the more recent past, and is tested on the most recent unseen data. This is the only valid validation methodology for financial time series.

## The Second Critical Mistake: Look-Ahead Features

Even with time-based splits, you can still leak future into past via features. If your feature set includes "trailing 30-second rolling volatility computed at time t" and your label is "will the spread persist in the next 30 seconds," then the rolling vol uses data from t-30 to t — but in real prediction at time t, you only have data up to t.

Wait — that's fine, that's the same time. The mistake would be computing "trailing 30s rolling vol centered at t" which uses data from t-15 to t+15. The "+15" is future leak.

Be careful. Every feature must be **strictly causal** — computed only from data at or before the prediction time.

## Calibration

A binary classifier outputs probabilities. For the model to be useful in expected-value calculations, those probabilities must be **calibrated**: when the model says "70% probability," the event should actually occur 70% of the time on held-out data.

Many ML algorithms (XGBoost, neural networks) produce uncalibrated probabilities — the score is monotonic with the true probability but not equal to it. Calibration is a post-training step that maps raw scores to calibrated probabilities.

Two standard methods:

### Platt Scaling

Fit a logistic regression on top of the model's raw scores using the validation set:

```
calibrated_prob = sigmoid(a × raw_score + b)
```

Where `a` and `b` are fit by maximum likelihood on the validation set. Simple, fast, usually sufficient.

### Isotonic Regression

A non-parametric calibration method: fit a monotone-increasing function from raw score to calibrated probability. More flexible than Platt scaling but requires more validation data.

### Reliability Diagrams

The diagnostic. Bin predictions by raw score (say, 10 bins from 0 to 1). For each bin, plot mean predicted probability vs observed positive rate.

A perfectly calibrated model lies on the diagonal. Above the diagonal = under-confident. Below the diagonal = over-confident.

```
Observed positive rate
  1.0 │                                  *
      │                              *
  0.8 │                          *
      │                      *
  0.6 │                  *
      │              *
  0.4 │          *
      │      *
  0.2 │  *
      │
  0.0 └────────────────────────────────────
      0.0   0.2   0.4   0.6   0.8   1.0
              Predicted probability
```

ECE (Expected Calibration Error) summarises this in a single number. Target ECE < 0.05 for production use.

## Feature Engineering for Time Series

The features for Vector C:

### Spread features
- All 6 pairwise spreads (V3-5bps↔V3-30bps, V3-5bps↔V2, ...) in absolute USD and basis points
- Spread sign indicator (which venue is currently cheaper)

### Recent volatility
- Rolling std of mid-price over multiple windows (30s, 5min, 30min)
- Captures the regime: low-vol regimes have stickier spreads, high-vol regimes have noisier spreads

### Gas features
- Current gas price (gwei)
- Gas relative to 1-hour rolling median (a regime indicator)

### Time features
- Time of day (cyclic encoded as `sin(2π × hour/24)` and `cos(...)` for circularity)
- Day of week (one-hot for Mon-Sun)
- Captures session effects (US open, Asia open, weekend lull)

### Liquidity features
- V2 reserve sizes (deeper pool = stickier price)
- V3 in-range liquidity (when available — requires reading more pool state than current Aurix)

### Cross-feature interactions
- Gas-adjusted spread (already-computed metric)
- Spread × volatility (high spread in volatile regime is more likely to persist)

The standardisation: zero-mean unit-variance per-feature, computed from training set statistics, applied identically at inference.

## Time-Series-Specific Gotchas

### Regime change

Markets shift. A model trained on March 2026 data may not work in October 2026 if the underlying dynamics changed (new liquidity sources, new MEV strategies, regulatory shifts). Mitigation:

- Retrain regularly (weekly batch is reasonable for V1)
- Monitor calibration drift over time (track ECE day-by-day)
- Maintain a "regime indicator" feature that captures observable shifts

### Fat tails

Financial returns have fat tails — extreme events occur much more often than a normal distribution would predict. Models that assume normality (e.g. linear regression) are biased; tree-based methods (XGBoost) and ensembles handle fat tails better.

### Autocorrelation

Consecutive observations are NOT independent — the state at time t is highly correlated with t-1. This breaks the i.i.d. assumption underlying most ML theory. Implications:
- Standard error estimates are too tight (the "effective sample size" is much less than N)
- Cross-validation needs to use time-based blocks, not random folds
- Some models (RNNs, LSTMs) explicitly model the temporal dependence

### Class imbalance

Spread persistence at 30s might be very rare or very common depending on the venue pair and time of day. If your positive class is <5% or >95%, accuracy becomes meaningless — use AUC, precision/recall, or balanced accuracy instead.

## Choice of Algorithm

For Vector C's MVP, the options:

### Logistic regression (baseline)

Fast, interpretable, calibrated by default. Should be the FIRST model trained. Establishes a floor — if your fancier model doesn't beat this, your features are the problem.

### XGBoost (workhorse)

Gradient boosting on decision trees. Handles tabular features well, captures non-linear interactions, robust to outliers. Probably the right choice for V1 because:
- Fast training (minutes on 600k rows)
- Built-in handling of missing values
- Good performance out of the box
- Existing Caner skill (already on resume)

### Small MLP via PyTorch (signal-of-ML-depth)

A 3-layer feedforward network. Probably not significantly better than XGBoost for this feature size, but signals NN literacy on the resume. Add only if XGBoost is shipped first.

### Recurrent / Sequence models (premature)

LSTMs or transformers on the raw time series would be more sophisticated but introduce a lot of complexity. Not warranted for V1. Could be V3 if Vector C ships and proves valuable.

## Cross-Runtime Deployment

Vector C trains in Python (sklearn / XGBoost / PyTorch) but deploys in Rust (Aurix's backend). The bridge is ONNX:

1. Train in Python, export to ONNX
2. Load ONNX model in Rust via `tract-onnx` or `ort`
3. Implement feature pipeline in Rust mirroring Python exactly
4. Cross-runtime parity test: pick 100 historical observations, run both pipelines, predictions must match within 1e-5

The cross-runtime parity test is the most important quality gate. Feature transformation drift between Python and Rust is the #1 silent failure mode in deployed ML — your model trained on one feature space and infers on a slightly different one, and accuracy degrades silently.

## Common Misunderstandings

❌ **"Random splits work fine for time series if I shuffle within each split."** No. Any random shuffling leaks information from one period to another. Strict chronological splits are mandatory.

❌ **"Higher AUC means better model."** Higher AUC means better discrimination. It says nothing about calibration. A model with AUC 0.85 and ECE 0.30 is much worse for expected-value computation than a model with AUC 0.75 and ECE 0.02.

❌ **"More features always helps."** More features risks overfitting and slows training. Each feature should have a justification — a hypothesis for why it should help. Start with ~10 features, add more only when validation justifies it.

❌ **"My model is 92% accurate so it's good."** Accuracy is misleading on imbalanced classes. If 90% of samples are negative, "always predict negative" achieves 90% accuracy. Use AUC, precision/recall, or balanced accuracy instead.

❌ **"The model says 'trade now,' so I should trade now."** Even if the model is well-calibrated and AUC is high, real execution faces gas, slippage, and competition. Aurix's read-only design exists in part because the gap between "model says positive expected value" and "trade actually profits" is massive on public mempool.

## Related Files

- `concepts/advanced/statistical-primitives-for-risk-modelling.md` — the stats vocabulary
- `materials/ml-for-finance-resources.md` — Lopez de Prado, Wilmott, etc.
- `context/plans/vector-c-ml-arbitrage-survival.md` — the implementation plan
- `concepts/core/arbitrage-and-cross-venue-equilibrium.md` — what we're trying to predict
