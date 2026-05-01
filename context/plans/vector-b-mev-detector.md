---
title: "Vector B — Mempool MEV Detector"
status: proposed
created: 2026-05-01
vector: B
hiring-audience: Flashbots, Jump Crypto, Wintermute MEV team, Paradigm-portfolio firms, MEV-Boost ecosystem
estimated-effort: 3-5 weeks focused
depends-on: independent of A and C; could share persistence layer if A is in flight
---

# Vector B — Mempool MEV Detector

## Goal

Subscribe to Ethereum's public mempool, decode pending swap transactions before they land in a block, locally simulate their price impact, classify MEV intent (sandwich attempts, frontrunning, JIT liquidity, liquidations), and surface sub-10ms classification latency. Aurix never executes — the detector is purely observational, consistent with the read-only principle.

## Why This Vector

- **Adds latency-sensitive systems engineering to the portfolio.** Aurix as currently scoped is a 1 Hz polling loop. A mempool watcher with sub-10ms classification puts Aurix in a different performance regime entirely.
- **Pairs naturally with Nyquestro.** Nyquestro covers the matching-engine / market-making side; Vector B covers the mempool / MEV side. Together they signal "I understand both sides of order flow — on-chain and off-chain."
- **Hiring signal for the highest-paying crypto roles.** Flashbots, Jump Crypto's crypto desk, Wintermute's MEV team, and most Paradigm-portfolio firms hire specifically for mempool literacy. Most "DeFi devs" have never opened a WebSocket against `eth_subscribe newPendingTransactions`. This puts you in the small fraction that has.
- **Conceptually independent of Vector A.** Can be developed in parallel without merge conflicts. Could share the persistence layer if both are in flight.

## Architecture

```
                ┌─────────────────────────────────┐
                │  React frontend                 │
                │  Tab 6: Mempool                 │
                │  · live tx feed                 │
                │  · classification badges        │
                │  · SEV / latency strip          │
                └────────────────┬────────────────┘
                                 │ IPC (high frequency)
                ┌────────────────▼────────────────┐
                │  Rust backend                   │
                │                                 │
                │  ┌──────────────────────────┐   │
                │  │ Latency instrumentation  │   │
                │  └────────────┬─────────────┘   │
                │  ┌────────────▼─────────────┐   │
                │  │ Classifier               │   │
                │  │ (sandwich/frontrun/JIT/  │   │
                │  │  liquidation/normal)     │   │
                │  └────────────┬─────────────┘   │
                │  ┌────────────▼─────────────┐   │
                │  │ Pool state simulator     │   │
                │  │ (V2 + V3 sim + SEV calc) │   │
                │  └────────────┬─────────────┘   │
                │  ┌────────────▼─────────────┐   │
                │  │ Calldata decoder         │   │
                │  │ (Universal Router, V2/V3 │   │
                │  │  routers, Sushi, 1inch)  │   │
                │  └────────────┬─────────────┘   │
                │  ┌────────────▼─────────────┐   │
                │  │ Mempool subscription     │   │
                │  │ (eth_subscribe over WS)  │   │
                │  └────────────┬─────────────┘   │
                └───────────────┼─────────────────┘
                                │ WebSocket
                ┌───────────────▼─────────────────┐
                │  Ethereum node                  │
                │  newPendingTransactions stream  │
                └─────────────────────────────────┘
```

Six layers:
1. **WebSocket subscription** — receive pending tx hashes (or full bodies if available)
2. **Calldata decoder** — turn raw bytes into structured `SwapIntent`
3. **Pool state simulator** — predict execution price + price impact
4. **Classifier** — assign each tx to one of {sandwich, frontrun, JIT, liquidation, normal}
5. **SEV calculator** — for sandwichable swaps, compute optimal extractable value
6. **Latency instrumentation** — per-tx timing histograms

## Milestones

### M-MEV.1 — WebSocket subscription

- [ ] Connect to a node with `eth_subscribe newPendingTransactions` support (free options: Alchemy, QuickNode, Ankr)
- [ ] Parse incoming pending tx hashes
- [ ] Fetch full tx body via `eth_getTransactionByHash` (or use `newPendingTransactionsWithBody` if available — Alchemy supports a variant)
- [ ] Reconnection logic with exponential backoff
- [ ] Latency benchmark: hash-received → body-fetched p50/p99 (target: p99 < 100ms)

### M-MEV.2 — Calldata decoder

- [ ] Recognise Uniswap V2 router 02 (`swapExactTokensForTokens`, `swapTokensForExactTokens`, `swapExactETHForTokens`, etc. — 8 variants)
- [ ] Recognise Uniswap V3 SwapRouter / SwapRouter02 (`exactInputSingle`, `exactInput`, `exactOutputSingle`, `exactOutput`)
- [ ] Recognise SushiSwap router (V2-fork ABI is identical to V2)
- [ ] Recognise Universal Router (Uniswap's newer multi-protocol router with command encoding)
- [ ] Recognise 1inch aggregator (different ABI per version)
- [ ] Output: structured `SwapIntent { router, input_token, output_token, amount_in, amount_out_min, recipient, deadline, path }`
- [ ] Test fixtures: 50+ historical mainnet txs, manually labelled with expected SwapIntent, decoded must match
- [ ] Unrecognised tx → tagged as "non-swap" or "unknown router" (not a failure)

### M-MEV.3 — Pool state simulator

- [ ] Maintain cached pool state for top-N WETH pairs (refresh each block via the existing Aurix fetch layer)
- [ ] V2 simulation: apply `x * y = k` math to compute output amount + new pool state
- [ ] V3 simulation: walk ticks accounting for liquidity changes at each crossed boundary
- [ ] Output for any swap intent: predicted execution price, price impact %, output amount
- [ ] Cross-check: simulator output for a recently-confirmed historical swap matches the actual on-chain execution within rounding

### M-MEV.4 — Classifier

- [ ] **Sandwich attempt detection:** look for the pattern (large swap with high slippage tolerance OR `amount_out_min` set very low) — these are the SANDWICHABLE candidates
- [ ] **Sandwich execution detection:** look for tx pairs where two swaps from the same sender bracket a third (tx_n: swap_A→B, tx_n+1: victim swap_A→B, tx_n+2: swap_B→A from same sender as tx_n) within a small block window
- [ ] **Frontrunning detection:** swap with priority fee significantly above current baseline, observed shortly after another tx that creates an opportunity (e.g. an oracle update, a large user swap)
- [ ] **JIT (just-in-time) liquidity detection:** mint LP position immediately before a large swap, burn immediately after — within 1-2 blocks
- [ ] **Liquidation detection:** Aave / Compound / Maker liquidator function calls (separate ABI set, well-documented)
- [ ] **Normal swap:** everything else — the residual class
- [ ] Each classification carries a confidence score (rule-based for V1; can become ML-based for V2)

### M-MEV.5 — Sandwich extractable value (SEV) calculator

- [ ] For each sandwichable swap, compute the optimal front-run size (the size that maximises sandwich profit given the victim's `amount_out_min`)
- [ ] Predicted SEV in USD = (front_run_back_run_profit) − (front_run_gas + back_run_gas)
- [ ] Counter: hourly rolling SEV total (sum of "would-have-been-extractable" value across all sandwichable txs)
- [ ] Counter: hourly rolling extracted SEV (sum of value actually extracted by detected executed sandwiches)
- [ ] The gap between "extractable" and "extracted" is itself an interesting metric (efficiency of MEV market)

### M-MEV.6 — Latency instrumentation

- [ ] Per-tx timing markers: `t0` (hash received), `t1` (body fetched), `t2` (decoded), `t3` (simulated), `t4` (classified)
- [ ] Histogram of (t4 - t0) — full pipeline latency
- [ ] p50, p95, p99 reported in UI
- [ ] Target: p99 < 100ms end-to-end (achievable on a non-self-hosted node); p99 < 10ms end-to-end (achievable on a self-hosted Geth/Erigon node — stretch goal worth flagging in resume)

### M-MEV.7 — Frontend

- [ ] New tab "Mempool" with:
  - Live feed of classified pending txs (max 100 visible, scrollable buffer)
  - Per-tx card: classification badge, simulated price impact %, gas tip vs baseline, SEV (if applicable)
  - Filter: show all / sandwich-targets only / executed-sandwiches only / liquidations only
  - Hourly counters strip: txs by class, total SEV (extractable), total SEV (extracted)
  - Latency badge: "p99: 8ms" with histogram on hover
- [ ] Sound (optional, off by default): ping when a high-SEV opportunity appears

## Validation Strategy

| Layer | Method | Acceptance |
|---|---|---|
| Decoder | 50 historical mainnet txs manually labelled | 48/50 decoded correctly; document the 2 misses |
| Simulator | Recently-confirmed swaps replayed | Output matches actual execution within 0.1% |
| Classifier (sandwich) | Replay 10 known historical sandwich attacks (sourced from Eigenphi, libMEV, Flashbots dashboard) | Classifier flags all 10 correctly; <5% false positive rate on a control sample of 100 normal swaps |
| SEV calculator | For 10 known executed sandwiches, compare predicted SEV to actual extracted value | Mean error <10%, max error <30% |
| Latency | Continuous monitoring | p99 < 100ms via free public WS; document the path to <10ms via self-host |

## Open Decisions

- **WebSocket provider:** Alchemy free tier (easiest, capped at N pending tx/sec) vs QuickNode (similar) vs self-hosted Erigon archive node (hardest, free, real). Recommendation: Alchemy for V1; document the path to self-host as the latency story for V2.
- **Token universe for simulation:** WETH pairs only (~10 most-active) vs every ERC-20 (~thousands, requires generic ABI handling). Recommendation: WETH pairs only — keeps simulator tractable.
- **Classifier approach:** rule-based heuristics (V1, fast to ship, transparent) vs ML model (V2, harder, more nuanced — bridges into Vector C). Recommendation: rule-based for V1; ML model can reuse Vector C's training infrastructure.
- **Bundle simulation via Flashbots `eth_callBundle`:** include in M-MEV.5 vs defer to V2. Recommendation: defer — adds Flashbots dependency and complicates simulation; the read-only "we don't bundle" framing is fine for V1.
- **Persistence:** stream-only (no historical replay capability) vs persist-everything (10MB/hour fast). Recommendation: persist with rolling 7-day window; enables historical replay for validation and Vector C feature engineering.

## Dependencies / Blocked-by

- Status Decision must be "revive"
- Independent of Vector A — no shared code paths beyond optional persistence layer
- Could share persistence with Vector A's M2.0 if both are in flight (single SQLite, separate tables)
- Could feed Vector C's feature engineering (mempool flow features improve the classifier — Vector C model becomes "given mempool state + spread state, predict survival")

## Out of Scope

- Actually submitting MEV transactions or Flashbots bundles (read-only principle is non-negotiable)
- L2 mempool watching (mainnet only for V1)
- Bot economics modelling (we classify and measure SEV; we don't model the bot's optimisation problem)
- Cross-block MEV (atomic arbitrage that spans multiple blocks via specific orderings)
- Generalised front-run detection (only swap-context for V1)
- Re-org-aware classification (we classify on the public mempool; if a tx is later re-orged out, the classification stands as observed)

## Hiring Signal Payoff

Resume bullet candidates:

- "Built a mempool MEV classifier in Rust with sub-10ms p99 classification latency, validated against 10 historical sandwich attacks; identified [observation from data] across [N] hours of mainnet observation."
- "Implemented calldata decoders for V2/V3 routers, Universal Router, and 1inch aggregator with structured `SwapIntent` extraction; replayed against 50 historical mainnet txs at 96% decode accuracy."
- "Pool state simulator predicting V2/V3 execution price within 0.1% of on-chain truth, used as the substrate for SEV (sandwich extractable value) computation."

Interview talking points:
- "Here's the calldata format the Universal Router uses, and here's why decoding it requires walking a command stream rather than just reading function selectors."
- "Here's the difference between sandwichable (high slippage tolerance) and JIT-liquidity-targeted (large size) swaps."
- "Here's why my SEV estimate for [specific historical sandwich] was off by 12% — turned out to be the back-run hitting a different liquidity surface than my simulator assumed."
- "Sub-10ms p99 came from [specific implementation choice — pool state cache, lock-free read path, etc.]."
