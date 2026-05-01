# Vector Roadmap

## Current State

Three vectors have been authored as direction proposals for Aurix's next phase. Each is documented in detail in `context/plans/`. None has been committed to — all three are blocked on the open Status Decision (revive / pause / decommission, pending since 2026-04-19).

This file gives the high-level map; the plan files have the milestone-level detail.

## The Three Vectors

| Vector | What it adds | Audience for hiring signal | Effort | Plan file |
|---|---|---|---|---|
| **A** | V3 LP backtester with verified tick math + statistical analysis | Quant LP desks, Uniswap Labs, DeFi-aware trading firms | 4-6 weeks | `context/plans/vector-a-v3-lp-backtester.md` |
| **B** | Mempool MEV detector with calldata decoding + sandwich classification + sub-10ms latency | Flashbots, Jump Crypto, Wintermute MEV team, Paradigm-portfolio | 3-5 weeks | `context/plans/vector-b-mev-detector.md` |
| **C** | ML arbitrage-survival classifier with calibrated probabilities + Rust ONNX deployment | Crypto-quant desks (the rare ML+DeFi cross-section) | 4-8 weeks | `context/plans/vector-c-ml-arbitrage-survival.md` |

## Why Three?

Each vector serves a different audience and signals different engineering substance. They're complementary, not redundant:

- **Vector A** signals AMM math literacy, numerical correctness, validation discipline
- **Vector B** signals low-latency systems engineering, mempool literacy, MEV ecosystem awareness
- **Vector C** signals ML deployment chops, calibration discipline, the rare DeFi+ML cross-section

You don't have to do all three. You can ship one and have a much stronger Aurix; ship two and have a portfolio piece; ship three and have a substantial body of work. Pick by audience.

## Recommended Order (If Doing Multiple)

If you commit to more than one vector, the order matters because of shared dependencies:

```
                ┌───────────────────────────┐
                │   Vector A — V3 LP        │
                │   Backtester              │
                │                           │
                │   Includes M2.0:          │
                │   Persistence layer       │
                └─────────────┬─────────────┘
                              │
                              │ persistence is shared
                              │
                ┌─────────────▼─────────────┐
                │   Vector C — ML Signal    │
                │                           │
                │   Needs persisted data    │
                │   for training            │
                └───────────────────────────┘

                ┌───────────────────────────┐
                │   Vector B — MEV Detector │
                │                           │
                │   Independent of A and C  │
                │   Could share persistence │
                │   if all three planned    │
                └───────────────────────────┘
```

**Recommended sequence: A first → C second (builds on A's persistence) → B third (independent).**

Estimated total: 11-19 weeks if committing to all three. Realistic completion: 4-6 months including breaks and other commitments.

## Why Not All At Once

Splitting attention across three vectors produces three half-finished projects. Each vector has a hiring-signal payoff that requires shipping the WHOLE thing — a partial Vector A is worse than no Vector A (because the resume bullet would still write a check the repo doesn't cash).

**The discipline: commit to one at a time, ship it, then choose the next.**

## What Each Vector Adds To The Resume

Once shipped, each vector enables specific resume bullet upgrades:

### After Vector A

> "Implemented and validated Uniswap V3 LP backtester with exact Q64.96 tick math against 5 on-chain reference positions; per-swap fee distribution within 0.5% of collected ground truth across 30+ days of mainnet data."

> "Designed strategy-comparison framework over 50+ LP configurations with Sharpe / drawdown / fee-IL decomposition."

### After Vector B

> "Built a mempool MEV classifier in Rust with sub-10ms p99 classification latency, validated against 10 historical sandwich attacks."

> "Implemented calldata decoders for V2/V3 routers, Universal Router, and 1inch aggregator with structured SwapIntent extraction at 96% decode accuracy across 50 mainnet samples."

### After Vector C

> "Trained and calibrated an XGBoost arbitrage-survival classifier on N days of real DEX market data, achieving AUC 0.7X on a held-out 7-day test set; deployed via ONNX Runtime in Rust with sub-ms inference."

> "Cross-stack ML pipeline: per-tick feature engineering, time-split train/val/test, Platt-calibrated probabilities, cross-runtime parity verification between Python training and Rust inference."

## The Status Decision Gating

All three vectors are blocked on the Status Decision. The vault Work file `Projects/Aurix/Work/Status Decision.md` has been pending since 2026-04-19 (~12 days as of 2026-05-01).

The decision has three honest answers:

1. **Revive** — Aurix becomes active again. Vector A is the natural first commitment given it closes the resume credibility gap and unblocks downstream tabs.

2. **Pause with explicit resume trigger** — record what would have to change for Aurix to come back (e.g. "after NeuroDrive M7 ships"). Update vault status. Don't start a vector.

3. **Decommission cleanly** — flip _Overview status to decommissioned-with-reason, update Projects/Index.md, archive without deletion. Don't start a vector. Rewrite resume bullet to scope down or remove entirely.

The vectors exist as authored plans regardless of which path is chosen — they're optionality. If revive is chosen, they're the menu. If pause or decommission, they're documentation of what could have been (and could resume from later).

## When To Update This File

- When the Status Decision resolves → reflect the chosen path here
- When a vector is committed to → mark it "In Progress" and add target dates
- When a vector ships → mark it "Done" and update the resume payoff section to past tense
- When a new vector is added (additive permission per `context/plans.md`) → list it here

## Related Files

- `context/plans/vector-a-v3-lp-backtester.md`
- `context/plans/vector-b-mev-detector.md`
- `context/plans/vector-c-ml-arbitrage-survival.md`
- `context/plans.md` — the plan index
- `project/evolution/five-tab-vision-vs-current-reality.md` — what the vectors fit into
- `Projects/Aurix/Work/Status Decision.md` (LifeOS vault) — the gating decision
