# Aurix

> A local-first DeFi analytics platform for Ethereum — built to understand how decentralised financial markets actually work: real-time arbitrage detection, liquidity pool analysis, on-chain position tracking, gas optimisation, and quantitative risk modelling, all running entirely on your machine.

---

## Why Aurix exists

Most DeFi tooling is either a hosted dashboard that requires a wallet connection and sends your data to third-party servers, or a raw blockchain explorer that gives you raw transaction data with no analytical layer on top.

Aurix sits between those two extremes: a fully local, privacy-first analytics platform that connects directly to Ethereum, processes on-chain data in a high-performance backend, and surfaces insights through a clean dashboard — with no cloud dependencies, no ongoing costs, and no wallet required.

The goal is not to build a trading bot or a production DeFi protocol. The goal is to deeply understand how decentralised markets function at an engineering level: how prices form across competing liquidity pools, how arbitrage opportunities emerge and close, how gas costs shape what is and is not profitable, and how on-chain risk can be quantified and monitored.

---

## What Aurix focuses on

Aurix is a deliberate, milestone-driven project targeting five distinct areas of DeFi analytics:

- **Arbitrage detection** — real-time price comparison across DEXes with gas-adjusted profit estimation
- **Liquidity pool analysis** — historical LP position simulation with Uniswap V3 tick mathematics, fee income, and impermanent loss modelling
- **On-chain position tracking** — wallet-level DeFi position monitoring across protocols with unrealised PnL
- **Gas price intelligence** — real-time gas monitoring with historical pattern analysis and optimal timing recommendations
- **Quantitative risk modelling** — token correlation matrices, rolling volatility, and Value-at-Risk estimation across a custom portfolio

Each tab is independently completable. A working Tab 1 is a shippable project. Each subsequent tab adds a new analytical capability without depending on the previous one being fully polished.

---

## Architecture

Aurix runs entirely locally as a desktop application. The high-level structure is:

```
┌─────────────────────────────────┐
│         React Frontend          │
│   Dashboard, charts, controls   │
└────────────────┬────────────────┘
                 │ IPC / local API
┌────────────────▼────────────────┐
│          Rust Backend           │
│  Concurrent data fetching       │
│  Price comparison engine        │
│  Financial calculation logic    │
│  Risk modelling                 │
└────────────────┬────────────────┘
                 │
┌────────────────▼────────────────┐
│        Local Storage            │
│  Opportunity log, price history │
│  position snapshots, gas data   │
└────────────────┬────────────────┘
                 │ Public endpoints
┌────────────────▼────────────────┐
│        Ethereum Mainnet         │
│  DEX price feeds, on-chain data │
│  Gas prices, wallet positions   │
└─────────────────────────────────┘
```

Specific technologies, RPC providers, and data sources will be decided during development based on what works best at each stage.

---

## Design Principles

- **Local-first**: all computation, storage, and data fetching runs on your machine with no intermediary servers
- **Zero cost**: built on free public data sources — no wallet, no ETH, no paid services required
- **Read-only**: Aurix never submits transactions or interacts with the blockchain in a write capacity; it only observes and analyses
- **Tab-scoped independence**: each analytical tab is a self-contained module that can be developed, tested, and demonstrated independently
- **Decisions during development**: specific libraries, endpoints, and implementation approaches are chosen when building each milestone, not prescribed upfront

---

## Roadmap

- [ ] Tab 1: Arbitrage Scanner
- [ ] Tab 2: Liquidity Pool Analyser
- [ ] Tab 3: Wallet & Position Tracker
- [ ] Tab 4: Gas Price Monitor & Predictor
- [ ] Tab 5: Token Correlation & Risk Dashboard

---

## 📍 Milestones

---

### Tab 1 — Arbitrage Scanner

> **Goal**: Detect real-time cross-DEX price discrepancies and surface gas-adjusted profit opportunities

#### Core Concept

Arbitrage in DeFi occurs when the same token pair trades at different prices on different decentralised exchanges simultaneously. Each DEX runs an independent automated market maker with its own liquidity pool, so prices drift apart until traders close the gap. Aurix detects these gaps, estimates whether they are profitable after gas costs, and surfaces them in a live feed. No trades are executed — only detection and analysis.

---

#### Milestone 1.1 — Data Pipeline Foundation

- [x] Project skeleton set up and running locally
- [x] Successfully connect to at least one DEX and retrieve a live token price
- [x] Price displayed in terminal or basic UI output

**Exit criteria**: A live ETH price from at least one DEX is fetched and displayed on demand

---

#### Milestone 1.2 — Concurrent Multi-DEX Fetching

- [x] Connect to at least two additional DEXes
- [x] All price feeds fetched concurrently without one blocking another
- [x] Prices normalised to a consistent format for comparison
- [ ] Failed or stale connections handled gracefully without crashing
- [ ] Raw prices logged locally with timestamps

**Exit criteria**: Prices from at least three DEXes fetched concurrently and stored locally

---

#### Milestone 1.3 — Spread Detection & Gas Modelling

- [x] Price comparison engine identifies spread between any two DEX prices
- [x] Current gas price fetched and incorporated into profit calculation
- [x] Net profit estimated: spread minus estimated gas cost
- [ ] Opportunities above a configurable threshold flagged and logged

**Exit criteria**: Scanner correctly identifies and logs profitable spreads with gas-adjusted profit estimates

---

#### Milestone 1.4 — Dashboard

- [x] Live price display per DEX updating in real-time
- [x] Pairwise spread overview across monitored DEXes
- [ ] Scrolling opportunity feed with spread, estimated profit, and timestamp
- [ ] Configurable minimum profit threshold
- [ ] Connection status visible per feed

**Exit criteria**: Full local desktop app with live price dashboard and opportunity feed

---

#### Milestone 1.5 — Polish & Historical View

- [ ] Historical chart of opportunity frequency and average spread over time
- [ ] Per-DEX statistics on opportunity generation
- [ ] Opportunity log exportable
- [ ] Clean shutdown and restart without data loss
- [ ] Demo recording for README

**Exit criteria**: Tab 1 fully demonstrable as a standalone project

---

### Tab 2 — Liquidity Pool Analyser

> **Goal**: Simulate historical LP position performance on Uniswap V3 with fee income and impermanent loss modelling

#### Core Concept

Uniswap V3 introduced concentrated liquidity — liquidity providers choose a price range within which their capital is active. Inside the range they earn trading fees; outside it they stop earning and suffer impermanent loss as their position converts to the cheaper token. The underlying mathematics involves fixed-point arithmetic and tick-based price representation, making accurate simulation genuinely non-trivial. This tab answers the question every LP asks: given historical prices, which range and strategy would have performed best?

---

#### Milestone 2.1 — Historical Data Ingestion

- [ ] Source for historical pool price data identified and integrated
- [ ] Price history for a selected pool stored locally
- [ ] Pool selector allowing the user to choose a token pair

**Exit criteria**: Historical price data for a selected pool available locally

---

#### Milestone 2.2 — Tick Mathematics Engine

- [ ] Tick-to-price and price-to-tick conversion implemented correctly
- [ ] Liquidity share for a given position calculated accurately
- [ ] Position active/inactive status determined at each historical price point
- [ ] Mathematical primitives validated against known reference values

**Exit criteria**: Tick math engine produces correct outputs validated against reference data

---

#### Milestone 2.3 — Backtesting Engine

- [ ] LP position simulated across full historical price dataset
- [ ] Fee income calculated per period based on active time in range
- [ ] Impermanent loss calculated at each price point
- [ ] Net position value tracked against a hold-only baseline
- [ ] Multiple price range strategies comparable in a single run

**Exit criteria**: Backtester produces fee income, impermanent loss, and net return for any historical position

---

#### Milestone 2.4 — Dashboard

- [ ] Selected price range visualised over historical price chart
- [ ] PnL chart showing fee income, impermanent loss, and net return over time
- [ ] Strategy comparison view for multiple price ranges
- [ ] Position summary: total fees, total IL, net return, percentage of time in range

**Exit criteria**: Full visual backtesting dashboard for any supported pool

---

### Tab 3 — Wallet & Position Tracker

> **Goal**: Monitor any Ethereum wallet's current DeFi positions with live valuation and unrealised PnL

#### Core Concept

Every action on Ethereum is public and readable without authentication. Given any wallet address, it is possible to fetch its token balances, active liquidity positions, lending positions, and historical activity entirely from public on-chain data. This tab builds a read-only position monitor — no private key, no wallet connection, just an address and a live view of what it holds.

---

#### Milestone 3.1 — Wallet Balance Fetching

- [ ] Accept any Ethereum wallet address as input
- [ ] Fetch ETH balance and major token balances
- [ ] Resolve token symbols and display with USD valuation

**Exit criteria**: Any wallet address shows current token balances with USD values

---

#### Milestone 3.2 — DeFi Position Decoding

- [ ] Active Uniswap V3 LP positions detected and decoded
- [ ] Current position value calculated at live prices
- [ ] At least one additional protocol position type detected

**Exit criteria**: Wallet view shows active DeFi positions with current valuations

---

#### Milestone 3.3 — PnL Tracking

- [ ] Periodic position snapshots stored locally
- [ ] Unrealised PnL calculated from first snapshot to current
- [ ] Position value history chart

**Exit criteria**: Wallet tracker shows unrealised PnL for all tracked positions

---

### Tab 4 — Gas Price Monitor & Predictor

> **Goal**: Track Ethereum gas prices in real-time, surface historical patterns, and recommend optimal transaction timing

#### Core Concept

Gas prices on Ethereum fluctuate significantly across hours and days of the week, driven by network demand. Submitting a transaction at a high-gas moment can cost several times more than waiting a few hours. This tab monitors gas in real-time, surfaces the historical patterns that make certain windows cheaper, and uses local data to make timing recommendations.

---

#### Milestone 4.1 — Real-Time Gas Monitoring

- [ ] Current gas price fetched on a configurable polling interval
- [ ] Live display of base fee and priority fee
- [ ] Historical gas readings stored locally

**Exit criteria**: Live gas price dashboard with local historical logging

---

#### Milestone 4.2 — Pattern Analysis

- [ ] Gas history aggregated by hour-of-day and day-of-week
- [ ] Statistically cheapest transaction windows identified
- [ ] Heatmap visualisation of gas cost patterns across the week

**Exit criteria**: Heatmap showing cheapest and most expensive gas windows

---

#### Milestone 4.3 — Prediction & Recommendations

- [ ] Simple model trained on local gas history to estimate near-term price range
- [ ] Recommendation surfaced based on current gas versus recent average
- [ ] Suggested wait time if gas is currently elevated

**Exit criteria**: Gas predictor surfaces actionable timing recommendations from local data

---

### Tab 5 — Token Correlation & Risk Dashboard

> **Goal**: Quantify portfolio risk through correlation analysis, rolling volatility, and Value-at-Risk estimation

#### Core Concept

Holding multiple tokens does not automatically reduce risk if those tokens move together. Correlation measures how similarly two assets behave; a portfolio of highly correlated tokens has far less diversification than it appears. Value-at-Risk quantifies the maximum expected loss over a given period at a given confidence level. This tab builds the quantitative layer that most DeFi dashboards never include — turning raw price data into actionable risk metrics.

---

#### Milestone 5.1 — Price History Collection

- [ ] Historical price data sourced for a set of major tokens
- [ ] Stored locally with configurable lookback window
- [ ] Token selector for custom portfolio composition

**Exit criteria**: Historical price data available locally for selected tokens

---

#### Milestone 5.2 — Correlation & Volatility Engine

- [ ] Rolling pairwise correlation matrix computed across selected tokens
- [ ] Rolling annualised volatility calculated per token
- [ ] Correlation heatmap displayed
- [ ] High-correlation pairs flagged as concentration risk

**Exit criteria**: Live correlation matrix and volatility metrics for any token selection

---

#### Milestone 5.3 — Value-at-Risk & Stress Testing

- [ ] Historical simulation VaR implemented at configurable confidence levels
- [ ] Portfolio VaR calculated for user-defined position sizes
- [ ] Stress test against at least two major historical drawdown periods
- [ ] Risk summary: VaR, max drawdown, Sharpe ratio

**Exit criteria**: Portfolio risk dashboard with VaR, Sharpe, and historical stress test results

---

## Running Locally

```bash
# Clone the repository
git clone https://github.com/Capataina/Aurix
cd Aurix

# Setup and run instructions will be added as the project develops
```

No paid services or API keys required. All data sources used are free and publicly accessible.

---

## Summary

Aurix is a local-first DeFi analytics desktop application that brings together real-time arbitrage detection, liquidity pool mathematics, on-chain position tracking, gas intelligence, and quantitative risk modelling. It demonstrates concurrent systems engineering, deep understanding of Ethereum and DeFi protocol mechanics, and end-to-end product delivery — with zero cloud dependencies and zero running costs.
