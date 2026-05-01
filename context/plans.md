# Plans

Project plans live as files in `plans/`. Each plan is a proposed direction with goal, milestones (with checkboxes), validation strategy, open decisions, dependencies, what's out of scope, and the hiring-signal payoff. Plans are mutated as work progresses (tick checkboxes inline) and removed once their criteria are fully met.

A plan being on this index does NOT mean it has been accepted. Acceptance happens when a Status Decision (or equivalent direction call) authorises work to begin. Until then, plans sit here as written-up options.

## Active plans

- [vector-a-v3-lp-backtester](plans/vector-a-v3-lp-backtester.md) — Uniswap V3 LP backtester with exact Q64.96 tick math, validated against on-chain reference positions, with multi-strategy comparison. Closes the resume credibility gap on Tab 2; defends the AMM Mathematics skill claim. Audience: quant LP desks, Uniswap Labs.
- [vector-b-mev-detector](plans/vector-b-mev-detector.md) — Mempool MEV detector subscribing to pending swap transactions, decoding calldata, simulating price impact, classifying intent (sandwich / frontrun / JIT / liquidation). Adds latency-sensitive systems engineering to the portfolio. Audience: Flashbots, Jump Crypto, Wintermute MEV team.
- [vector-c-ml-arbitrage-survival](plans/vector-c-ml-arbitrage-survival.md) — Calibrated ML classifier predicting arbitrage opportunity survival, with per-tick feature engineering, time-split training, and ONNX deployment to Rust inference. Cross-stack hiring signal (DeFi + ML, rare combination). Audience: crypto-quant desks.

## Completed plans

_(none yet — newly-created index 2026-05-01)_

## Status

All three vectors are blocked on the open Status Decision (revive / pause / decommission, vault Work file pending since 2026-04-19). Plans authored to make the "what would revive look like" question concrete; selecting a vector does not commit to all three.
