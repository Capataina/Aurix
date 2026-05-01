# Vector Preparation Path

## Who This Path Is For

You've decided to revive Aurix and ship one of the three vectors authored in `context/plans/`. You want to load the right context before starting the implementation work — the prerequisite theory, the relevant project systems, and the specific plan file.

This path is branched: one branch per vector. Pick the branch that matches your vector commitment.

## What This Path Assumes

- Completion of `project-systems-path` (you understand the existing codebase)
- Understanding of the relevant domain theory (the path will say what that means for each vector)
- Acceptance of the Status Decision (revive) — you've already committed to working on Aurix

## Vector A — V3 LP Backtester

**Read in this order:**

- [ ] `concepts/core/amm-mechanics-v2-and-v3.md` (V3 sections — re-read carefully)
- [ ] `concepts/core/liquidity-providers-and-impermanent-loss.md` (the IL math is what you're about to validate)
- [ ] `concepts/advanced/uniswap-v3-tick-mathematics.md` (the deep math)
- [ ] `materials/amm-foundational-resources.md` → **Uniswap V3 whitepaper** (read sections 2-6 carefully; section 6 is the load-bearing math)
- [ ] `context/plans/vector-a-v3-lp-backtester.md` (the actual plan)
- [ ] `context/notes/wire-convention.md` (you'll be extending the IPC contract)
- [ ] `context/systems/runtime-foundation.md` (you'll be adding to the persistence layer)

**Practice before you start:**

- [ ] `exercises/foundations/amm-constant-product-by-hand.md`
- [ ] `exercises/foundations/impermanent-loss-worked-example.md`
- [ ] `exercises/core/decode-sqrtpricex96.rs` — gets your hands on the actual V3 decode

**By the time you start coding M2.0 (persistence layer), you should be able to:**
- Convert ticks to prices and back without looking it up
- Explain Q64.96 fixed-point representation in your own words
- Walk through fee distribution per swap (which LPs earn what fraction of the fee)
- Sketch the validation harness mentally before implementing it

**Estimated prep time:** 8-15 hours.

## Vector B — MEV Detector

**Read in this order:**

- [ ] `concepts/domain-patterns/mev-and-transaction-ordering.md`
- [ ] `concepts/domain-patterns/the-mempool-public-vs-private.md`
- [ ] `concepts/advanced/mempool-mev-detection-mechanics.md`
- [ ] `materials/mev-resources.md` → Flashbots docs + libMEV + Eigenphi
- [ ] `materials/ethereum-internals-resources.md` → Mastering Ethereum chapters on transaction lifecycle
- [ ] `context/plans/vector-b-mev-detector.md`
- [ ] `context/systems/arbitrage-market-data.md` (you'll extend this with mempool reads)

**Practice before you start:**

- [ ] `exercises/foundations/sandwich-attack-economics.md`

**By the time you start coding M-MEV.1 (WebSocket subscription), you should be able to:**
- Explain calldata encoding for V2 and V3 router functions
- Articulate the difference between sandwichable and JIT-liquidity-targeted swaps
- Sketch the SEV (sandwich extractable value) calculation for a known mempool tx
- Identify why sub-10ms latency is achievable on a self-hosted node but not on free tier RPC

**Estimated prep time:** 6-12 hours.

## Vector C — ML Arbitrage-Survival Classifier

**Read in this order:**

- [ ] `concepts/advanced/statistical-primitives-for-risk-modelling.md` (vol, correlation, basic stats)
- [ ] `concepts/advanced/ml-for-market-microstructure.md`
- [ ] `materials/ml-for-finance-resources.md`
- [ ] `materials/quant-finance-resources.md`
- [ ] `context/plans/vector-c-ml-arbitrage-survival.md`
- [ ] `context/systems/arbitrage-analytics.md` (you'll extend this with predictions)

**Pre-existing knowledge you should already have (from your tinygrad LSTM and burn A-FINE contributions):**
- ONNX export and runtime
- Standard sklearn / XGBoost / PyTorch training workflows
- Cross-runtime ML deployment

**What's new in this vector:**
- Time-split validation (you cannot use random splits on time series — that leaks future into past)
- Calibration (Platt scaling, isotonic regression)
- The specific gotchas of financial data (regime change, fat tails, autocorrelation)

**Practice before you start:**

- [ ] `exercises/foundations/sandwich-attack-economics.md` (helps with feature ideation)
- [ ] Optional: re-read your A-FINE PR — the same calibration discipline applies

**By the time you start coding M-ML.1 (persistence + collection), you should be able to:**
- Explain why time-split validation is mandatory for financial ML
- Articulate the difference between AUC and calibration error and why both matter
- Sketch the feature set you'll engineer (without committing to specifics yet)
- Identify which ONNX runtime (`tract` vs `ort`) makes more sense for Aurix and why

**Estimated prep time:** 10-20 hours (less if you have recent ML practice; more if you're rusty).

## Cross-Vector Dependencies

If you're shipping more than one vector over time, the order matters:

1. **A → C** is natural. Vector A's persistence layer (M2.0) is also Vector C's prerequisite (M-ML.1 is the same thing). The data Vector A backtests against is also the training data for Vector C.
2. **B → C** is natural. Vector B's mempool features become Vector C's input features in the V2 model.
3. **B is independent of A.** They don't share code paths.

The recommended sequence if you commit to all three over time: **A first (4-6 weeks)** → **C (4-8 weeks, builds on A's persistence)** → **B (3-5 weeks, independent)**.

## What To Do After Prep

Open the plan file you read, scroll to the milestones section, and start ticking checkboxes against M2.0 (Vector A) / M-MEV.1 (Vector B) / M-ML.1 (Vector C). The first milestone for each vector is intentionally the foundational one — don't skip it.
