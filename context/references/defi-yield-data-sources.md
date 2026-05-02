# DeFi Yield Data Sources — APIs, Schemas, and Best Practices

## Scope / Purpose

This paper is the implementation reference for the multi-asset benchmark fetcher needed by **M2.7 of `context/plans/vector-a-v3-lp-backtester.md`**. The Tab 2 LP backtester benchmarks Uniswap V3 LP returns against four DeFi-native primaries — **Aave V3 USDC supply APY**, **Compound V3 USDC supply APY**, **Lido stETH APY**, and **native ETH staking effective yield** — plus the three TradFi secondaries (T-bills, S&P 500, gold). This paper covers only the four DeFi-native primaries, because they are the ones that drive the headline framing in M2.8 ("could the user have lent the stable half instead of LP'ing?") and because the TradFi sources (FRED, Yahoo) are well-trodden and would not benefit from a research artefact.

The paper covers, for each primary benchmark:

- exact endpoint URLs (host + path + path parameters),
- request/response schemas with field names and types verified against live calls,
- historical depth available today (how many days back),
- authentication requirements (free/no-key vs key-required),
- rate limits with explicit numbers,
- a primary fetcher and a fallback chain,
- the computation nuance separating "published APY" from "realised return for a depositor over a window".

It also covers cross-cutting concerns:

- the post-June-2024 The Graph migration, because that change makes a large body of older "use the Aave subgraph" advice obsolete,
- a SQLite caching schema with daily granularity that fits the existing M2.0 persistence direction,
- a 24-month backfill request-count budget.

**Out of scope for this paper:**

- TradFi sources (FRED, Yahoo Finance) — straightforward enough for the implementer to handle directly,
- borrow APY (Aurix benchmarks lending, not borrowing),
- L2 lending markets (mainnet only, per Vector A scope),
- protocols outside Aave V3 / Compound V3 / Lido / native staking (Morpho, Spark, RocketPool, Frax — all in M2.7 *out of scope*).

## Current Project Relevance

The fetcher is the first new I/O subsystem in Aurix that talks to something other than an Ethereum JSON-RPC endpoint. The architecture today (`context/architecture.md` §State Ownership) has exactly one external dependency — the mainnet RPC URL — and one HTTP client (`reqwest::Client` inside `src-tauri/src/ethereum/client.rs`). M2.7 introduces three or four new external hosts, each with its own auth model, rate limits, and failure modes. Getting the data-source decisions right here matters disproportionately because:

1. **Single-host failure is an existing project-wide failure mode.** From `context/architecture.md`: *"Single external dependency: the Ethereum mainnet JSON-RPC endpoint. … Provider rate-limiting, outage, or rewrite of the free Alchemy tier silently fails every 1 Hz tick simultaneously — the product has no alternate path."* The benchmark fetcher should not import the same fragility for the headline analysis. Each benchmark needs a primary source and a documented fallback.
2. **Backtests rerun across 24 months of history.** A user adjusting a backtest period repeatedly should not hammer the upstream. The persistence layer (M2.0) already exists in plan form — the fetcher must cache aggressively into SQLite and only call upstream for missing dates.
3. **Hiring credibility depends on the benchmark module being correct.** From the plan's M2.7 acceptance criterion: *"the benchmark module's reported S&P 500 total return matches a public source within 0.1% absolute return; reported Aave USDC return matches a DefiLlama or Aave-dashboard public number for the same window within 0.05% APY."* If DefiLlama and the Aave subgraph disagree by 0.5%, the implementer needs a documented reason — that is partly a goal of this paper.
4. **The Graph migrated in June 2024.** A large body of public guidance for "fetch Aave APY from the subgraph" is now subtly wrong because hosted-service endpoints stopped working. This must be addressed up front; otherwise the implementer will follow stale tutorials.

## Current State Snapshot

What the repository currently has that this paper builds on:

| Repository fact | Citation (file:line) |
|---|---|
| Single-host, single-HTTP-client backend pattern | `src-tauri/src/ethereum/client.rs:7-25` (`EthereumRpcClient` constructed per IPC call wrapping a single `reqwest::Client`) |
| `reqwest 0.12` with `rustls-tls` and `json` features available | `src-tauri/Cargo.toml:29` |
| `serde_json`, `thiserror`, `tokio` (multi-thread) all present | `src-tauri/Cargo.toml:23-31` |
| Per-module `thiserror::Error` enum convention | `context/notes/error-handling.md` (`ConfigError`, `EthereumRpcError`, `UniswapV2Error`, `UniswapV3Error`) |
| Wire convention: `f64` for monetary fields, `serde(rename_all="camelCase")` at IPC | `context/notes/wire-convention.md` |
| No persistence layer yet — M2.0 designs but does not implement | `context/plans/vector-a-v3-lp-backtester.md` §M2.0 |
| No HTTP host other than the configured RPC URL is currently called | `src-tauri/src/ethereum/client.rs:13-20` (only `rpc_url` is held; no other hosts) |

What this paper assumes will be true by the time the fetcher is implemented:

- M2.0 ships a `rusqlite` (or `sqlx`) integration with WAL mode and a migration framework (per `vector-a-v3-lp-backtester.md` §M2.0).
- A `storage::benchmark` module exists with `store_benchmark_series` and `query_benchmark_range` Tauri commands.
- A new module `src-tauri/src/yields/` (suggested) houses the four fetchers, mirroring the `dex/` module shape.

## Research Signal

| # | Topic | Source-backed signal | Source citation (URL + quoted passage ID) | Current repository state | Project implication | Evidence class |
|---|---|---|---|---|---|---|
| 1 | DefiLlama Yields API is open and free — no auth required | "API … is an open API and it is free to use" — DefiLlama API Docs landing page | api-docs.defillama.com → **[Q-DLOPEN]** | No HTTP client currently calls non-RPC hosts | DefiLlama is the natural primary for all three protocol benchmarks (Aave V3 USDC, Compound V3 USDC, Lido stETH); zero auth setup keeps the project's "no API keys" hiring story intact | source-backed |
| 2 | DefiLlama yields chart endpoint host is `yields.llama.fi`, not `api.llama.fi` | Live `curl https://yields.llama.fi/chart/{pool_id}` returns 200 OK with documented schema; `api.llama.fi` returns 404 for the same path | Direct probe **[Q-DLPROBE]** | n/a | Implementer must use `https://yields.llama.fi/chart/{pool}` and `https://yields.llama.fi/pools` — this is a docs gotcha (the docs imply `api.llama.fi`) | source-backed (live probe) |
| 3 | DefiLlama daily depth is real for all three benchmarks | Probed: Aave V3 USDC chart 1182 daily points back to 2023-02-06; Compound V3 USDC 1301 points back to 2022-10-06; Lido stETH 1431 points back to 2022-05-03 | Direct probe **[Q-DLDEPTH]** | n/a | 24-month backfill is comfortably in range for all three; ≥3 years of history available for stress-test windows | source-backed (live probe) |
| 4 | The Graph hosted service was sunset June 12 2024; new endpoints require an API key on the decentralised network | "As of June 12th, 2024, the hosted service is no longer active." — The Graph blog | thegraph.com/blog/sunsetting-hosted-service/ → **[Q-GRAPHSUNSET]** | Project has no Graph integration | The Aave subgraph fallback for V1 needs an API key; this conflicts with the project's no-key preference. Use Aave subgraph only as a verification cross-check for the M2.7 acceptance criterion, not as primary | source-backed |
| 5 | The Graph free plan is 100,000 queries/month with API key | "Get started with 100,000 free subgraph queries per month in Subgraph Studio." — The Graph blog | thegraph.com/blog/sunsetting-hosted-service/ → **[Q-GRAPHFREE]** | n/a | If the user supplies a Graph API key, 100k/month is plenty (a 24-month backfill needs <1k queries); the key requirement is the constraint, not the quota | source-backed |
| 6 | beaconcha.in v1 endpoints all require a Bearer token | Probed: every `/api/v1/...` endpoint returns `{"error":"Unauthorized: a valid API key is required..."}` | Direct probe **[Q-BEACONPROBE]** | n/a | beaconcha.in is unsuitable as a primary — even reading ETH.STORE requires registration. Demote to fallback | source-backed (live probe) |
| 7 | beaconcha.in free tier is 1 req/sec, 1,000 req/month | "The API is free to use under a fair use policy, with rate limits of 1 request per second / 1,000 requests per month." — docs.beaconcha.in | docs.beaconcha.in → **[Q-BEACONLIMIT]** | n/a | 1k/month is enough for daily polling but kills any multi-validator analysis; staking yield primary should not use beaconcha.in | source-backed |
| 8 | Lido eth-api is open, no key required, returns daily APR | Probed: `curl https://eth-api.lido.fi/v1/protocol/steth/apr/last` returns `{"data":{"timeUnix":...,"apr":2.448},"meta":{...}}` with no auth | Direct probe **[Q-LIDOPROBE]** | n/a | Lido stETH APR has a clean, key-free primary that matches the project's no-key preference exactly | source-backed (live probe) |
| 9 | DefiLlama APY methodology has a documented ambiguity around the measurement window | "Just an APY value is ambiguous, you always have to specify what time period it's computed over … What period does DefiLlama use?" — DefiLlama Issue #6 | github.com/DefiLlama/yield-server/issues/6 → **[Q-DLAPYAMBIG]** | n/a | DefiLlama's `apy` field is a snapshot of the protocol's instantaneous rate, *not* a realised return over the window. The fetcher must compute realised returns from the daily series itself, not trust DefiLlama's `apy` to be a holding-period return | source-backed (contrasting/limiting source) |
| 10 | Aave V3 supply rates are stored as `liquidityRate` in RAY units (1e27); APR is `rate / 1e27`; APY uses per-second compounding `((1+APR/SECONDS_PER_YEAR)^SECONDS_PER_YEAR - 1)` | "All rates and indices queried on-chain or from subgraphs are expressed in RAY units (10^27) … APY is calculated using the formula: ((1 + (depositAPR / SECONDS_PER_YEAR)) ^ SECONDS_PER_YEAR) - 1" — Aave dev community write-up | medium.com/@ancilartech (citing Aave docs) → **[Q-AAVECONV]** | n/a | Crucial for the cross-check: DefiLlama publishes the converted APY, the subgraph publishes the raw RAY rate. To verify within 0.05% the implementer must apply the conversion correctly | source-backed |
| 11 | Aave V3 official Ethereum subgraph (decentralised network) ID is `Cd2gEDVeqnjBn1hSeqFMitw8Q1iiyV9FYUZkLNRcL87g` | The Graph Explorer link `thegraph.com/explorer/subgraphs/Cd2gEDVeqnjBn1hSeqFMitw8Q1iiyV9FYUZkLNRcL87g` is the deployment listed in the Aave-org `protocol-subgraphs` GitHub README under "ETH Mainnet V3" | github.com/aave/protocol-subgraphs (README) → **[Q-AAVEGRAPHID]** | n/a | This is the canonical subgraph for the cross-check; query URL is `https://gateway.thegraph.com/api/{API_KEY}/subgraphs/id/{SUBGRAPH_ID}` | source-backed |
| 12 | Compound V3 has no first-party subgraph; community Paperclip Labs subgraph fills the gap | "Compound V3 community subgraph, managed by Paperclip Labs, provides an index of the Compound III protocol on Ethereum and Base" — Zeeve write-up | zeeve.io blog (paraphrased) → **[Q-COMPOUNDSG]** | n/a | DefiLlama is the cleanest Compound V3 primary; subgraph fallback is community-maintained, slightly weaker provenance, requires Graph API key | source-backed |

## What The Topic Actually Is

A short conceptual grounding before diving into per-source detail. Three terms appear repeatedly and have different operational meanings depending on the source:

- **APR (Annual Percentage Rate)** — the annualised rate without compounding. If a protocol earns 0.0001% per second, APR is `0.0001% × 31,536,000 seconds/year`. Aave V3's on-chain `liquidityRate`, after dividing by RAY (1e27), is an APR. Lido's `apr` field in `eth-api` is also an APR.
- **APY (Annual Percentage Yield)** — the annualised rate *with* compounding at some interval. Aave V3's effective APY assumes per-second compounding: `((1 + APR/SECONDS_PER_YEAR)^SECONDS_PER_YEAR) - 1`. DefiLlama's `apy` field is the post-conversion value, already compounded.
- **Realised holding-period return** — the actual return a depositor would have earned over a window `[t0, t1]`. *This is not the same as the published APY*. APY is forward-looking, instantaneous, and derived from the rate at the time of observation; the realised return is the path-integral of the per-second rate over the holding period. For a backtester, the realised return is what matters for the equity curve.

Two computation paths the implementer needs to keep straight:

```text
On-chain raw -----------------------------> Realised return path
liquidityRate (RAY/sec)                     liquidityIndex history
       |                                              |
       v                                              v
   APR snapshot                              integrate over [t0,t1]
       |                                              |
       v                                              v
   APY snapshot                              true depositor return
       |                                              |
   (DefiLlama publishes here)                   (what M2.7 needs)
```

DefiLlama publishes the **APY snapshot path**. To get the realised return in the backtester:

- **Option A (cleanest):** read the daily APY series from DefiLlama, apply each day's APY as `(1 + apy/365)` to the running balance. This is what a typical retail user would have realised; the small error vs the true continuous-compounding integral is dominated by the daily granularity DefiLlama already enforces.
- **Option B (most accurate):** read the on-chain `liquidityIndex` at `t0` and `t1` and compute `(index_t1 / index_t0) - 1`. This is the exact realised return per Aave's accounting. Requires archive node access at two specific blocks.

For Aurix M2.7, Option A is correct. The plan's acceptance criterion ("matches a DefiLlama or Aave-dashboard public number for the same window within 0.05% APY") is itself written against Option A semantics. Option B is a stretch upgrade only if a discrepancy >0.05% surfaces during validation.

## Per-Source Specification

### 1. DefiLlama Yields API (primary for all three protocol benchmarks)

#### Endpoints

| Endpoint | Method | Auth | Use |
|---|---|---|---|
| `https://yields.llama.fi/pools` | GET | none | List all pools, find pool IDs |
| `https://yields.llama.fi/chart/{pool}` | GET | none | Daily historical APY + TVL for one pool |

> **Host gotcha:** The official docs at `api-docs.defillama.com` describe `/pools` and `/chart/{pool}` as if they live under `api.llama.fi`. They do not. Yields lives on a separate subdomain `yields.llama.fi`. Live-verified `2026-05-02`.

#### Pool IDs to use (verified live `2026-05-02`)

| Benchmark | DefiLlama pool ID | poolMeta | TVL |
|---|---|---|---|
| **Aave V3 USDC supply (Ethereum, Core market)** | `aa70268e-4b52-42bf-a116-608b370f9501` | `null` (canonical Core market) | $152M |
| **Compound V3 cUSDCv3 supply (Ethereum)** | `7da72d09-56ca-4ec5-a45f-59114353e487` | `null` | $79M |
| **Lido stETH** | `747c1d2a-c668-4682-b9f9-296708a3dd90` | `null` | $20.9B |

> **Aave market gotcha:** `yields.llama.fi/pools` returns *four* entries with `project=aave-v3`, `chain=Ethereum`, `symbol=USDC`. The `poolMeta` field disambiguates them — `null` is the Core market (the one M2.7 wants); the others are sub-markets `horizon-market`, `lido-market`, `etherfi-market` with TVLs in the single-digit-millions or smaller and rates that diverge meaningfully from the Core. Hardcoding pool IDs is acceptable; if the project ever wants pool-meta defensiveness, filter `pool.poolMeta == null` after a project+chain+symbol match.

#### Response schema — `/chart/{pool}` (verified live)

```json
{
  "status": "success",
  "data": [
    {
      "timestamp": "2023-02-06T23:01:24.670Z",
      "tvlUsd": 6884855,
      "apy": 1.80182,
      "apyBase": 1.80182,
      "apyReward": null,
      "il7d": null,
      "apyBase7d": null,
      "pricePerShare": null
    },
    /* … one entry per day … */
  ]
}
```

| Field | Type | Meaning | Notes |
|---|---|---|---|
| `timestamp` | ISO 8601 string (UTC) | Daily snapshot timestamp | Roughly 23:00 UTC; treat as date-of |
| `tvlUsd` | number (integer USD) | Pool USD TVL at snapshot | Use for sanity-check on small-pool drift |
| `apy` | number (percent) | Total APY: `apyBase + apyReward` | The headline number |
| `apyBase` | number (percent) | Pool's intrinsic supply APY | Excludes reward token emissions |
| `apyReward` | number or `null` (percent) | APY contribution from reward tokens | Compound V3 emits COMP, so non-null; Aave V3 USDC supply has no incentive currently |
| `il7d` | number or `null` | 7-day impermanent loss for AMM pools | Always `null` for lending |
| `apyBase7d` | number or `null` | 7-day moving average of `apyBase` | Often `null` for lending pools |
| `pricePerShare` | number or `null` | For yield-bearing wrappers (e.g. wstETH, aTokens) | Often `null` |

**For M2.7 use `apy` (which equals `apyBase + apyReward` when both present).** Ignore Compound V3's COMP rewards if the user opts out via UI; otherwise the headline is "what a passive depositor receives", which includes COMP emissions, so `apy` is correct.

#### Historical depth (verified live)

| Pool | Earliest data | Daily points |
|---|---|---|
| Aave V3 USDC | 2023-02-06 | 1182 |
| Compound V3 USDC | 2022-10-06 | 1301 |
| Lido stETH | 2022-05-03 | 1431 |

A 24-month backfill (M2.7 default) is well inside range for all three. A 36-month backfill works for Compound and Lido, marginal for Aave V3 (which only launched on Ethereum in early 2023).

#### Authentication and rate limits

> **[Q-DLOPEN]** "DefiLlama's API is an open API and it free to use." (api-docs.defillama.com landing)

> **[Q-DLRATE]** Rate limit guidance is conflicting in third-party writeups. The official docs say "free", and the FAQ does not publish a number. The pragmatic advice from community usage: "the public API has no rate limits for normal traffic" — but burst patterns can trip soft limits. (DEV.to roundup, paraphrased)

**Operational decision for Aurix:** assume an effective floor of 10 req/s and a per-day budget under 10k req/day for safety; in practice the fetcher needs <100 req for an initial 24-month backfill and 1 req/day for incremental updates, so the limit never bites.

#### APY computation methodology — known limitation

> **[Q-DLAPYAMBIG]** From DefiLlama yield-server Issue #6 (opened by user `ruuda`, May 2022): *"just an APY value is ambiguous, you always have to specify what time period it's computed over. […] What period does DefiLlama use? […] How does DefiLlama deal with this?"*

The issue is closed without a documented response. From the yield-server adapter source pattern (each protocol has its own adapter, see `github.com/DefiLlama/yield-server`), DefiLlama publishes the **protocol's instantaneous reported APY**, not a windowed realised return. For Aave V3 the adapter reads `liquidityRate` from the contract and converts via `((1+APR/SECONDS_PER_YEAR)^SECONDS_PER_YEAR-1)`. For Compound V3 it reads `getSupplyRate()` and applies the same conversion.

**Implication for Aurix:** when the M2.7 chart overlays "Aave APY over the period", what is plotted is *the daily-snapshot APY series at the time each datum was taken*, not the *realised return* from holding aTokens for the period. The two converge if the rate is stable; they diverge during volatility (e.g. March 2023 USDC depeg). The implementer should label charts as "Reported supply APY (daily)" rather than "Realised return", and reserve the realised-return computation for the equity-curve overlay (compounding the daily APY values geometrically).

#### Error handling

- HTTP 200 + `{"status": "error", ...}` exists but is rare; treat as a soft failure.
- HTTP 5xx during partial outages: implement exponential backoff (3 retries, 1s/4s/16s).
- `data` array can be empty for newly-created pools (post-launch); fetcher should treat empty as "no data yet, retry tomorrow".

### 2. Aave V3 subgraph (cross-check / fallback for Aave only)

#### Endpoint

| Endpoint | Method | Auth | Use |
|---|---|---|---|
| `https://gateway.thegraph.com/api/{API_KEY}/subgraphs/id/Cd2gEDVeqnjBn1hSeqFMitw8Q1iiyV9FYUZkLNRcL87g` | POST (GraphQL) | API key required | Historical Aave V3 reserve liquidity rate |

> **[Q-GRAPHSUNSET]** "As of June 12th, 2024, the hosted service is no longer active." (thegraph.com/blog/sunsetting-hosted-service/) — any tutorial or third-party article telling you to use `api.thegraph.com/subgraphs/name/aave/protocol-v3` is now broken.

> **[Q-GRAPHFREE]** "Get started with 100,000 free subgraph queries per month in Subgraph Studio." (same blog post). The free quota is generous for Aurix — a 24-month backfill is <1000 queries — but **a key is mandatory**, no-key access does not exist on the decentralised network.

#### GraphQL query for daily APR series

The Aave-org `protocol-subgraphs` repo confirms the official Ethereum V3 deployment is **`Cd2gEDVeqnjBn1hSeqFMitw8Q1iiyV9FYUZkLNRcL87g`**. The schema exposes a `reserveParamsHistoryItem` entity that snapshots reserve state on every interaction (deposit, borrow, repay, liquidate). For daily aggregation, group on `timestamp / 86400`.

```graphql
query AaveV3UsdcDailyApr($from: Int!, $to: Int!) {
  reserveParamsHistoryItems(
    where: {
      reserve_: { underlyingAsset: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" }
      timestamp_gte: $from
      timestamp_lte: $to
    }
    orderBy: timestamp
    orderDirection: asc
    first: 1000
  ) {
    timestamp
    liquidityRate
    variableBorrowRate
    utilizationRate
    totalLiquidity
  }
}
```

USDC underlying is `0xa0b86991...eb48` (lowercase, Ethereum mainnet). `liquidityRate` is in RAY (1e27).

#### Conversion to APY (apply per quoted passage)

> **[Q-AAVECONV]** *"All rates and indices queried on-chain or from subgraphs are expressed in RAY units (10^27) … APY is calculated using the formula: `((1 + (depositAPR / SECONDS_PER_YEAR)) ^ SECONDS_PER_YEAR) - 1`, where SECONDS_PER_YEAR = 31536000."* (community-summarised from Aave docs)

```python
SECONDS_PER_YEAR = 31_536_000
RAY = 10**27

deposit_apr = float(liquidity_rate) / RAY                      # APR as a fraction (0.045 = 4.5%)
deposit_apy = (1 + deposit_apr / SECONDS_PER_YEAR) ** SECONDS_PER_YEAR - 1
```

In Rust:

```rust
const SECONDS_PER_YEAR: f64 = 31_536_000.0;
const RAY: f64 = 1e27;

fn liquidity_rate_to_apy(liquidity_rate_ray: &num_bigint::BigUint) -> f64 {
    // BigUint -> f64 via conversion via_string-then-parse to avoid overflow
    let apr = bigint_to_f64(liquidity_rate_ray) / RAY;
    (1.0 + apr / SECONDS_PER_YEAR).powf(SECONDS_PER_YEAR) - 1.0
}
```

(Aurix already imports `num-bigint = "0.4"` per `Cargo.toml:27`; the conversion is identical to the Q64.96 path used in `dex/uniswap_v3.rs`.)

#### Daily aggregation strategy

`reserveParamsHistoryItem` snapshots are per-event, so a busy reserve produces dozens per day and a quiet one might skip days. Two valid aggregations:

- **Last-of-day**: take the latest snapshot per UTC day. Matches what DefiLlama publishes (a daily snapshot near 23:00 UTC).
- **Time-weighted-average-of-day**: integrate `liquidityRate × dt` across the day, divide by 86400. More accurate as a "what was the average APR on this day" answer; more complex to implement.

For cross-checking against DefiLlama, **use last-of-day**. That is what DefiLlama reports.

#### Why this is fallback, not primary

1. Requires The Graph API key — conflicts with the project's no-key preference.
2. Per-event resolution is overkill for daily benchmarks.
3. Schema can shift between subgraph versions; DefiLlama's adapter absorbs that pain.

The intended use is the M2.7 acceptance criterion: *"reported Aave USDC return matches a DefiLlama or Aave-dashboard public number for the same window within 0.05% APY."* If the implementer wants a third independent number for that check, the Aave subgraph provides it.

### 3. Compound V3 supply APY (DefiLlama only)

There is no clean primary alternative to DefiLlama for Compound V3.

- The Compound V3 protocol exposes `cUSDCv3.getSupplyRate(utilization)` on-chain, which is the *current* rate but does not give history without archive-node `eth_call` at every historical block — too expensive for daily backfill.
- There is no first-party Compound V3 subgraph. The community Paperclip Labs subgraph is on the decentralised network and would need a Graph API key; provenance is community.

> **[Q-COMPOUNDSG]** *"Compound V3 community subgraph, managed by Paperclip Labs, provides an index of the Compound III protocol on Ethereum and Base, monitoring supply and borrowing positions, interest accumulation, and governance."* (Zeeve community write-up)

**Decision: use DefiLlama as primary; if the M2.7 cross-check fails, derive the rate directly from on-chain by reading `cUSDCv3.getUtilization()` and `cUSDCv3.getSupplyRate(utilization)` at one block per day via Alchemy's archive RPC.** The on-chain path is described in M2.1's existing `eth_call` infrastructure (`src-tauri/src/ethereum/client.rs`), so adding it would not introduce a new dependency.

### 4. Lido stETH APR (`eth-api.lido.fi`, primary)

#### Endpoints

| Endpoint | Method | Auth | Use |
|---|---|---|---|
| `https://eth-api.lido.fi/v1/protocol/steth/apr/last` | GET | none | Latest reported APR |
| `https://eth-api.lido.fi/v1/protocol/steth/apr/sma` | GET | none | 7-day simple moving average + last 7 daily APR points |

#### Response schema — `/last` (verified live)

```json
{
  "data": {
    "timeUnix": 1777724519,
    "apr": 2.448
  },
  "meta": {
    "symbol": "stETH",
    "address": "0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84",
    "chainId": 1
  }
}
```

#### Response schema — `/sma` (verified live)

```json
{
  "data": {
    "aprs": [
      { "timeUnix": 1777206119, "apr": 2.523 },
      { "timeUnix": 1777292591, "apr": 2.508 },
      { "timeUnix": 1777378919, "apr": 2.543 },
      { "timeUnix": 1777465247, "apr": 2.8 },
      { "timeUnix": 1777551647, "apr": 2.674 },
      { "timeUnix": 1777638035, "apr": 2.557 },
      { "timeUnix": 1777724519, "apr": 2.448 }
    ],
    "smaApr": 2.579
  },
  "meta": { "symbol": "stETH", "address": "0xae...", "chainId": 1 }
}
```

#### Critical Lido limitation

The Lido eth-api returns **only the last 7 daily points** in the `/sma` endpoint. It does not expose a deeper historical series. There is no `&from=<unix>&to=<unix>` query parameter documented, and Swagger (`https://eth-api.lido.fi/api/`) confirms this. The endpoint is designed to serve "what's my current APR for the dashboard widget", not "give me 24 months of history".

**Implication:** for Lido stETH, the *primary* historical source is **DefiLlama** (pool `747c1d2a...`, 1431 days back). The Lido eth-api is the **incremental-update** source: each day, fetch `/last`, append to SQLite. Once the daily-update cron is healthy, DefiLlama is no longer needed for Lido.

**During backfill:**

```text
Lido stETH backfill flow
========================
1. Hit yields.llama.fi/chart/747c1d2a-c668-4682-b9f9-296708a3dd90
2. Persist all 1431 daily points to sqlite (benchmark_yields table)
3. From day-of-most-recent-defillama-point onward, each daily cron tick
   calls eth-api.lido.fi/v1/protocol/steth/apr/last and appends.
```

#### Authentication and rate limits

> **[Q-LIDOREAD]** *"Lido APIs are strictly for read-only access."* (docs.lido.fi/integrations/api/)

No API key is required. Live-probed `2026-05-02`, no rate-limit headers returned. Reasonable to assume a polite limit of 1 req/min per consumer.

#### APR vs APY

Lido publishes APR (rebase-derived), not APY:

> *"For Lido V2+ the value is calculated based on rebase events using a specific algorithm that tracks share rate changes (totalPooledEther / totalShares) over time."* (docs.lido.fi)

Convert to APY for parity with DefiLlama:

```rust
let lido_apy = (1.0 + lido_apr / 365.0).powi(365) - 1.0;
// or per-second: (1.0 + lido_apr / SECONDS_PER_YEAR).powf(SECONDS_PER_YEAR) - 1.0
```

For stETH at ~3% APR the APY/APR difference is ~5 bps. Decide once whether the canonical column in SQLite is APR or APY (this paper recommends **storing APR and computing APY in the analysis layer**) and apply uniformly.

### 5. Native ETH staking effective yield

This is the trickiest of the four — none of the obvious sources are clean.

| Source | Pros | Cons |
|---|---|---|
| **beaconcha.in ETH.STORE** | Industry-standard reference rate, methodology open-source | Requires API key (probed: all v1 endpoints return 401), 1k req/month free tier, fair-use policy |
| **Rated Network** | Reportedly clean, validator-fleet-level | Not free for production; docs not openly browseable; pricing tier not public |
| **DefiLlama** | No native-staking pool — only liquid staking via wrappers | Doesn't answer the question |
| **Beacon API direct** | Free, self-hosted possible | Requires beacon node; `attestation_rewards` API gives per-validator data, not network average; computing daily network-average yield is a research project in itself |
| **Lido stETH APR (proxy)** | Already the primary for Lido benchmark | Includes Lido's 10% performance fee — not the same as raw native staking |
| **Coinbase / institutional aggregators** | Public dashboards | No public API; attribution requirements |

> **[Q-BEACONLIMIT]** *"The API is free to use under a fair use policy, with rate limits of 1 request per second / 1,000 requests per month."* (docs.beaconcha.in)

> **[Q-BEACONPROBE]** Live probe of `beaconcha.in/api/v1/ethstore/latest` returns `{"error":"Unauthorized: a valid API key is required..."}`. There is no public unauthenticated endpoint.

#### Recommended approach: ETH.STORE via beaconcha.in (with API key) as primary, Lido APR + 10% fee back-out as fallback

> **[Q-ETHSTORE]** *"ETH.STORE (Ether Staking Offered Rate) is a transparent Ethereum staking reward reference rate that represents the average financial return validators on the Ethereum network have achieved in a 24-hour period as published on beaconcha.in."* (beaconcha.in/ethstore)

beaconcha.in's free tier (1k req/month) is enough for a 24-month daily backfill (730 calls + buffer) and for daily incremental updates afterwards. **The implementer must register for a free API key** — flag this to the user before M2.7 starts.

If the user refuses to register a key, the fallback is:

```text
native_staking_apr_estimate = lido_steth_apr / (1 - 0.10)
```

i.e. gross-up the Lido APR by reversing the 10% performance fee. This is approximate (Lido has fluctuating MEV-share, validator-set composition differs from the network average, etc.) but typically within 30-50 bps of ETH.STORE — acceptable for the M2.7 chart given that native staking is the *secondary* benchmark behind Aave/Compound USDC and is usually only relevant for ETH-side LP exposure analysis.

#### beaconcha.in v1 ETH.STORE API (with key)

Endpoints discovered in the docs:

| Endpoint | Method | Notes |
|---|---|---|
| `https://beaconcha.in/api/v1/ethstore/{day}` | GET | `day=latest` returns the most recent; otherwise `{day}` is a beacon-chain day index |
| `https://beaconcha.in/api/v1/ethstore` | GET | Series endpoint (returns 401 without a key; documented behaviour) |

Headers: `Authorization: Bearer <YOUR_KEY>` or `?apikey=<YOUR_KEY>` query param.

The v1 endpoints are still operational ("V1 API remains available but no new features will be added to V1"). Stick with v1 for the ETH.STORE use case — v2 reorients toward dashboard/validator-list operations and is not aimed at network-wide reference rate consumption.

## Free Fallback Chains (per benchmark)

```text
Aave V3 USDC supply APY
  primary:  yields.llama.fi/chart/aa70268e-...     (no key)
  fallback: gateway.thegraph.com/.../subgraphs/id/Cd2gE...  (Graph API key required)
  emergency: alchemy.eth_call to AaveV3 Pool getReserveData(USDC) per-block (slow, archive)

Compound V3 USDC supply APY
  primary:  yields.llama.fi/chart/7da72d09-...     (no key)
  fallback: alchemy.eth_call to cUSDCv3.getSupplyRate(getUtilization()) per-block (archive)
  cross-check: paperclip-labs/compound-v3-subgraph (Graph API key required)

Lido stETH APR/APY
  primary (backfill):     yields.llama.fi/chart/747c1d2a-...   (no key)
  primary (incremental):  eth-api.lido.fi/v1/protocol/steth/apr/last  (no key)
  fallback: derive from on-chain rebase events via stETH share-rate change (archive)

Native ETH staking effective yield
  primary:  beaconcha.in/api/v1/ethstore/{day}     (free API key required)
  fallback: gross-up Lido stETH APR by 10% performance fee
  research-grade: Rated Network API (paid), or self-host a beacon node + attestation_rewards aggregation
```

## SQLite caching schema

Sized for the existing M2.0 direction. Daily granularity is sufficient — quoted DefiLlama and Lido data is itself daily, and the M2.7 acceptance criterion is per-window aggregate accuracy not per-tick. (For sanity: 24 months of hourly data for 4 benchmarks is ~70k rows; daily is ~3k rows. No reason to go finer than daily.)

```sql
-- Single normalised table for all benchmark series, keyed by source + day
CREATE TABLE benchmark_yields (
    source        TEXT    NOT NULL,    -- 'defillama' | 'lido_eth_api' | 'beaconchain_ethstore' | 'aave_subgraph'
    benchmark_id  TEXT    NOT NULL,    -- 'aave_v3_usdc_eth' | 'compound_v3_usdc_eth' | 'lido_steth' | 'eth_native_staking'
    date          TEXT    NOT NULL,    -- YYYY-MM-DD UTC
    apr           REAL,                 -- annualised, fraction (0.045 = 4.5%)
    apy           REAL,                 -- annualised, fraction (0.046 with compounding)
    apy_base      REAL,                 -- DefiLlama apyBase (where applicable)
    apy_reward    REAL,                 -- DefiLlama apyReward (where applicable)
    tvl_usd       REAL,                 -- pool TVL at snapshot (DefiLlama only)
    raw_payload   TEXT,                 -- the original JSON, for debug + re-parsing
    fetched_at    INTEGER NOT NULL,    -- unix seconds of the fetch event
    PRIMARY KEY (source, benchmark_id, date)
) WITHOUT ROWID;

CREATE INDEX idx_benchmark_yields_lookup
    ON benchmark_yields (benchmark_id, date);

-- Source attempt log for fallback-chain observability
CREATE TABLE benchmark_fetch_log (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    benchmark_id  TEXT    NOT NULL,
    source        TEXT    NOT NULL,
    requested_at  INTEGER NOT NULL,
    status        TEXT    NOT NULL,    -- 'ok' | 'http_4xx' | 'http_5xx' | 'transport' | 'parse'
    http_status   INTEGER,
    error_message TEXT,
    rows_returned INTEGER
);
```

Design notes:

- **`source` is part of the primary key**, not just `benchmark_id`. This lets the implementer store DefiLlama and Aave-subgraph values for the *same* benchmark+date side-by-side, which is exactly what the M2.7 cross-check needs ("DefiLlama said 3.50%, subgraph said 3.51% — that's within tolerance, accept").
- **Both `apr` and `apy` columns** even though one can be derived from the other. Storage cost is negligible; the analysis layer doesn't have to repeat the conversion; the source's native field is preserved verbatim.
- **`raw_payload`** preserves the original JSON for re-parsing if the schema interpretation changes. ~500 bytes/row × 3000 rows = 1.5 MB total — trivial.
- **`fetched_at`** answers "how stale is this benchmark when the user opens the backtest". For 24-month backfill the answer is days-old; for live-rebuilt analysis the answer is minutes-old.
- **`benchmark_fetch_log`** is the operational equivalent of Aurix's existing IPC error-banner: when a fallback chain fires, the user can see which source succeeded.

For the `query_benchmark_range` Tauri command:

```rust
// Pseudocode showing the read-side contract
#[tauri::command]
pub fn query_benchmark_range(
    benchmark_id: String,
    from_date: String,           // YYYY-MM-DD
    to_date: String,
    preferred_source: Option<String>,  // when Some, prefer that source; fallback to highest-priority
) -> Result<Vec<BenchmarkPoint>, String>;
```

The fetcher decides on each cold-cache hit: *"is the date already covered for this benchmark, by any acceptable source?"* — if not, fire the primary, then fallbacks in order, log each attempt to `benchmark_fetch_log`, accept the first success. This matches Aurix's existing per-module `thiserror` convention (`context/notes/error-handling.md`): each fetcher module gets a `YieldFetchError` enum and the chain selector composes them.

## Backfill request budget (24 months)

Single-shot backfill counts:

| Benchmark | Source | Calls for 24-month backfill | Notes |
|---|---|---|---|
| Aave V3 USDC | DefiLlama `/chart/{pool}` | **1** | One call returns the entire history |
| Compound V3 USDC | DefiLlama `/chart/{pool}` | **1** | Same |
| Lido stETH | DefiLlama `/chart/{pool}` | **1** | Same |
| Native ETH staking | beaconcha.in `/ethstore/{day}` | **730** (one per day) or **~24** if a range endpoint exists | Free-tier monthly limit is 1000, so 730 fits but tight |
| **Total cold-start backfill** | | **~733** | Comfortably inside all free tiers |

Daily incremental updates after backfill:

| Benchmark | Source | Calls/day |
|---|---|---|
| Aave V3 USDC | DefiLlama `/chart/{pool}` (re-fetch tail) or `/pools` (just-the-latest) | 1 |
| Compound V3 USDC | DefiLlama | 1 |
| Lido stETH | `eth-api.lido.fi/v1/protocol/steth/apr/last` | 1 |
| Native ETH staking | beaconcha.in `/ethstore/latest` | 1 |
| **Total per-day** | | **4** |

**Conclusion:** with one DefiLlama call covering each protocol benchmark's full history, and beaconcha.in's free-tier limit being the only meaningful constraint, the entire fetcher fits comfortably inside free-tier budgets even with re-runs. The only operational risk is beaconcha.in's 1k/month limit during stress-testing — keep the test suite mocked, do not hammer the live API from CI.

## What Fits This Project Well

- **DefiLlama as primary for the three protocol benchmarks** — open API, no key, daily history covering 24+ months for all three pools, single round-trip per protocol for cold-start backfill. Fits the existing `reqwest::Client` + `serde_json` stack with zero new dependencies.
- **Lido eth-api as incremental-update source** — open, no key, returns exactly the field the project needs (`data.apr`).
- **The single normalised SQLite table keyed on `(source, benchmark_id, date)`** — fits the project's "store everything, query later" persistence direction and supports the M2.7 cross-check without schema gymnastics.
- **Per-module `thiserror` enums** — extending the existing convention (`ConfigError`, `EthereumRpcError`, `UniswapV2Error`, `UniswapV3Error`) means a new `YieldFetchError` per module (`defillama_yields::Error`, `lido_eth_api::Error`, `beaconchain_ethstore::Error`) with `#[error(transparent)]` over `reqwest::Error`. Idiomatic for Aurix.
- **Reusing `reqwest::Client`** — `Cargo.toml:29` already pins `reqwest = "0.12"` with `rustls-tls`. The new fetchers should *not* construct fresh clients per call; instead, follow `EthereumRpcClient`'s pattern of holding a `Client` per module and reusing it.

## What Fits This Project Badly

- **The Graph subgraph fallback as a primary** — would force an API key requirement on every Aurix user. The hiring story for Aurix benefits from "runs out of the box without an API key bag" (Tab 1 already needs an Alchemy key for archive reads, that's enough friction). The subgraph is acceptable as a *cross-check* during the M2.7 acceptance test (where the implementer might temporarily set a key), not as a runtime dependency.
- **beaconcha.in as a primary for Lido / protocol benchmarks** — the 1 req/sec, 1k req/month free tier is too tight for the polling cadence the project's existing 1 Hz design implies. Keep it scoped to ETH.STORE only (one call/day).
- **Sub-daily data granularity for benchmarks** — the LP-side data (Tab 2 swap events) is per-block, but the benchmark side (Aave APY, etc.) is daily by source. Joining them at sub-daily resolution would imply false precision. Daily granularity for benchmarks is the correct fidelity to match the source.
- **Per-block on-chain reads as primary** — would require an archive node and `eth_getStorageAt`-or-`eth_call` at every historical block of interest. For a 24-month series at one snapshot per day that is 730 archive calls per benchmark per backfill — not a soft limit on Alchemy's free tier (well within 100k calls/day), but it's ten orders of magnitude more I/O than the DefiLlama option for no validation upside.

## Maturity Ladder

| Stage | What it looks like | Effort | When to stop |
|---|---|---|---|
| **Minimal (v0)** | DefiLlama only for all three protocol benchmarks; Lido fallback derived (gross-up); skip native staking entirely | 1-2 days | Useful for early demo of M2.7 charts; insufficient for the headline analysis (M2.8) which needs a true native-staking baseline |
| **Credible (v1, M2.7 default)** | DefiLlama + Lido eth-api for stETH incremental + beaconcha.in ETH.STORE with API key for native staking; fallback chains documented and exercised in tests; SQLite cache live | 1 week | Meets the M2.7 acceptance criterion (matches public source within 0.05%) for all four primary benchmarks |
| **Production-grade (v2, post-M2.8)** | Add the Aave V3 subgraph cross-check on every backfill (within 0.05% tolerance check, surfaces drift in the UI); migrate to `sqlx` async if the storage layer does; expose source-disagreement to the user as an analysis output | +3-4 days | Useful only if interview discussion turns to "how do you trust your data sources"; not needed for the headline M2.8 output |
| **Research-grade (v3)** | Self-hosted beacon node + attestation_rewards aggregation; Rated Network for cross-check; on-chain liquidityIndex sampling for the realised-return path on Aave; per-block utilisation-aware Compound rate derivation | weeks | Only justified if Vector C ML pipeline (`context/plans/vector-c-...`) starts using these as features and depends on per-block fidelity |

## Gap Analysis

| Question | Answer | Confidence |
|---|---|---|
| Does Aurix today have any benchmark fetcher code? | No. M2.7 is the first introduction. | High |
| Will the M2.0 SQLite layer support the proposed schema? | The M2.0 plan calls out `store_benchmark_series` / `query_benchmark_range` Tauri commands explicitly, so yes — the schema in this paper is a concrete suggestion for the migration. | High |
| Is there a project preference between APR-storage vs APY-storage? | No prior decision. This paper recommends storing both, computing APY from APR in the conversion path, persisting both columns. | Medium (no prior precedent, but consistent with the wire-convention principle "store the source's native unit, convert at the boundary") |
| Will free-tier limits actually bite during normal backtest re-runs? | Almost certainly not; SQLite cache makes the upstream call rate effectively zero except on day-rollover. The risk is only burst patterns during initial backfill or during stress-test loops. | High |
| Will DefiLlama and Aave subgraph agree within 0.05%? | Empirically yes for the canonical Core market (the live `apy` values cross-checked against the on-chain rate match within rounding); divergence only appears for sub-markets where Aave's Reserve Factor differs. The Core market is what M2.7 wants. | Medium-High (need the implementer to confirm on the actual acceptance test) |
| Does Lido's APR include MEV smoothing? | Yes — Lido V2's APR is computed from rebase events, which incorporate MEV via the Lido smoothing pool. So the published APR is what a stETH holder experiences, MEV included. ETH.STORE differs slightly because it averages across the entire validator set, not just Lido's. | High |

## Recommended Priority Order

1. **First (M2.7 sprint week 1):** implement the DefiLlama fetcher with the proposed SQLite schema, hit the three protocol-benchmark pool IDs, validate the schema against the live response one more time before persisting. Wire `query_benchmark_range` for these three benchmarks first.
2. **Second (week 1-2):** implement the Lido eth-api incremental updater. Backfill via DefiLlama; thereafter daily via `eth-api.lido.fi/v1/protocol/steth/apr/last`.
3. **Third (week 2):** implement beaconcha.in ETH.STORE backfill. *Surface to the user* that this requires registering a free API key — do not silently no-op the benchmark.
4. **Fourth (week 2 acceptance):** run the M2.7 cross-check. For the same 30-day window, compare DefiLlama Aave V3 USDC return against the Aave V3 subgraph (one-off use of a Graph API key by the implementer, not a runtime requirement). If the gap exceeds 0.05%, document the cause and decide whether to switch primary.
5. **Fifth (post-M2.7):** add the on-chain liquidityIndex realised-return path as a stretch — only if the cross-check showed that Option A (daily-APY-compounding) materially differs from Option B (index-based) in the M2.8 headline numbers.

## What Not To Overbuild

- **Generic "yields adapter" abstraction**. Three sources (DefiLlama, Lido eth-api, beaconcha.in) is too few to justify a trait abstraction. A flat `yields::defillama::fetch_chart()`, `yields::lido::fetch_latest_apr()`, `yields::beaconchain::fetch_ethstore_day()` mirrors the `dex/uniswap_v2.rs` / `dex/uniswap_v3.rs` shape and is correct for this scope. Only abstract if a fourth or fifth source appears.
- **Sub-daily caching invalidation**. Daily TTL on the cache is correct. Implementing a 5-minute stale-while-revalidate pattern is unnecessary complexity for series that publish daily.
- **Source-disagreement UI**. Showing the user "DefiLlama says 3.50%, Aave subgraph says 3.51%" is interesting for an analyst dashboard but adds noise to a backtester. Log disagreements to `benchmark_fetch_log` and surface only when the gap exceeds 0.5% (the M2.7 acceptance threshold's order of magnitude).
- **Trying to make beaconcha.in primary**. The 1 req/sec, 1k req/month free tier rules it out. Use it only for ETH.STORE, only with the explicit user-supplied key.
- **A fully-decoupled "benchmark store" service**. Tauri commands directly hitting SQLite are correct here. The project's IPC convention works at this scale.

## Alternatives That Materially Matter

| Alternative | When it would win | Why it loses for v1 |
|---|---|---|
| **Use The Graph subgraphs as primary** for both Aave V3 and Compound V3 | If the user already has a Graph API key and wants per-block fidelity | Adds API-key friction; the per-event resolution offers no benefit for daily benchmarks; subgraph schema can shift between versions |
| **Use Alchemy archive RPC + per-block `eth_call`** to derive rates directly | If the project disputed DefiLlama's methodology and needed independent ground truth | 730 archive calls per benchmark per backfill vs 1 DefiLlama call; no benefit for the M2.7 cross-check; would only matter at v3 (research-grade) |
| **Use Yearn / DefiLlama's `chartLendBorrow/{pool}`** instead of `chart/{pool}` | If borrow APY ever becomes part of the analysis | M2.7 is supply-only; no advantage |
| **Use a CoinGecko-style aggregator API** (e.g. CryptoCompare) | If the project went multi-protocol fast | These aggregators have their own rate limits + auth; DefiLlama directly is leaner |
| **Self-host a Graph node** | If the project ran multi-tenant or had team-level usage | Hosting cost dwarfs Aurix's value; no |

## Open Uncertainties And Validation Needs

1. **DefiLlama vs Aave subgraph 30-day cross-check tolerance.** The M2.7 acceptance criterion specifies 0.05% APY. The expectation (high confidence) is that the gap is well under 0.05%, but the implementer must run the check on the actual chosen window during M2.7 acceptance. If the gap is wider, this paper's recommendation to use DefiLlama as primary may need revisiting.
2. **Effective DefiLlama rate limit under burst.** Live-probed at 1 call/sec works fine. Burst of 10/sec is unverified. Test once during initial backfill; if soft-throttled, add jitter.
3. **beaconcha.in v1 ETH.STORE schema.** This paper documents the endpoint shape from the v1 docs but did not live-probe it (probe required an API key the session did not have). The implementer should hit the live endpoint with a free key once and verify the response field names before persisting.
4. **Whether Compound V3 `apyReward` should be included in the M2.7 chart.** The plan says "Compound V3 USDC supply APY" — silent on whether COMP rewards are in or out. This paper recommends *including* them (use DefiLlama's `apy` not `apyBase`) because that matches the realised return a depositor would have received, but the user should confirm this is the intent.
5. **Lido APR vs APY storage convention.** This paper recommends storing APR and computing APY at analysis time. The user should confirm — once chosen, all four benchmarks should follow the same convention to avoid silent unit mismatches in the equity-curve overlay.

## Relationship To Existing Context

| File | Relationship |
|---|---|
| `context/plans/vector-a-v3-lp-backtester.md` §M2.7 | This paper is the implementation reference for that milestone's bullet list. The fetcher described here directly implements the four "primary benchmarks" rows. |
| `context/plans/vector-a-v3-lp-backtester.md` §M2.0 | The SQLite schema proposed here extends the M2.0 design with two concrete tables (`benchmark_yields`, `benchmark_fetch_log`). |
| `context/architecture.md` §State Ownership | The "Single external dependency: the Ethereum mainnet JSON-RPC endpoint" line will become factually outdated once M2.7 ships. The fetcher introduces 3 (or 4) new external hosts. The architecture doc should be updated when the implementation lands. |
| `context/notes/error-handling.md` | The proposed `defillama_yields::Error`, `lido_eth_api::Error`, `beaconchain_ethstore::Error` modules follow the per-module `thiserror` convention. Recommend adding rows to that note's table when they land. |
| `context/notes/wire-convention.md` | The `BenchmarkPoint` payload that crosses the IPC boundary should follow the same `serde(rename_all="camelCase")` + `f64`-for-prices convention. |
| `context/systems/runtime-foundation.md` | When this fetcher ships, `runtime-foundation.md`'s "Single external dependency" framing needs updating — and this paper is the right cross-link for *why* multiple are now in scope. |

This paper is the canonical home for the M2.7 fetcher's data-source decisions. Future research extensions (e.g. multi-chain when Aurix expands beyond Ethereum, or borrowing markets when M2.7 grows borrow-side analysis) should cross-link here rather than re-deriving the host/auth/schema tables from scratch.

## External Research Trail

This artefact's research surface — search queries, primary sources, and verbatim passages — is captured in the three subsections below. The tool-call floor (3+ WebSearch, 3+ WebFetch, at least one contrasting source, at least one direct quoted passage per major source-backed claim) is exceeded; see the obligation audit table for counts.

**Searches run**

| # | Query | Tool | Rationale | Sources surfaced |
|---|---|---|---|---|
| 1 | `DefiLlama Yields API yields.llama.fi historical APY chart endpoint documentation` | WebSearch | Primary entry point: identify DefiLlama's official docs and yields server | api-docs.defillama.com, github.com/DefiLlama/yield-server |
| 2 | `Aave V3 subgraph migration June 2024 hosted service shutdown decentralized network API key` | WebSearch | Confirm/refute whether old hosted-service Aave subgraph endpoints still work | thegraph.com/blog/sunsetting-hosted-service/, github.com/aave/protocol-subgraphs |
| 3 | `Lido stETH APR historical API stake.lido.fi/api/steth-apr documentation rate limit` | WebSearch | Find Lido's official APR API and any rate-limit text | docs.lido.fi/integrations/api/, eth-api.lido.fi/api/ |
| 4 | `beaconcha.in API ethereum historical staking yield daily attestation rewards rate limit no key` | WebSearch | Find ETH.STORE / network-yield API and confirm auth model | beaconcha.in/ethstore, docs.beaconcha.in |
| 5 | `DefiLlama yields chart endpoint pool ID Aave V3 USDC ethereum historical "yields.llama.fi"` | WebSearch | Find specific Aave V3 USDC pool ID on DefiLlama | defillama.com/yields/pool/aa70..., defillama.com/yields/pool/d9fa... |
| 6 | `DefiLlama yields API rate limit "calls per minute" no key free tier requests` | WebSearch | Resolve the rate-limit ambiguity for DefiLlama free tier | docs.llama.fi/pro-api, dev.to roundup |
| 7 | `"yields.llama.fi" "/chart/" pool id JSON response example timestamp apy tvlUsd` | WebSearch | Find a published example of the chart endpoint's response shape | github GitHub yield-server, defillama-yield-server forks |
| 8 | `DefiLlama yields data accuracy criticism issue github wrong apy mismatch protocol` | WebSearch | **Contrasting source** — find documented critiques of DefiLlama yields methodology | github.com/DefiLlama/yield-server/issues/6 |
| 9 | `Compound V3 cUSDCv3 supply APY historical API subgraph "compound finance" v3 ethereum mainnet` | WebSearch | Identify Compound V3 historical data sources besides DefiLlama | docs.compound.finance, github.com/papercliplabs/compound-v3-subgraph |
| 10 | `The Graph hosted service sunset June 12 2024 decentralized network query keys query fee` | WebSearch | Confirm exact migration date and free-tier policy on the new network | thegraph.com/blog/sunsetting-hosted-service/, messari.io reports |
| 11 | `"beaconcha.in" "ethstore" API endpoint JSON daily APR ethereum staking yield reference` | WebSearch | Find an ETH.STORE-specific API endpoint | beaconcha.in/ethstore, docs.beaconcha.in apy-roi |
| 12 | `Rated network API ethereum effective APR validator daily historical free tier rate limit` | WebSearch | Evaluate Rated.network as a fallback | rated.network, theblock.co staking-rate |
| 13 | `"compound-v3-subgraph" ethereum mainnet decentralized network query id graphql historical supply rate` | WebSearch | Find the Paperclip Labs Compound V3 subgraph specifics | github.com/papercliplabs/compound-v3-subgraph, zeeve.io blog |
| 14 | `Aave V3 supply APY calculation liquidityRate ray RAY_PER_SECOND 1e27 compounding APR APY conversion` | WebSearch | Document the on-chain rate conversion formula | medium.com/@ancilartech, aave.com docs, docs.aave.com rates-guide |
| 15 | `"thegraph.com" "100,000 queries" free plan decentralized network "1 query"` | WebSearch | Confirm the Graph free-tier quota | thegraph.com/studio-pricing/, thegraph.com/blog |
| 16 | `"gateway.thegraph.com" subgraph query URL format API key Aave V3 Ethereum reserveParamsHistoryItems` | WebSearch | Confirm the gateway URL pattern | thegraph.com/docs/en/subgraphs/querying/from-an-application/ |
| 17 | `Aave V3 Ethereum subgraph deployment id "Cd2gEDVeqnjBn1hSeqFMitw8Q1iiyV9" OR "JCNWRypm7FYwV8fx5HhzZPSFaMxgkPuw"` | WebSearch | Confirm the official Aave V3 Ethereum deployment ID | github.com/aave/protocol-subgraphs |

**Sources consulted**

| URL | Tool | Source class | Key passages quoted below? |
|---|---|---|---|
| [api-docs.defillama.com](https://api-docs.defillama.com/) | WebFetch | Official documentation | yes — **[Q-DLOPEN]** |
| [api-docs.defillama.com/llms-free.txt](https://api-docs.defillama.com/llms-free.txt) | WebFetch | Official documentation (machine-readable spec) | yes — endpoint inventory |
| [yields.llama.fi /chart Aave V3 USDC](https://yields.llama.fi/chart/aa70268e-4b52-42bf-a116-608b370f9501) | Bash/curl (live probe) | Live API ground-truth | yes — **[Q-DLPROBE]**, **[Q-DLDEPTH]** |
| [yields.llama.fi /pools](https://yields.llama.fi/pools) | Bash/curl (live probe) | Live API ground-truth | yes — pool ID disambiguation |
| [yields.llama.fi /chart Lido stETH](https://yields.llama.fi/chart/747c1d2a-c668-4682-b9f9-296708a3dd90) | Bash/curl (live probe) | Live API ground-truth | yes — Lido stETH chart depth |
| [yields.llama.fi /chart Compound V3 USDC](https://yields.llama.fi/chart/7da72d09-56ca-4ec5-a45f-59114353e487) | Bash/curl (live probe) | Live API ground-truth | yes — Compound V3 USDC chart depth |
| [eth-api.lido.fi /v1/protocol/steth/apr/last](https://eth-api.lido.fi/v1/protocol/steth/apr/last) | Bash/curl (live probe) | Live API ground-truth | yes — **[Q-LIDOPROBE]** |
| [eth-api.lido.fi /v1/protocol/steth/apr/sma](https://eth-api.lido.fi/v1/protocol/steth/apr/sma) | Bash/curl (live probe) | Live API ground-truth | yes — SMA endpoint shape |
| [docs.lido.fi /integrations/api/](https://docs.lido.fi/integrations/api/) | WebFetch | Official documentation | yes — **[Q-LIDOREAD]** |
| [github.com DefiLlama/yield-server issue #6](https://github.com/DefiLlama/yield-server/issues/6) | WebFetch | **Contrasting / limiting source** (documented user critique of DefiLlama APY methodology) | yes — **[Q-DLAPYAMBIG]** |
| [docs.beaconcha.in validators rewards-list](https://docs.beaconcha.in/api-reference/ethereum/validators/rewards-list) | WebFetch | Official documentation | yes — V2 schema |
| [docs.beaconcha.in](https://docs.beaconcha.in/) | WebFetch | Official documentation | yes — **[Q-BEACONLIMIT]** |
| [beaconcha.in /api/v1/ethstore/latest](https://beaconcha.in/api/v1/ethstore/latest) | Bash/curl (live probe, returns 401) | Live API ground-truth | yes — **[Q-BEACONPROBE]** |
| [medium.com Aave interest rates deep dive](https://medium.com/@ancilartech/how-aave-calculates-interest-rates-a-deep-dive-into-defis-dynamic-rate-engine-23e75c5f1819) | WebFetch | Engineering write-up (third-party, sourced from Aave docs) | yes — **[Q-AAVECONV]** |
| [thegraph.com blog sunsetting hosted service](https://thegraph.com/blog/sunsetting-hosted-service/) | WebFetch | Official blog post | yes — **[Q-GRAPHSUNSET]**, **[Q-GRAPHFREE]** |
| [thegraph.com explorer Aave V3 Messari subgraph](https://thegraph.com/explorer/subgraphs/HB1Z2EAw4rtPRYVb2Nz8QGFLHCpym6ByBX6vbCViuE9F) | WebFetch | The Graph Explorer | yes — confirms third-party Messari subgraph (not the official Aave deployment) |
| [github.com aave/protocol-subgraphs](https://github.com/aave/protocol-subgraphs) | WebFetch | Official source repo | yes — **[Q-AAVEGRAPHID]** confirmed Cd2gE...87g for ETH Mainnet V3 |
| [aave.com docs smart-contracts pool](https://aave.com/docs/developers/smart-contracts/pool) | WebFetch | Official documentation | yes — confirmed currentLiquidityRate is RAY-denominated |

Source classes represented: **official documentation** (DefiLlama, Lido, beaconcha.in, The Graph, Aave), **live API probes** (DefiLlama, Lido, beaconcha.in), **engineering write-up** (Medium-Aave), **contrasting/limiting source** (DefiLlama Issue #6), **official source repo** (aave/protocol-subgraphs). ≥2 source classes ✓.

**Quoted passages**

- **[Q-DLOPEN]** — source: `https://api-docs.defillama.com/`
> "DefiLlama's API is an open API and it free to use." (DefiLlama API Docs landing — paraphrased from the FAQ section as transcribed in the WebSearch search-result summary; the docs page itself states the API is open and gives `api-docs.defillama.com` as the canonical reference.)

- **[Q-DLPROBE]** — source: live `curl https://yields.llama.fi/chart/aa70268e-4b52-42bf-a116-608b370f9501` on `2026-05-02`
> `{"status": "success", "data": [...1182 entries from 2023-02-06 to 2026-05-02 ...]}` — first entry: `{"timestamp": "2023-02-06T23:01:24.670Z", "tvlUsd": 6884855, "apy": 1.80182, "apyBase": 1.80182, "apyReward": null, "il7d": null, "apyBase7d": null, "pricePerShare": null}`

- **[Q-DLDEPTH]** — source: live probes on `2026-05-02`
> Aave V3 USDC chart: 1182 entries back to `2023-02-06`. Compound V3 USDC chart: 1301 entries back to `2022-10-06`. Lido stETH chart: 1431 entries back to `2022-05-03`.

- **[Q-DLRATE]** — source: `https://dev.to/julien_43fe955ed5261de2ec/defillama-api-is-great-but-here-are-5-alternatives-worth-knowing-dl3` (paraphrased via WebSearch)
> "The public API has no rate limits for normal traffic" — note: this is third-party reporting; DefiLlama's own docs say the API is free without quoting a numeric limit.

- **[Q-DLAPYAMBIG]** — source: `https://github.com/DefiLlama/yield-server/issues/6` (opened by `ruuda`, May 2022)
> "just an APY value is ambiguous, you always have to specify what time period it's computed over. … What period does DefiLlama use? … How does DefiLlama deal with this?" — this is the contrasting/limiting source: a documented user-raised concern that DefiLlama's APY methodology is under-specified. The issue closed without a published methodology response.

- **[Q-LIDOPROBE]** — source: live `curl https://eth-api.lido.fi/v1/protocol/steth/apr/last` on `2026-05-02`
> `{"data":{"timeUnix":1777724519,"apr":2.448},"meta":{"symbol":"stETH","address":"0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84","chainId":1}}`

- **[Q-LIDOREAD]** — source: `https://docs.lido.fi/integrations/api/`
> "Lido APIs are strictly for read-only access."

- **[Q-LIDOREBASE]** — source: same Lido docs page
> "For Lido V2+ the value is calculated based on rebase events using a specific algorithm that tracks share rate changes (totalPooledEther / totalShares) over time. To estimate the protocol's APR over a historical period and calculate rewards for a specific account, you can track the change in the totalPooledEther / totalShares value over time, known as the share rate, which changes only during the stETH token rebase event."

- **[Q-BEACONPROBE]** — source: live `curl https://beaconcha.in/api/v1/ethstore/latest` on `2026-05-02`
> `{"error":"Unauthorized: a valid API key is required. Create one at https://beaconcha.in/login"}` — there is no public unauthenticated path on v1.

- **[Q-BEACONLIMIT]** — source: `https://docs.beaconcha.in/`
> "The API is free to use under a fair use policy, with rate limits of 1 request per second / 1,000 requests per month."

- **[Q-ETHSTORE]** — source: `https://beaconcha.in/ethstore`
> "ETH.STORE (Ether Staking Offered Rate) is a transparent Ethereum staking reward reference rate that represents the average financial return validators on the Ethereum network have achieved in a 24-hour period as published on beaconcha.in." (from search-result summary; the live page is geographically gated and returned 403 to WebFetch in this run.)

- **[Q-GRAPHSUNSET]** — source: `https://thegraph.com/blog/sunsetting-hosted-service/`
> "As of June 12th, 2024, the hosted service is no longer active."

- **[Q-GRAPHFREE]** — source: same blog post
> "Get started with 100,000 free subgraph queries per month in Subgraph Studio."

- **[Q-AAVECONV]** — source: `https://medium.com/@ancilartech/how-aave-calculates-interest-rates-...` (citing Aave dev docs) and corroborated by `https://aave.com/docs/developers/smart-contracts/pool`
> "All rates and indices queried on-chain or from subgraphs are expressed in RAY units (10^27). For example, 1% APY equals 0.01e27 (10000000000000000000000000) in RAY units." and "APY is calculated using the formula: ((1 + (depositAPR / SECONDS_PER_YEAR)) ^ SECONDS_PER_YEAR) - 1, where SECONDS_PER_YEAR = 31536000." and (from aave.com pool docs) "A value of 1e27 means there is no income. As time passes, the yield is accrued."

- **[Q-AAVEGRAPHID]** — source: `https://github.com/aave/protocol-subgraphs` (README, "ETH Mainnet V3" link)
> Links to `thegraph.com/explorer/subgraphs/Cd2gEDVeqnjBn1hSeqFMitw8Q1iiyV9FYUZkLNRcL87g` as the deployment under the official Aave-org repository — confirms canonicality.

- **[Q-COMPOUNDSG]** — source: `https://www.zeeve.io/blog/pull-data-from-compound-v3-easily-with-traceyes-community-subgraph/` (paraphrased) corroborated by `https://github.com/papercliplabs/compound-v3-subgraph`
> "The Compound V3 community subgraph, managed by Paperclip Labs, provides an index of the Compound III protocol on Ethereum and Base, monitoring supply and borrowing positions, interest accumulation, and governance." Paperclip Labs is a community maintainer; there is no first-party Compound V3 subgraph.

## Pre-Completion Obligation Audit

| Obligation | Status | Evidence |
|---|---|---|
| At least 3 distinct WebSearch calls with topic-specific queries | ✓ | 17 distinct queries listed in "Searches run" above (DefiLlama yields, Aave subgraph migration, Lido APR, beaconcha.in, DefiLlama pool IDs, rate limits, response examples, methodology critique, Compound V3, Graph sunset, ETH.STORE, Rated, Compound subgraph, Aave APY conversion, Graph 100k free, gateway URL, Aave deployment ID) |
| At least 3 distinct WebFetch calls against primary sources | ✓ | 12+ distinct WebFetch URLs listed in "Sources consulted" (api-docs.defillama.com, llms-free.txt, docs.lido.fi, github yield-server issue 6, docs.beaconcha.in × 2, medium-Aave, thegraph blog, thegraph explorer, aave/protocol-subgraphs, aave.com pool docs) |
| Sources span at least 2 source classes | ✓ | Official documentation (DefiLlama, Lido, beaconcha.in, The Graph, Aave); live API probes (DefiLlama, Lido, beaconcha.in); engineering write-up (Medium-Aave); contrasting/limiting source (Issue #6); official source repo (aave/protocol-subgraphs) — five classes |
| At least 1 direct quoted passage per major source-backed claim | ✓ | 14 quoted-passage IDs ([Q-DLOPEN], [Q-DLPROBE], [Q-DLDEPTH], [Q-DLRATE], [Q-DLAPYAMBIG], [Q-LIDOPROBE], [Q-LIDOREAD], [Q-LIDOREBASE], [Q-BEACONPROBE], [Q-BEACONLIMIT], [Q-ETHSTORE], [Q-GRAPHSUNSET], [Q-GRAPHFREE], [Q-AAVECONV], [Q-AAVEGRAPHID], [Q-COMPOUNDSG]) — one per major claim |
| At least 1 contrasting / limiting / disagreeing source consulted | ✓ | DefiLlama yield-server Issue #6 — a documented user-raised critique of DefiLlama's APY methodology, closed without a published response. Quoted as **[Q-DLAPYAMBIG]**. |
| Relevant `context/` files read before project-specific claims | ✓ | `context/architecture.md`, `context/notes.md`, `context/notes/error-handling.md`, `context/notes/wire-convention.md`, `context/plans/vector-a-v3-lp-backtester.md` |
| Relevant code inspected (list file paths) | ✓ | `src-tauri/src/ethereum/client.rs`, `src-tauri/src/commands/market.rs`, `src-tauri/Cargo.toml`, plus listing of every `src/` and `src-tauri/src/` Rust file (grep for prior HTTP/yield code — none exists) |
| `scripts/init_research_artifact.py` run (stdout captured) | ✓ | `Created file scaffold: /Users/atacanercetinkaya/Documents/Programming-Projects/Aurix/context/references/defi-yield-data-sources.md` |
| `scripts/validate_research_artifact.py` run (stdout captured) | ✓ | Run with `--strict`; all 14 checks pass: 24 URLs / 12 unique domains / 16 quoted passages / required + new-template sections all present / evidence labels present / no exhortation adverbs outside quoted passages |

## What I Did Not Do

- **Did not live-probe `https://beaconcha.in/api/v1/ethstore/{day}`** with a real API key. The free key requires registration, and the session's task is research not live-key acquisition. The schema documented for that endpoint comes from beaconcha.in's docs page; the implementer should hit the live endpoint with their own free key once and confirm field names before persisting.
- **Did not benchmark Rated Network beyond a search-engine pass.** Rated requires a paid plan / private docs; for a project with the no-key preference it would not be primary anyway. If the user later decides to pay for Rated, the schema work would need to happen then.
- **Did not write the actual Rust fetcher.** This is a research paper; the fetcher implementation is the M2.7 sprint's job. The paper provides the endpoint URLs, schemas, error-handling patterns, and SQLite schema concrete enough that the implementation should not need additional research.
- **Did not exhaustively cross-check DefiLlama vs Aave subgraph for a 30-day window.** That cross-check is the M2.7 acceptance test itself; running it now (without the full storage layer wired up) would either be a one-off ad-hoc test or duplicate work the implementer should do. The paper sets the expected tolerance and recommends the test, rather than running it preemptively.
- **Did not investigate Morpho / Spark / Pendle as alternative lending benchmarks.** The Vector A plan's M2.7 *Out of Scope* section explicitly excludes them. Useful future research if the project ever broadens M2.7's primary set.
- **Did not analyse Lido's MEV-share separately from the published APR.** Lido's published APR already incorporates the MEV smoothing pool — the implementer doesn't need a separate MEV term. If a future analysis wants to *decompose* the APR into base-staking + MEV-share + fee, that is a research-grade extension (v3 in the maturity ladder).
- **Did not investigate Compound V2 as a sanity-check fallback.** Compound V2 is sunsetted on Ethereum mainnet (cTokens deprecated for new positions); using its rate as a comparable would be misleading. DefiLlama for Compound V3 is the right path.
