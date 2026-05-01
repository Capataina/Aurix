---
title: "Vector C — ML Arbitrage-Survival Classifier"
status: proposed
created: 2026-05-01
vector: C
hiring-audience: crypto-quant desks (Wintermute quant, Jump Crypto), MEV-Boost research, Paradigm-portfolio ML+DeFi cross-section roles
estimated-effort: 4-8 weeks focused
depends-on: persistence layer (shared with Vector A's M2.0); ≥7 days of continuous data collection before training
---

# Vector C — ML Arbitrage-Survival Classifier

## Goal

Train a calibrated machine-learning classifier that, given current observed market state, predicts the probability an arbitrage opportunity (a positive spread between two venues) will persist long enough for execution to be theoretically profitable. Surface predictions per-opportunity in the UI as confidence badges and gas-adjusted expected-value tags. Aurix never executes; the model is purely observational and educational.

## Why This Vector

- **Cross-stack hiring signal nobody else has.** Most DeFi engineers can't do ML; most ML engineers can't do DeFi. Caner has tinygrad ONNX LSTM and burn A-FINE OSS contributions on the resume — the ML half is documented. Adding "and I deployed a trained ML model in a Rust DeFi backend with sub-ms inference" is a combination that almost no candidate can show.
- **Builds on existing prior art.** PyTorch / TensorFlow / ONNX Runtime / NEAT / DEAP / XGBoost / scikit-learn are all already on the resume. Vector C exercises every one of them in a single coherent project.
- **Compounds with Vectors A and B.** Vector B's mempool features become Vector C's input features ("given current spread state AND current mempool flow, predict survival"). Vector A's backtester becomes Vector C's evaluation harness ("the model said 73%; what would have happened if a trader had acted on every >70% confidence signal?").
- **Audience: the rare crypto-quant cross-section.** Wintermute's quant team, Jump Crypto, MEV-Boost research groups, and Paradigm-portfolio firms specifically hire for ML+DeFi. The total job count is small but the per-role compensation and learning curve are steep.

## Architecture

```
                ┌─────────────────────────────────┐
                │  React frontend                 │
                │  Tab 1 (augmented):             │
                │  · per-opportunity confidence   │
                │  · expected-value tag           │
                │  · calibration diagnostics tab  │
                └────────────────┬────────────────┘
                                 │ IPC
                ┌────────────────▼────────────────┐
                │  Rust backend (production)      │
                │                                 │
                │  ┌──────────────────────────┐   │
                │  │ Inference layer          │   │
                │  │ (ONNX Runtime / tract)   │   │
                │  └────────────┬─────────────┘   │
                │  ┌────────────▼─────────────┐   │
                │  │ Feature pipeline         │   │
                │  │ (Rust mirror of Python)  │   │
                │  └────────────┬─────────────┘   │
                │  ┌────────────▼─────────────┐   │
                │  │ Tick collector           │   │
                │  │ (writes to SQLite)       │   │
                │  └────────────┬─────────────┘   │
                └───────────────┼─────────────────┘
                                │
                ┌───────────────▼─────────────────┐
                │  SQLite store                   │
                │  (per-tick snapshots)           │
                └───────────────┬─────────────────┘
                                │ batch export
                ┌───────────────▼─────────────────┐
                │  Python training pipeline       │
                │  (offline, weekly)              │
                │                                 │
                │  ┌──────────────────────────┐   │
                │  │ Feature engineering      │   │
                │  └────────────┬─────────────┘   │
                │  ┌────────────▼─────────────┐   │
                │  │ Label generation         │   │
                │  │ (look-forward windows)   │   │
                │  └────────────┬─────────────┘   │
                │  ┌────────────▼─────────────┐   │
                │  │ Model training           │   │
                │  │ (logreg → XGBoost → MLP) │   │
                │  └────────────┬─────────────┘   │
                │  ┌────────────▼─────────────┐   │
                │  │ Calibration              │   │
                │  │ (Platt / isotonic)       │   │
                │  └────────────┬─────────────┘   │
                │  ┌────────────▼─────────────┐   │
                │  │ Export to ONNX           │   │
                │  └──────────────────────────┘   │
                └─────────────────────────────────┘
```

Two halves: **production** (Rust, low-latency inference) and **training** (Python, offline batch). The contract between them is the ONNX model file + the feature spec.

## Milestones

### M-ML.1 — Persistence + collection

- [ ] SQLite schema for per-tick snapshots: `(timestamp, venue_a_price, venue_b_price, ..., gas_price_gwei, derived_features_json)`
- [ ] Background tick collector that writes every Aurix poll to SQLite (additive to in-memory window)
- [ ] Disk budget: ~1MB/hour at 1Hz polling — acceptable for months of collection
- [ ] Backfill from existing in-memory window on first run
- [ ] Goal: ≥7 days of continuous data (~600k ticks) before training begins

### M-ML.2 — Feature engineering

Per-tick feature set (~25 features). Documented in `context/plans/feature-spec.md` alongside this file.

- [ ] Spread features: 6 pairwise spreads (V3-5bps↔V3-30bps, V3-5bps↔V2, V3-5bps↔Sushi, V3-30bps↔V2, V3-30bps↔Sushi, V2↔Sushi), each as absolute USD and as basis points
- [ ] Recent volatility: rolling std of mid-price over 30s, 5min, 30min windows
- [ ] Gas features: current gas price, gas price relative to 1-hour rolling median
- [ ] Time features: time of day (cyclic encoding sin/cos), day of week (one-hot)
- [ ] Liquidity features: V2 reserve sizes, V3 in-range liquidity (when available)
- [ ] Gas-adjusted spread: each pairwise spread minus current gas cost
- [ ] Recent activity: number of swaps observed in the last 60s on each venue
- [ ] (V2) Mempool features (if Vector B is in flight): pending-swap count, total pending volume, recent SEV total
- [ ] Standardisation: zero-mean unit-variance per-feature, computed from training set, applied to inference

### M-ML.3 — Label generation

For each historical tick, look forward N seconds to determine the label.

- [ ] Multi-horizon labels: did the spread persist with the same sign for at least N seconds? (N ∈ {5, 10, 30, 60})
- [ ] For each horizon, also compute "did the gas-adjusted profit remain positive?" (the more useful label — pure persistence isn't profitability)
- [ ] Class balance check: report positive/negative class ratio per horizon; if highly imbalanced, document upsampling strategy
- [ ] Labels stored alongside features in SQLite

### M-ML.4 — Training pipeline (Python)

- [ ] Train/validation/test split by TIME (NEVER random — random splits leak future into past)
  - Train: oldest 60% of data
  - Validation: middle 20%
  - Test: most recent 20% (held out, never touched until final eval)
- [ ] Baseline: logistic regression (sklearn)
- [ ] Stronger: XGBoost (already in skills section; gradient boosting handles heterogeneous tabular features well)
- [ ] Optional advanced: small MLP via PyTorch (3-layer, ~50 hidden units; overkill for this feature size but signals NN literacy)
- [ ] Metrics on validation: AUC, precision/recall at various thresholds, calibration error (ECE — expected calibration error)
- [ ] Hyperparameter search: grid search on validation; final retrain on train+val before test eval
- [ ] Export every trained model + its eval report to a runs directory

### M-ML.5 — Calibration

- [ ] Platt scaling (logistic regression on top of model logits) — fast, simple, usually sufficient
- [ ] Isotonic regression alternative — non-parametric, more flexible
- [ ] Reliability diagram: x-axis = predicted probability bin, y-axis = observed positive rate. A perfectly-calibrated model lies on the diagonal.
- [ ] ECE before vs after calibration, reported in the model card
- [ ] Calibrated model exported to ONNX with the calibration layer baked in

### M-ML.6 — Inference layer in Rust

- [ ] ONNX runtime selection: `tract-onnx` (pure Rust, no C++ dependency, slightly slower) vs `ort` (FFI to ONNX Runtime, faster, more deps). Recommendation: `tract` for simpler build, `ort` if latency becomes the bottleneck.
- [ ] Feature pipeline mirrored in Rust — exactly the same transformations as Python, otherwise predictions diverge
- [ ] Cross-runtime parity test: pick 100 historical observations, compute features in both Python and Rust, run inference in both, predictions must match within 1e-5 absolute tolerance
- [ ] Latency benchmark: feature compute + inference, p50 and p99
- [ ] Model loading on startup; model file path configurable (so retrain → swap file → restart updates production)

### M-ML.7 — Frontend integration

- [ ] Per-opportunity confidence badge: "73% likely to persist 30s" (with the 30s being the chosen primary horizon)
- [ ] Multi-horizon display on hover: "10s: 91% / 30s: 73% / 60s: 41%"
- [ ] Gas-adjusted expected value tag: "with current gas, expected net +$2.40 IF executed" (computes spread × confidence − gas)
- [ ] New diagnostics tab: reliability diagram updated daily, cumulative AUC over the last 7 days, feature importance plot
- [ ] Visual treatment: high-confidence-and-positive-EV opportunities highlighted; low-confidence flicker dimmed

## Validation Strategy

| Layer | Method | Acceptance |
|---|---|---|
| Time-split | Train on oldest 60%, val on middle 20%, test on most recent 20% | Test AUC > 0.65 for "this works"; > 0.75 for "this is real" |
| Calibration | Reliability diagram on test set | ECE < 0.05 (predicted 70% should occur 65–75% of the time) |
| Cross-runtime | 100 obs through both Python and Rust pipelines | Predictions match within 1e-5 |
| Stability | Retrain weekly; compare test AUC week-over-week | Drift < 0.05 AUC week-over-week (regime change red flag if larger) |
| Sanity | "Always predict majority class" baseline AUC = 0.5 | Trained model must beat baseline by ≥0.10 |

## Open Decisions

- **Training framework choice:** scikit-learn + XGBoost (pragmatic, fast iteration, broad familiarity) vs burn (Rust-native, dogfoods OSS contribution) vs PyTorch (most ML coursework experience). Recommendation: sklearn + XGBoost for V1 (fastest path to a calibrated model and broadest reproducibility), then port the inference path to burn for V2 (the "I implemented end-to-end in burn" line is itself a hiring signal worth the effort).
- **Inference runtime:** `tract` (pure Rust, simpler build) vs `ort` (FFI binding to ONNX Runtime, faster). Recommendation: `tract` initially; switch to `ort` only if latency benchmarks demand it.
- **Primary horizon:** 30s (long enough to be meaningful, short enough to have plenty of observations) vs 60s (more practically execution-relevant) vs 5s (high signal-to-noise but trivial to predict). Recommendation: 30s primary, all four shown on hover.
- **Online vs batch retrain:** batch weekly is simpler and the "model card" framing fits naturally; online learning is cooler but harder to validate. Recommendation: weekly batch for V1.
- **Label type:** pure persistence ("spread sign unchanged") vs profitability ("gas-adjusted spread remains positive"). Recommendation: train both, ship the profitability variant in the UI; persistence is a useful intermediate diagnostic.
- **Feature scope:** market-state only (V1) vs include mempool features once Vector B ships (V2). Recommendation: V1 ships market-state; V2 adds mempool when Vector B's data is available.

## Dependencies / Blocked-by

- Status Decision must be "revive"
- Persistence layer (shared with Vector A's M2.0) is the prerequisite — implies M-ML.1 = M2.0 with extra schema
- Need ≥7 days of continuous collection before training is meaningful → calendar-time blocking
- Optional: Vector B's mempool features (V2 of this vector)

## Out of Scope

- Real execution / capital deployment (read-only principle non-negotiable)
- Multi-pair models (WETH/USDC only for V1)
- Cross-asset features (BTC price, equity vol, macro indicators)
- Online learning (batch retrain weekly only)
- Adversarial robustness analysis (no specific attack model)
- Counterfactual evaluation ("what if I had acted on every prediction") — that's an extension via Vector A's backtester chassis
- Multi-task models (we predict survival; we don't predict the spread magnitude evolution)

## Hiring Signal Payoff

Resume bullet candidates:

- "Trained and calibrated an XGBoost arbitrage-survival classifier on N days of real DEX market data, achieving AUC 0.7X on a held-out 7-day test set; deployed via ONNX Runtime in Rust with sub-ms inference."
- "Built end-to-end ML pipeline: per-tick feature engineering, time-split train/val/test, Platt-calibrated probability outputs, cross-runtime parity verification between Python training and Rust inference."
- "Calibration error (ECE) below 0.05; predicted 70% confidence opportunities materialise 67% of the time on held-out data."

Interview talking points:
- "Why I chose time-split over random split — and the look-ahead bias that random splits introduce in time series."
- "Calibration matters more than raw AUC for a probability that's used in expected-value calculations downstream."
- "Why I cross-validated the Rust inference path against Python — feature transformation drift is the #1 silent failure mode in deployed ML."
- "How model performance degrades during high-volatility periods, and why that's a regime-change problem rather than a model-quality problem."
- "The combination of ML and DeFi is rare — here's why I think it's the right cross-section."
