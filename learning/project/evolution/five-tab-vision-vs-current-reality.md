# Five-Tab Vision vs Current Reality

## Current State

Aurix's README describes a **five-tab DeFi analytics platform** covering arbitrage, LP backtesting, wallet tracking, gas intelligence, and risk modelling. The codebase has **one tab partially implemented**: Tab 1 (Arbitrage Scanner) at roughly Milestone 1.4 of the README's own 5-milestone progression.

| Tab | README description | Status |
|---|---|---|
| 1 — Arbitrage Scanner | Cross-DEX price comparison with gas-adjusted profit | **Partial** — live scanner runs (M1.1-1.3 done), opportunity feed/threshold/historical chart unchecked (M1.4-1.5) |
| 2 — Liquidity Pool Analyser | Uniswap V3 backtesting with tick math, IL | **Not started** — no `src-tauri/src/backtest/`, no `src/features/lp/` |
| 3 — Wallet & Position Tracker | Read-only on-chain position monitoring | **Not started** — no wallet input, no position decoder |
| 4 — Gas Price Monitor & Predictor | Historical gas analysis + recommendations | **Not started** — gas read live, never persisted |
| 5 — Token Correlation & Risk Dashboard | Correlation, volatility, VaR | **Not started** — no statistical engine |

This file documents the gap between vision and reality, and what bridging it would actually require.

## Why The Gap Matters

The gap is the project's primary credibility issue. The Aurix resume bullet says:

> "Building an on-device Ethereum analytics app targeting cross-DEX arbitrage, Uniswap V3 LP backtesting, wallet tracking, gas prediction, and risk modelling."

A senior engineer who clicks through to GitHub sees: one tab (Tab 1, partial) and four READMEs. The bullet is writing checks the repo doesn't fully cash. Either the bullet needs scoping down to current reality, or the repo needs to ship enough to make it true.

The vault's `_Overview.md` explicitly cautions: *"Do not describe Aurix as a five-tab platform in any downstream context."* This is honest — describing Aurix that way creates a credibility problem when verification happens.

## What Bridging The Gap Looks Like

### Path A — Honest scope-down

Rewrite the resume bullet to match current reality:

> "A real-time cross-DEX arbitrage scanner with raw JSON-RPC + handcrafted ABI encoding (no ethers-rs), BigUint sqrtPriceX96 decoding, four concurrent venues at 1Hz, and a TypeScript insight engine with rolling-window persistence detection. Target architecture supports four additional analytical tabs (LP backtesting, wallet tracking, gas intelligence, risk modelling)."

This is honest. It's what's actually built. The "target architecture" framing makes the vision visible without promising delivery.

### Path B — Ship Tab 2 to make the bullet truer

Vector A (V3 LP backtester) explicitly closes the credibility gap on Tab 2. After Vector A ships:

- Tab 1 stays at Milestone 1.4 (partial)
- Tab 2 is shipped with verified tick math
- The resume bullet becomes "Building... with arbitrage [shipped] and LP backtesting [shipped]; wallet tracking, gas, risk in roadmap."

This is the strongest path. It moves the project from "1/5 with credibility issue" to "2/5 with the rest in roadmap."

### Path C — Pause Aurix and let the bullet be honest about scope

If you're committing to NeuroDrive or another project, the honest move is:

- Rewrite the bullet to scope down (Path A)
- Mark the vault `_Overview.md` status as "paused"
- Document the resume trigger condition (e.g. "after NeuroDrive M7 ships")

This isn't failure — it's prioritisation. Five projects can't all be active simultaneously.

## What's Done vs What's Planned (Per Tab)

### Tab 1 — Arbitrage Scanner

Done:
- M1.1 Project skeleton + first DEX integration
- M1.2 Concurrent multi-DEX fetching (4 venues, tokio::join!)
- M1.3 Spread detection + gas-adjusted profit estimation

Partial:
- M1.4 Dashboard (price cards + chart done; opportunity feed, configurable threshold, connection status missing)

Not started:
- M1.5 Polish (historical chart, per-DEX statistics, exportable log, demo recording)

Path forward: ship M1.4 (~1 week), then M1.5 (~1-2 weeks). After M1.5, Tab 1 is a real product.

### Tab 2 — Liquidity Pool Analyser

Done: nothing.

Plan: Vector A (`context/plans/vector-a-v3-lp-backtester.md`) is the explicit Tab 2 implementation plan with M2.0-2.6. Estimated 4-6 weeks.

### Tab 3 — Wallet & Position Tracker

Done: nothing.

Plan: not yet written. Would require:
- Wallet address input UI
- Token balance fetcher (multiple ERC-20s)
- DeFi protocol decoders (Uniswap LP positions, Aave positions, Compound positions)
- USD valuation layer using Tab 1's price feeds

Estimated 3-5 weeks if built linearly.

### Tab 4 — Gas Price Monitor & Predictor

Done: gas is read live every tick (used by Tab 1 for gas-adjusted profitability).

Plan: not yet written. Would require:
- Persistent gas history (depends on Vector A's M2.0 persistence layer)
- Day-of-week × hour-of-day aggregation
- Heatmap visualisation
- Optional: ML predictor (which would tie into Vector C's infrastructure)

Estimated 2-4 weeks if persistence is already in place.

### Tab 5 — Token Correlation & Risk Dashboard

Done: nothing.

Plan: not yet written. Would require:
- Multi-token price collection (currently only WETH/USDC)
- Statistical engine (correlation, rolling vol, VaR)
- Portfolio definition UI
- Risk dashboard with Sharpe, drawdown, etc.

Estimated 3-5 weeks if persistence and multi-pair fetching are already in place.

## Cross-Tab Dependencies

Notice the pattern: most tabs depend on **persistence**. Tab 4 needs persisted gas history. Tab 5 needs persisted multi-token prices. Even Tab 1's M1.5 (historical chart) needs persistence.

This is why Vector A's M2.0 (persistence layer) is the most-impactful single milestone in the entire roadmap. It unblocks 4 of 5 tabs and fixes Gap 1 (no persistence) which is the highest-priority known issue.

```
M2.0 Persistence Layer (Vector A)
   │
   ├── unblocks Tab 1 M1.5 (historical opportunity chart)
   ├── unblocks Tab 2 backtesting (Vector A core)
   ├── unblocks Tab 4 gas history
   ├── unblocks Tab 5 historical price data
   └── unblocks Vector C (training data collection)
```

If you commit to ANY single thing, M2.0 is the highest-leverage choice.

## Trade-Off: Depth vs Breadth

There's a real tension between:

- **Going deep on one tab** (e.g. Vector A makes Tab 2 genuinely impressive) → strong hiring signal but project still looks "narrow"
- **Going broad across tabs** (e.g. ship M1.5 + minimal M3 + minimal M4 + minimal M5) → project looks "complete" but each tab is shallow

For hiring purposes, **depth wins**. A genuinely deep Tab 2 (validated tick math, statistical analysis) signals more than five shallow tabs. The five-tab vision is OK as roadmap; what matters is shipping at least one tab that demonstrates real engineering substance.

The right strategy: ship Tab 1 M1.5 (small effort, makes Tab 1 demoable), then go deep on Tab 2 via Vector A. Leave Tabs 3-5 in the roadmap.

## How The Vault Tracks This

The LifeOS vault has structured documentation per Aurix tab:
- `Projects/Aurix/Roadmap/LP Backtesting.md` — design space for Tab 2
- `Projects/Aurix/Roadmap/Wallet Tracker.md` — Tab 3
- `Projects/Aurix/Roadmap/Gas Intelligence.md` — Tab 4
- `Projects/Aurix/Roadmap/Risk Modelling.md` — Tab 5

These are design notes, not implementation plans. They describe what each tab would look like if built, what the technical challenges are, and what dependencies exist. Read them when scoping work on a specific tab.

## When To Update This File

- When Tab 1 M1.5 ships → update Tab 1 status
- When Vector A ships → update Tab 2 status, recompute "what's done"
- When the resume bullet is updated (Path A or Path B) → reflect that here
- When the Status Decision resolves (revive/pause/decommission) → reflect that here

## Related Files

- `project/evolution/vector-roadmap.md` — the three-vector path forward
- `context/plans/vector-a-v3-lp-backtester.md` — the explicit Tab 2 plan
- `Projects/Aurix/_Overview.md` (LifeOS vault) — the current overall status
- `Projects/Aurix/Work/Status Decision.md` (LifeOS vault) — the pending revive/pause/decommission decision
