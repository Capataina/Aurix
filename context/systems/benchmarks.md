# Benchmarks

## Scope / Purpose

- Multi-asset benchmark series for the LP backtester's headline verdict — fetches and persists daily reference series for DeFi (Aave V3 USDC, Compound V3 USDC, Lido stETH), TradFi (FRED 3-month Treasury, FRED S&P 500, FRED LBMA gold), Ethereum staking (beaconcha.in ETH.STORE), V2 LP synthetic constant-product baseline, and HODL.
- Provides the apples-to-apples comparison surface for "should you have just held / staked / yielded instead?" — the question the M2.8 capital-allocation [headline](headline.md) verdict answers.

## Boundaries / Ownership

- Owns: per-provider HTTP fetchers (DefiLlama, FRED, beaconcha.in), per-source response-shape parsing, daily-series normalisation, the alpha-decomposition module (period + rolling 30/60/90), V2 LP synthetic constant-product baseline computation, persistence to [storage](storage.md)'s `benchmark_series` table.
- Does **not** own: the verdict prose synthesis (that's [headline](headline.md)), the storage schema (that's [storage](storage.md)).

## Current Implemented Reality

```text
benchmarks/
├── mod.rs              # public surface + provider-key constants
├── error.rs            # BenchmarkError thiserror enum
├── http.rs             # ReqwestFetcher + MockHttpFetcher trait
├── defi.rs             # DefiLlamaProvider — Aave / Compound / Lido pool APYs
├── tradfi.rs           # TradFiProvider — FRED CSV parsing for DGS3MO, SP500, gold
├── beaconchain.rs      # ETH.STORE staking yield (KEY_REQUIRED)
├── v2lp.rs             # Synthetic V2 constant-product LP baseline
├── hodl.rs             # HODL price-only baseline
└── alpha.rs            # Alpha decomposition vs each benchmark (period + rolling)
```

**Free-data constraint.** All sources except beaconcha.in (`KEY_REQUIRED`) are usable without API keys per the project's `no-paid-APIs` policy. The Stooq → FRED SP500 swap in commit 391eadd was driven by Stooq adding an API key requirement; FRED remains key-free.

**Daily normalisation.** Every series is normalised to a `BenchmarkPoint { series_key, date, value }` shape. Daily-cadence series go in directly; sub-daily series (some FRED ones) are aggregated to end-of-day. The `series_key` is the canonical identifier used by [headline](headline.md) and the LP page UI.

**Alpha decomposition.** `alpha.rs` computes:
- Period alpha — total return over the backtest window minus the benchmark's total return over the same window.
- Rolling 30/60/90-day alpha — windowed alpha series for the chart UI.

## Key Interfaces / Data Flow

| Boundary | Direction | Shape | Notes |
|---|---|---|---|
| `commands::lp::lp_fetch_benchmark_series(series_key)` → benchmarks | inbound | series_key string | dispatches to the right provider via match block |
| benchmarks → external API | outbound | HTTP GET via `ReqwestFetcher` | keyed by series; DefiLlama/FRED/beaconcha.in |
| benchmarks → [storage](storage.md) | outbound (write) | `Vec<BenchmarkPoint>` via `insert_benchmark_points_batch` | replace-on-duplicate-key (per `storage/benchmarks.rs::tests::replace_on_duplicate_key`) |
| `commands::lp::lp_query_benchmark_range(series_key, start, end)` → storage | outbound (read) | `Vec<BenchmarkPoint>` | for chart re-render path |
| benchmarks → [headline](headline.md) | outbound (data) | persisted benchmark series | composed with backtest results in M2.8 verdict |

## Implemented Outputs / Artifacts

- `benchmark_series` rows in [storage](storage.md).
- 4+ tests in `storage/benchmarks.rs` (batch insert + range query + replace-on-duplicate).

## Known Issues / Active Risks

- **HTTP timeout** is 15s per fetch (raised from previously-hanging Aave/Lido fetches in commit 391eadd). External API outages still cause user-facing latency; no retry-with-backoff yet.
- **beaconcha.in API key requirement** surfaces as `IngestError::KeyRequired("beaconchain")` per the global KEY_REQUIRED contract. The frontend can prompt the user to configure a key; until then, the ETH.STORE row is missing from the headline verdict.
- **Stooq swap fragility.** The legacy `stooq_voo` series_key is preserved for back-compat but transparently routes to the FRED SP500 series. If FRED ever changes the SP500.txt URL or format, both `stooq_voo` and `fred_sp500` series break simultaneously. Documented in `notes/lp-backtester-data-sources.md`.

## Partial / In Progress

- None.

## Planned / Missing / Likely Changes

- `reqwest::Client` reuse via `Lazy<Client>` (recorded as a perf finding in [audit findings](../plans/code-health-audit/ipc-commands.md) §"Reuse the reqwest::Client").
- Possible move to `MockHttpFetcher`-based integration testing for the per-provider parse logic (currently the providers are tested via the storage round-trip layer, not at the fetch boundary).
- Retry-with-backoff for transient external-API failures.

## Durable Notes / Discarded Approaches

- **DefiLlama over individual lender APIs.** Aurix originally considered hitting Aave's, Compound's, and Lido's APIs directly; DefiLlama aggregates them with one canonical normalisation, which both simplifies the code and produces consistent decimal-rate semantics across providers. Documented in `references/defi-yield-data-sources.md`.
- **FRED over Stooq for S&P 500.** Stooq's API now requires a key; FRED is free, and provides additional macro series Aurix may need anyway (T-bill rates, gold). Migration in commit 391eadd. Documented in `references/tradfi-benchmark-data-sources.md`.
- **Replace-on-duplicate-key over insert-or-ignore for benchmark points.** Benchmark series can be revised by their providers (e.g. FRED revises older S&P prints); replacing on `(series_key, date)` collision keeps the cache aligned with provider truth. Storage tests pin this behaviour.

## Obsolete / No Longer Relevant

- None.

## Cross-references

- Caller: `commands::lp::lp_fetch_benchmark_series`, `commands::lp::lp_query_benchmark_range`.
- Consumer of: [storage](storage.md) (write benchmark_series rows), external APIs (DefiLlama, FRED, beaconcha.in).
- Producer for: benchmark series consumed by [headline](headline.md).
- Related research: `references/defi-yield-data-sources.md`, `references/tradfi-benchmark-data-sources.md`.
- Related notes: `notes/lp-backtester-data-sources.md`, `notes/free-data-fallback-chain.md`.
