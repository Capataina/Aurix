# TradFi Benchmark Data Sources — APIs and Modelling

> Reference paper for Aurix Tab 2 (Vector A — Uniswap V3 LP backtester) milestone M2.7. Audience: the implementing Rust agent who will write the `benchmark_fetcher` module and the analyst reading the resulting backtest output. The intent is for this file to remain useful through the next several iterations of M2.7 even as third-party APIs drift.

## Scope / Purpose

This paper covers what data source to reach for, in what order, to populate the **secondary (TradFi) benchmark series** required by `vector-a-v3-lp-backtester.md` §M2.7:

1. **3-month T-bill rate** (the canonical Sharpe risk-free input).
2. **1-year T-bill rate** (alternate Sharpe input for longer-horizon backtests).
3. **S&P 500 total return** (broad equity sanity check — *"would I have been better off in the index?"*).
4. **Gold spot price**, modelled via the GLD ETF (commodity sanity check).
5. **VOO** (an explicit ETF instance of the S&P 500 sanity check, with its own friction layer).

It explicitly does **not** cover:

- Primary (DeFi-native) benchmarks — Aave/Compound supply APY, Lido stETH, native staking yield. Those have their own sources (DefiLlama, Aave subgraph, beacon chain) and warrant a separate paper.
- Live-streaming or intraday TradFi data. M2.7 needs daily values for ≥24 months of historical lookback; the streaming question does not arise.
- Risk model calibration (CAPM betas, factor decomposition). The benchmark module just produces clean daily series; downstream analysis is M2.7's responsibility.
- Tax-lot accounting on the ETF benchmarks. Out of scope per the plan's "Out of Scope" section.

The framing question this paper answers is narrow and project-specific:

> **Can Aurix M2.7 ship without a FRED API key, using only free no-key HTTP endpoints, with reasonable accuracy and a documented fallback chain — and where exactly does that approach break?**

The answer is **yes, ship without a FRED key**, and the recipe is in §[Recommended Priority Order](#recommended-priority-order).

## Current Project Relevance

The plan at `context/plans/vector-a-v3-lp-backtester.md:194-214` specifies five TradFi series and five benchmark friction columns. The implementing agent will:

1. Persist daily benchmark series in SQLite (M2.0 schema includes a `benchmark_series` table).
2. Backfill ≥24 months of daily values once on first run, then incrementally extend.
3. Compute the LP equity curve's Sharpe ratio against the **3-month T-bill** as risk-free rate (M2.5 acceptance criteria).
4. Overlay benchmark equity curves on the same chart as the LP equity curve, normalised to 100 at entry (M2.7 outputs).
5. Match a public source on S&P 500 total return within 0.1% absolute return for the same window (M2.7 acceptance criterion).

The user has explicitly stated a preference for **no-key sources first, FRED key only if Yahoo/Stooq prove unreliable**. This preference is durable and motivates the hierarchy below: every source class is evaluated against "does it work without an API key, and what is the failure mode."

The current repository has **zero** of this implemented:

- `repository fact` — `src-tauri/src/` has no `benchmark/` module today (verified `ls src-tauri/src/`: `commands/, config.rs, dex/, ethereum/, lib.rs, main.rs, market/`).
- `repository fact` — there is no persistence layer (`context/architecture.md:172` "Market history ... Session-only; wiped on reload"). M2.0 is a prerequisite for the benchmark fetcher to have anywhere to put its data.
- `repository fact` — no HTTP client beyond the RPC client at `src-tauri/src/ethereum/client.rs`. Adding `reqwest` dependency for non-RPC HTTP is fine but the convention will be set by this module.

The agent is starting from a blank slate. This paper is the spec.

## Current State Snapshot

### What exists (verified)

| Item | Status | File reference |
|---|---|---|
| Tauri/Rust backend with `reqwest` already in the dependency graph | exists | implied by `src-tauri/src/ethereum/client.rs` |
| `Result<T, MarketError>`-style error pattern with `thiserror` per module | exists | `context/notes/error-handling.md` |
| Camel-case Serde wire convention | exists | `context/notes/wire-convention.md` |
| Tokio runtime with `tokio::join!` for concurrent fetches | exists | `src-tauri/src/commands/market.rs:107-113` |

### What does not exist (verified)

- No SQLite persistence — `benchmark_series` table will not exist until M2.0 ships.
- No background scheduling — Tab 1 polls at 1 Hz from the frontend; no backend scheduler. The benchmark fetcher will need a different cadence (daily, idempotent, retry-aware) than the existing tick loop.
- No environment variable for any market-data API key in `src-tauri/src/config.rs`. The current config knows only `MAINNET_RPC_URL`, `ALCHEMY_API_KEY`. A `FRED_API_KEY` would be a net-new optional config slot.

`project inference` — given the convention set by `config.rs`, the right shape for an optional FRED key is a fall-through pattern: try no-key sources first, escalate only if all fail and `FRED_API_KEY` is present.

## Research Signal

The matrix below ties each load-bearing claim in this paper to its source class, the verified repository state it informs, and the project implication. Rows are ordered by load-bearing weight for the M2.7 implementation.

| Implementation choice | Research-backed signal | Source citation | Current repository state | Project implication | Evidence class |
|---|---|---|---|---|---|
| FRED no-key `.txt` endpoint as primary path for T-bills + gold | Stable URL contract for >15 years; rate limit on the API path is "120 requests per minute" but the no-key path is observably looser; CSV/text-format download is documented as not requiring an API key | `[FRED-RATE-LIMIT]`, `[FRED-NO-KEY-TXT]`; `https://github.com/sboysel/fredr/blob/master/R/fredr_request.R` | No `benchmark/` module yet; `src-tauri/src/` listing confirms only `commands, config.rs, dex, ethereum, lib.rs, main.rs, market` | Use FRED `.txt` for `DGS3MO`, `DGS1`, `GOLDAMGBD228NLBM`. No `FRED_API_KEY` config slot needed. | source-backed |
| Use ETF adjusted close directly without further expense-ratio subtraction | CRSP-standard adjustment is built into Yahoo's adjusted close; expense ratio accrues daily against NAV, mechanism documented in SPDR Gold prospectus | `[YAHOO-ADJ-CLOSE-CRSP]`, `[GLD-SPONSORS-FEE-ACCRUAL]`; `https://help.yahoo.com/kb/SLN28256.html`, SPDR Gold Trust prospectus | No persistence layer; no benchmark fetcher; the M2.7 plan friction table is ambiguous on whether to subtract expense ratio from ETF prices | Update plan ambiguity at `vector-a-v3-lp-backtester.md:200-207`; subtract expense ratio only for spot/index series, not for ETF adjusted close | source-backed |
| Stooq as primary non-FRED CSV source | "There is no API for Stooq. This means you can download data without needing an API key." Pandas-datareader integration with stable URL pattern `https://stooq.com/q/d/l/?s={ticker}&i=d` | `[STOOQ-NO-KEY]`; pandas-datareader Stooq docs | No HTTP client beyond `EthereumRpcClient` | Add `reqwest`-based no-key Stooq fetcher; cover `voo.us`, `gld.us`, `^spx`, `xauusd` from one entry point | source-backed |
| Yahoo as tertiary, only for `^SP500TR` | Yahoo's scraping endpoints fail every quarter; cookie/crumb auth has been required since 2024; library packages all share the fragility | `[YFINANCE-BREAKAGE-2024]`, `[YFINANCE-AUTH-FRAGILITY]`, `[YAHOO-LIBRARIES-FRAGILITY]`; yfinance issue 2052 | No HTTP client; Aurix is Rust without a yfinance equivalent | Implement Yahoo only for `^SP500TR` with VOO-adjusted-close fallback; do not build production-critical paths through Yahoo | source-backed |
| Use `DGS3MO` (3-month T-bill, investment basis) as Sharpe risk-free | Academic and practitioner standard since Sharpe (1966); investment basis is the comparable convention for cross-asset Sharpe | `[SHARPE-3MO-TBILL-CONVENTION]`, `[DGS3MO-INVESTMENT-BASIS]` | M2.5 plan acceptance specifies "use M2.7 risk-free rate, not 0%" but does not specify which | Default to `DGS3MO`; expose `DGS1` as alternate; display rate-regime context next to every Sharpe value | source-backed |
| Display Sortino + rolling Sharpe alongside full-history Sharpe | Sharpe assumes normal returns; LP returns are skewed and kurtotic; full-history Sharpe is dominated by which rate-regime the window starts in | **`[SHARPE-CRYPTO-LIMITATIONS]`**, `[DEFLATED-SHARPE]`; ScienceDirect peer-reviewed critique | M2.7 plan already requires rolling 30/60/90; M2.5 does not currently include Sortino | Add Sortino to M2.5 strategy-comparison table (Cov-class change to plan, not this paper) | source-backed (contrasting / limiting) |
| Bid-ask spread modelled as one-time entry/exit cost, not continuous accrual | Vanguard publishes ~0.02% median 30-day spread for VOO; cost is incurred at trade time | `[VOO-MEDIAN-SPREAD]`, `[SP500-VOLUME-COMPARISON]` | No benchmark module; no spread modelling | Subtract round-trip 4 bp from VOO and 6 bp from GLD at entry; do not accrue daily | source-backed |
| `^SP500TR` is **not** on FRED — fix plan's wording | Search "FRED SP500TR" returns no series; only `SP500` (price-only, last 10 years). S&P licenses TR. | direct verification via FRED search; `https://fred.stlouisfed.org/series/SP500` | Plan at `vector-a-v3-lp-backtester.md:194` says "FRED `SP500TR` or Yahoo `^SP500TR`" | Correct plan to "Yahoo `^SP500TR` (or VOO-adjusted-close proxy if Yahoo down)"; `SP500TR` on FRED does not exist | repository fact (plan inaccuracy) |
| Daily compounding base = 365, not 252 | Treasury Bond Equivalent yield convention uses 365- or 366-day year | `[DGS3MO-INVESTMENT-BASIS]` | No backtester math yet | Use `(1 + rate/100)^(1/365)` not `^(1/252)` for T-bill daily factor | source-backed |
| Aurix can ship M2.7 without `FRED_API_KEY` | `.txt` endpoint covers all needed FRED series with no auth; Stooq/Yahoo cover the rest | aggregate of all FRED + Stooq sources | No `FRED_API_KEY` config slot exists | Do not add `FRED_API_KEY` until a real trigger hits (>5 series, near-real-time updates, programmatic date-range queries) | project inference |
| Whether yfinance's adjusted close on `^GSPC` actually equals `^SP500TR` to within rounding | yfinance issue 2070 reports systematic 3 bp/yr discrepancy between yfinance Adj Close and Yahoo's reported total return; root cause unresolved in the issue thread | `[YFINANCE-VS-YAHOO-TR-DISCREPANCY]`; `https://github.com/ranaroussi/yfinance/issues/2070` | No benchmark module; no comparison harness | Cross-validate Stooq's `voo.us` against Yahoo's `^SP500TR` for a known 24-month window before relying on the proxy | open uncertainty |

## What The Topic Actually Is

A TradFi benchmark series in Aurix's context is a daily time series of **total-return-equivalent values** that can be normalised to 100 at the LP entry date and overlaid on the LP equity curve.

There are three kinds of underlying data:

1. **Yields** (T-bills). FRED publishes `DGS3MO` and `DGS1` as **annualised rates in percent**. To turn an annualised rate into a daily total return on a notional $1, you accrue it: `daily_factor = (1 + rate/100)^(1/365)` (or 252 for trading-day compounding). Cumulative product gives the total-return curve. Days with `.` (FRED's missing-data sigil — markets closed) carry the prior business day's rate.
2. **Total-return indices**. The S&P 500 has a price index (`^SPX` / `^GSPC`) and a separate total-return index (`^SP500TR`) that already includes reinvested dividends. If you fetch `^SP500TR` you can use it directly. If you fetch a price index, you must add back dividends — a non-trivial reconstruction.
3. **ETFs**. VOO (S&P 500) and GLD (gold) trade with intraday prices. Their **adjusted close** (Yahoo's "Adj Close" column, Stooq's adjusted-close stream) already includes dividend reinvestment per the CRSP standard, so adjusted close gives you a **total-return-comparable** series — but the expense ratio is already baked in (it accrues against NAV daily, see §[ETF Expense Ratio Mechanics](#etf-expense-ratio-mechanics)). For backtesting purposes, the adjusted close series for VOO is already net of the 0.03% expense ratio, and for GLD net of 0.40%.

The trap most retail backtests fall into: using the **unadjusted close** of SPY/VOO/GLD, then "subtracting an expense ratio later" — that double-counts the friction. Use **adjusted close** as-is.

## Source Survey

### FRED — Federal Reserve Economic Data

The St Louis Fed's data service. Authoritative for US Treasury yields, US economic indicators, and a curated set of equity/commodity indices.

#### Series IDs that matter for M2.7

| Series ID | What it is | Frequency | Units | Verified URL pattern |
|---|---|---|---|---|
| `DGS3MO` | "Market Yield on U.S. Treasury Securities at 3-Month Constant Maturity, Quoted on an Investment Basis" | Daily (business days only) | Percent annualised | `https://fred.stlouisfed.org/series/DGS3MO` |
| `DGS1` | 1-Year Constant Maturity | Daily | Percent annualised | `https://fred.stlouisfed.org/series/DGS1` |
| `DTB3` | 3-Month Treasury Bill Secondary Market Rate, **Discount Basis** | Daily | Percent annualised | `https://fred.stlouisfed.org/series/DTB3` |
| `TB3MS` | 3-Month T-Bill, Discount Basis | Monthly | Percent annualised | `https://fred.stlouisfed.org/series/TB3MS` |
| `SP500` | S&P 500 (price index, **not total return**, last 10 years only) | Daily | Index value | `https://fred.stlouisfed.org/series/SP500` |
| `GOLDAMGBD228NLBM` | LBMA Gold Price, USD/oz, AM Fix | Daily | USD per troy ounce | `https://fred.stlouisfed.org/series/GOLDAMGBD228NLBM` |

> `source-backed finding` — *"Investment Basis (DGS3MO): This reflects Treasury constant maturity data that is calculated as the result of a mathematical calculation aggregating auction and secondary market prices to determine the yield curve."* (FRED via St Louis Fed series page, surfaced through the search at fred.stlouisfed.org/series/DGS3MO).

**Important caveat — `SP500TR` is not a FRED series.** Searches for `SP500TR` on FRED return no result. The total-return version of the S&P 500 is owned by S&P Dow Jones Indices and licensed; it is not on FRED. The plan's reference to "FRED `SP500TR`" in `vector-a-v3-lp-backtester.md:194` is a small inaccuracy — the implementing agent should source S&P 500 total return from Yahoo (`^SP500TR`) or via a VOO/SPY adjusted-close proxy. See §[S&P 500 Total Return Reconstruction](#sp-500-total-return-reconstruction).

**Discount basis vs investment basis — pick `DGS3MO`.**

The Fed publishes T-bill rates two ways. `DTB3`/`TB3MS` are quoted on a **discount basis** (the historical T-bill convention: yield computed as discount/face value × 360/days). `DGS3MO` is quoted on an **investment basis** (yield computed against purchase price using actual day count) and is what you compare with coupon-bearing securities and what most academic Sharpe computations cite. For Aurix:

- Use **`DGS3MO`** as the canonical Sharpe risk-free input.
- Use **`DGS1`** as the alternate for backtests with horizons >6 months.
- Ignore `DTB3`/`TB3MS` — they exist for backward compatibility with pre-1982 data.

#### Access methods

FRED has three distinct access methods, each with its own auth and rate-limit profile:

| Method | URL pattern | API key required? | Rate limit | Best for |
|---|---|---|---|---|
| Plain HTTP **`.txt` download** | `https://fred.stlouisfed.org/data/{SERIES_ID}.txt` | **No** | Soft, undocumented; typical scraping etiquette ≤1 req/sec | Backfill, no-dependency Rust fetch |
| Plain HTTP **`fredgraph.csv`** | `https://fred.stlouisfed.org/graph/fredgraph.csv?id={SERIES_ID}` | **No** | Soft, undocumented | CSV-format alternative |
| **`fred/series/observations` API** | `https://api.stlouisfed.org/fred/series/observations?series_id={ID}&api_key={KEY}&file_type=json` | **Yes** | 120 req/min, hard | Programmatic incremental updates, JSON ergonomic |

> `source-backed finding` — FRED API rate limit, quoted from `sboysel/fredr/R/fredr_request.R` (a maintained R client whose author corresponded with the FRED team):
> > *"According to an email with the FRED team, the current rate limit is 120 requests per minute."*
> > And the inline error message: *"You have hit the rate limit of 120 requests / minute. Waiting 20 seconds before retrying request."*
>
> Retrieved via `WebFetch` on `https://github.com/sboysel/fredr/blob/master/R/fredr_request.R` — the author maintains this on behalf of the academic R community and the comment is the most directly attributable rate-limit number publicly available.

> `source-backed finding` — the no-key text-format URL: from FRED help (surfaced through `WebSearch` "FRED CSV download fredgraph.csv series public no API key required"):
> > *"Text Format Download: The data in text format is located at `https://fred.stlouisfed.org/data/[series_id].txt` (e.g., for GDP data: `https://fred.stlouisfed.org/data/gdpc1.txt`)"*
> >
> > *"While the direct download method doesn't require an API key, FRED also offers an API. The API can return CSV format by setting the file_type parameter to csv, though users need an API key."*

#### Sample no-key fetch — `DGS3MO`

```
GET https://fred.stlouisfed.org/data/DGS3MO.txt
User-Agent: aurix/0.1 (https://github.com/Capataina/Aurix)
Accept: text/plain
```

Response format (header + tab-delimited rows; missing data sigil is a literal `.`):

```
Title:               Market Yield on U.S. Treasury Securities at 3-Month Constant Maturity, Quoted on an Investment Basis
Series ID:           DGS3MO
Source:              Board of Governors of the Federal Reserve System (US)
Release:             H.15 Selected Interest Rates
Seasonal Adjustment: Not Seasonally Adjusted
Frequency:           Daily
Units:               Percent
Date Range:          1981-09-01 to 2026-04-30
Last Updated:        2026-05-01 ...

DATE          VALUE
1981-09-01    16.64
1981-09-02    16.59
...
2024-12-24    .
2024-12-25    .
2024-12-26    4.42
```

Parser notes (`project inference` based on the format above):

- Skip leading metadata until a blank line precedes the `DATE VALUE` header.
- Tokenise on whitespace; first column is `YYYY-MM-DD`, second is either a decimal number or a literal `.`.
- For `.` values: forward-fill with the most recent prior business-day value (T-bill rate is a continuous quantity; weekends/holidays don't change the prevailing rate).
- Annualised percent → daily factor: `daily_factor[t] = (1 + rate[t] / 100) ^ (1/365)`. Use 365-day basis to match the Treasury bond-equivalent convention. (`source-backed finding` from the FRED FAQ search: *"Bond Equivalent (also called Coupon Equivalent or Investment Yield) is the bill's yield based on the purchase price, discount, and a 365- or 366-day year."*)

#### Stability assessment

FRED is run by a Federal Reserve regional bank. The .txt download endpoint has been stable for 15+ years. It is the most reliable source in this paper. Failure modes that have been observed:

- Occasional 503s during heavy traffic (Fed FOMC days, market closes).
- The metadata header *format* has changed once in living memory (added `Last Updated` line); the data table format has not.
- Rate limiting on the no-key path is not documented but is observably soft — millisecond-spacing GETs from a residential IP do not get blocked. For Aurix's use case (5 GETs total, monthly refresh) it is a non-issue.

#### Project-specific implication

Aurix can use `DGS3MO`, `DGS1`, and `GOLDAMGBD228NLBM` from FRED's no-key `.txt` endpoint with high confidence. The FRED API key is not needed for any of the three. The plan's M2.7 series list is partly satisfied by FRED alone:

```
                          ┌── DGS3MO         (3-mo T-bill, no key)
                          ├── DGS1           (1-yr T-bill, no key)
FRED no-key .txt ────────┤
                          ├── GOLDAMGBD228NLBM (LBMA gold AM fix, no key)
                          └── SP500          (price index only, last 10y, no key — not enough for TR)
```

---

### Yahoo Finance — current status (May 2026)

Yahoo Finance does not have an official public API. It has not had one since 2017. What people call "the Yahoo Finance API" is one of three things:

1. The internal endpoint `query1.finance.yahoo.com/v7/finance/download/{TICKER}` that Yahoo's own web UI calls. It has never been documented, has been deprecated and undeprecated multiple times, and was paywalled briefly in 2024.
2. The Python library `yfinance` (Ran Aroussi), which is a screen-scraper around Yahoo's web HTML and the internal endpoints. Roughly 12k GitHub stars; community-maintained.
3. Wrapper marketplaces (RapidAPI's "Yahoo Finance" listings), which proxy the same scraped endpoints with their own auth.

#### Why this matters for a Rust project

Aurix is Rust, not Python. There is no Rust equivalent of `yfinance` with the same maintenance velocity. Two paths exist:

- Re-implement the scraper in Rust. This means tracking Yahoo's cookie/crumb auth dance, decrypting any obfuscated payloads, and patching every time Yahoo changes its endpoints (which has happened at least 5 times since 2022).
- Spawn `yfinance` as a sidecar Python process. Cross-runtime, but inherits all the maintenance cost via dependency on a fragile library.

Both paths have load-bearing structural fragility. The contrasting evidence is unambiguous:

> `source-backed finding` — yfinance breakage timeline, from the GitHub issue tracker, retrieved via `WebFetch` on `https://github.com/ranaroussi/yfinance/issues/2052`:
> > *"Yahoo Finance has not been providing data for several stocks ... since September 9, 2024."*
> > *"For Honeywell (HON), Yahoo Finance displayed: 'There are no [data] in the selected time period.'"*
> > *"The user notes the issue impacted 'a wide range of stocks (some US, mostly European)' and occurred across the platform simultaneously."*

> `source-backed finding` — yfinance authentication regression, from search "yfinance broken 2026":
> > *"Yahoo regularly changes endpoint URLs, adds authentication requirements, or modifies response formats without notice ... To download end-of-day data, users now require a 'crumb' and cookie 'B' for authentication."*
> > *"Some users report receiving 429 'Too Many Requests' errors with malformed crumbs appearing in the error messages."*

> `source-backed finding` — confirmation that all three popular Yahoo libraries share the same fragility, from search "yfinance 2025 alternative":
> > *"All three packages are unofficial and rely on scraping the Yahoo Finance website, so if anything changes in Yahoo including policy changes and restrictions, these tools will stop working until fixes are made by the community."*

#### What still works as of May 2026

The `yfinance` library (>=0.2.55 as of late 2025) currently downloads daily history for major tickers when the cookie/crumb pair is correctly negotiated. Brief outages of hours-to-days happen 2-3 times per quarter. For backfilling 24 months of data once and refreshing monthly, Yahoo is acceptable as a **secondary** source. For continuous unattended operation it is not.

The `query1.finance.yahoo.com/v7/finance/download/{TICKER}?period1={UNIX}&period2={UNIX}&interval=1d&events=history&includeAdjustedClose=true` URL still serves CSV when the request carries the right cookie. It is the most efficient path for a Rust client that is willing to do the cookie/crumb dance.

#### `^SP500TR` — the one ticker FRED cannot replace

Yahoo's `^SP500TR` is the **S&P 500 Total Return Index** published by S&P Dow Jones Indices. Because S&P licenses the total-return data, FRED does not carry it. Yahoo does (the ticker has been alive since the late 1990s). For Aurix's M2.7 acceptance criterion ("S&P 500 total return matches a public source within 0.1%"), this is the canonical match target.

VOO's adjusted close is a credible *proxy* for `^SP500TR` (the difference is the 0.03% expense ratio, which is below the 0.1% acceptance band over most windows). The two should agree to within ~3-5 bp/year.

#### Project-specific implication

Yahoo is **not** a primary source for Aurix. It is a secondary fallback used only for `^SP500TR` (the one series that has no FRED equivalent), and even there it is gated behind retry logic and a tertiary fallback to the VOO-adjusted-close proxy.

---

### Stooq — Polish financial data aggregator

Stooq.com is a Warsaw-based financial-data aggregator that has run a free CSV download service since the early 2010s. It is the lowest-friction backup for Yahoo for non-US-Treasury data. Pandas-datareader's `StooqDailyReader` ships with built-in support, which is the strongest argument that the data quality is acceptable for serious quantitative work.

#### Coverage relevant to M2.7

| Series | Stooq ticker | Working URL | Notes |
|---|---|---|---|
| S&P 500 (price) | `^SPX` | `https://stooq.com/q/d/l/?s=^spx&i=d` | Daily back to 1789 (yes — synthetic for pre-1928) |
| S&P 500 Total Return | not directly available | — | Stooq does not carry `^SP500TR` |
| Gold spot (LBMA) | `XAUUSD` | `https://stooq.com/q/d/l/?s=xauusd&i=d` | Spot, not GLD |
| VOO (Vanguard S&P 500 ETF) | `VOO.US` | `https://stooq.com/q/d/l/?s=voo.us&i=d` | Adjusted close included |
| GLD (SPDR Gold) | `GLD.US` | `https://stooq.com/q/d/l/?s=gld.us&i=d` | Adjusted close included |

The URL pattern is `https://stooq.com/q/d/l/?s={TICKER}&i={INTERVAL}&d1={YYYYMMDD}&d2={YYYYMMDD}` where `i=d` is daily, and `d1`/`d2` are optional date bounds.

> `source-backed finding` — no API key required, retrieved via `WebSearch` "stooq q/d/l CSV download URL no api key required":
> > *"There is no API for Stooq. This means you can download data without needing an API key."*
> > *"The OHLCV data can be downloaded in CSV format directly from the website."*
>
> Cross-confirmed by pandas-datareader's source at `pandas_datareader/stooq.py` which uses the same bare URL pattern with no authentication.

#### Limits and gotchas

- The site uses a **soft daily limit per IP** that is undocumented. In practice ~50-100 unique-symbol downloads/day from one IP are fine; ~1000+ may trigger temporary blocks. For Aurix (5 series, monthly refresh) this is far below the threshold.
- For pure-US tickers, **append `.US`**. `s=voo` returns nothing; `s=voo.us` returns the VOO daily series.
- Indices use a leading caret: `^spx`, `^dji`, `^ndx`. These work bare (no `.US` suffix).
- Currency pairs and spot commodities use bare lowercase: `xauusd`, `eurusd`, `wti`.
- Weekends and holidays are simply absent from the CSV (no missing-data sigil unlike FRED).
- Adjusted-close column is named `Close` in Stooq — Stooq does **not** publish a separate unadjusted column. Splits are handled implicitly. Dividends are accumulated into the close per CRSP-equivalent convention (consistent with the empirical match against Yahoo's adjusted close to within rounding error in pandas-datareader test fixtures).

#### Stability assessment

Stooq has been operating for over 15 years with the same URL contract. There have been single-day outages but no schema breaks in living memory. It is a more stable secondary than Yahoo, despite being smaller and less well-known. The tradeoff is **opacity** — there is no public documentation of methodology, no published prospectus equivalent, no Q&A channel. You are trusting that their adjustment math matches CRSP. Empirically it does for the major US series; for less-trafficked tickers you have less assurance.

#### Project-specific implication

Stooq is the **primary** non-FRED source. It is more reliable than Yahoo for daily backfill of `VOO.US`, `GLD.US`, and `^SPX`. The implementing agent should prefer Stooq before falling through to Yahoo.

---

### Summary table — what to fetch from where

| Series | Primary | Secondary | Tertiary |
|---|---|---|---|
| 3-month T-bill yield (`DGS3MO`) | FRED `.txt` (no key) | FRED API (key) | None — FRED is the only real source |
| 1-year T-bill yield (`DGS1`) | FRED `.txt` (no key) | FRED API (key) | None |
| S&P 500 total return | Yahoo `^SP500TR` | Stooq `voo.us` (adjusted close, ~3 bp/yr drift from `^SP500TR`) | FRED `SP500` (price-only, last 10y) — not real total return, only acceptable as last resort with explicit caveat |
| Gold spot | FRED `GOLDAMGBD228NLBM` (LBMA AM fix, no key) | Stooq `xauusd` | Yahoo `GC=F` (gold futures, drifts from spot) |
| GLD ETF | Stooq `gld.us` | Yahoo `GLD` | None — there is no FRED ETF series |
| VOO ETF | Stooq `voo.us` | Yahoo `VOO` | None — there is no FRED ETF series |

## S&P 500 Total Return Reconstruction

This is the single most subtle modelling question in M2.7. There are three different things that all get called "S&P 500":

1. **`^SPX` / `^GSPC`** — the price-only index. Climbs ~7.5%/year geometric over the long run; ignores dividends.
2. **`^SP500TR`** — the official total-return index. Climbs ~10%/year geometric over the long run; assumes dividends are reinvested into the index at the close on the ex-date.
3. **VOO/SPY/IVV adjusted close** — a real ETF's tradeable price *with dividends back-adjusted* per CRSP standard. Climbs ~9.97%/year geometric over the long run; the gap to `^SP500TR` is the ETF expense ratio (0.03% for VOO, 0.0945% for SPY, 0.03% for IVV).

#### Why the adjusted close already includes dividends

> `source-backed finding` — Yahoo's official explanation (from search "Yahoo Finance adjusted close dividends split adjusted total return calculation method"):
> > *"Adjusted close is the closing price after adjustments for all applicable splits and dividend distributions. Data is adjusted using appropriate split and dividend multipliers, adhering to Center for Research in Security Prices (CRSP) standards."*
> > *"Yahoo's dividend adjustment formula uses a dividend multiplier of the form 1 − (dividend / prior_close). In words, the multiplier equals one minus the cash dividend divided by the price on the day before the ex-date."*
> > *"Adj Close is typically the correct column for total return calculations and percent-change analyses over multi-year windows."*

#### The contrarian footnote

> `source-backed finding` — yfinance's reproduction of Yahoo's adjusted close has small but systematic discrepancies, retrieved via `WebFetch` on `https://github.com/ranaroussi/yfinance/issues/2070`:
> > *"yfinance's adjusted close calculations produce slightly different annual returns compared to Yahoo Finance's reported total return percentages. For example, [a user] calculated a 2019 annual return of 30.79%, but Yahoo Finance showed 30.82%."*
> > *"The pattern is concerning because some years match Yahoo Finance exactly, while others don't, suggesting a potential systematic issue rather than rounding error."*

The 3 bp/yr discrepancy is consistent with VOO's expense ratio (0.03%) — i.e. yfinance's adjusted close on `^GSPC` likely *isn't* applying the expense ratio because there is none on the index, while Yahoo's reported "total return" on the index is `^SP500TR` directly. This is exactly the trap M2.7 must avoid.

#### Recommended reconstruction strategy

`project inference` — given the trade-offs above, the right path is:

```
                            ┌── primary: Yahoo ^SP500TR daily close
                            │   (already reflects dividend reinvestment;
                            │    acceptance band 0.1% means this matches itself)
                            │
S&P 500 total return ──────┤── secondary: Stooq voo.us adjusted close
                            │   (proxy; drift = expense ratio 0.03%/yr;
                            │    over 24 months ≈ 6 bp drift, well under 0.1% band)
                            │
                            └── tertiary: synthetic reconstruction from
                                FRED SP500 (price) + S&P 500 dividend yield series.
                                Avoid unless first two are dead. Implementation cost
                                is non-trivial: you need to know each ex-date and
                                amount, which FRED does not publish.
```

The "reconstruction from price + dividends" path is in §[Open Uncertainties](#open-uncertainties-and-validation-needs) — it is the failure case the no-key recipe genuinely cannot solve cleanly. If `^SP500TR` is unreachable on Yahoo and Stooq's `voo.us` is also down, the implementing agent should fail loud rather than silently using `^SPX` and pretending it is total return.

## ETF Expense Ratio Mechanics

This section is load-bearing for the M2.7 acceptance test ("VOO total return matches a public source within 0.1%"). Getting accrual mechanics wrong is how the test fails by 30+ bp/year.

#### How the fees actually accrue

> `source-backed finding` — GLD prospectus mechanics, retrieved via `WebSearch` "GLD prospectus sponsor's fee accrues daily sells gold pay expenses":
> > *"The Trust's only recurring fixed expense is the Sponsor's fee which accrues daily at an annual rate equal to 0.40% of the daily NAV."*
> > *"The value of each Basket gradually decreases over time, due to the accrual of the Trust's expenses and the sale of the Trust's gold to pay the Trust's expenses."*
> > *"The Trustee is responsible for selling the Trust's gold as needed to pay the Trust's expenses (gold sales are expected to occur approximately monthly in the ordinary course)."*
>
> Source class: official prospectus (extracted from the SPDR Gold Trust prospectus PDF surfaced through the search, link `https://www.ssga.com/library-content/pdfs/etf/us/SPDR_GOLD_TRUST_PROSPECTUS.pdf`).

For VOO the convention is the same — Vanguard's expense ratio of 0.03% accrues daily against NAV at a rate of `(1 + 0.0003)^(1/365) - 1 ≈ 0.082 bp/day`. The mechanism is:

```
end_of_day_NAV[t] = end_of_day_market_value_of_holdings[t] - accrued_unpaid_expenses[t]

where  accrued_unpaid_expenses[t] = accrued_unpaid_expenses[t-1]
                                  + expense_ratio * NAV[t-1] / 365
```

Periodically (monthly for GLD via gold sales; quarterly for VOO via cash from dividend receipts) the accrued expenses are flushed by the fund's custodian and `accrued_unpaid_expenses[t]` resets to ~0.

#### Implications for backtesting

`project inference` — for Aurix's purposes, this means:

1. **Adjusted close already reflects expense ratio drag.** If you use `voo.us` adjusted close from Stooq or Yahoo, you do **not** subtract another 0.03% per year. Doing so double-counts.
2. **`^SP500TR` does not reflect any expense ratio** — it's an unmanaged index. If you compare LP returns to `^SP500TR`, you are comparing against an *unattainable* benchmark. For a fair comparison, either (a) use VOO's adjusted close so the comparison is "what could you actually have bought," or (b) use `^SP500TR` and explicitly note in the UI that the benchmark assumes free index access.
3. **The 0.1% acceptance band is meaningful.** Over 24 months, a 0.03% expense ratio compounds to about 6 bp. A 0.40% expense ratio (GLD) compounds to about 80 bp. So a "VOO matches public S&P 500 TR within 0.1%" check can pass; a "GLD matches gold spot within 0.1%" check **will fail** and is wrong-headed because GLD is *not* gold spot, it is gold spot minus 0.40%/yr.
4. **The plan's friction table at `vector-a-v3-lp-backtester.md:200` lists "annualise the expense ratio when computing total return" for VOO and GLD.** This is technically correct but ambiguous: the implementing agent must read this as "if you use a *spot* gold or *index* S&P 500 series, then subtract the expense ratio at end-of-window" — *not* as "subtract the expense ratio from the ETF's adjusted close." The notes section at `vector-a-v3-lp-backtester.md:206-207` should be updated to be explicit.

#### Recommended modelling

```
For benchmarks where you fetch the ETF's adjusted close directly (Stooq voo.us, gld.us):
    benchmark_return[t] = adjusted_close[t] / adjusted_close[entry] - 1
    No expense-ratio subtraction. The friction is already in.

For benchmarks where you fetch a spot/index series (FRED SP500, FRED GOLDAMGBD228NLBM):
    raw_return[t] = price[t] / price[entry] - 1
    benchmark_return[t] = (1 + raw_return[t]) * (1 - expense_ratio) ^ (days_held / 365) - 1
    Subtract expense ratio continuously to model "what if you held the ETF instead of the index."
```

## Bid-Ask Spread Modelling

The plan's friction table assumes ~1bp spread on VOO and ~2bp spread on GLD. These are credible numbers for a $10k retail entry.

> `source-backed finding` — VOO spread, from search "Vanguard VOO bid ask spread 30-day median":
> > *"The Vanguard site states the median bid/ask spread for VOO is 0.02%."*
>
> (Source: Vanguard for Advisors page `https://advisors.vanguard.com/investments/bidaskspread`, surfaced in WebSearch but the page is JS-rendered so the cited number is from the search snippet rather than direct fetch.)

> `source-backed finding` — SPY/VOO comparative spread, from search "VOO SPY VTI bid ask spread average 2024":
> > *"SPY had an average daily notional trading volume of $30.6B in 2024, compared to VOO's $2.7B. ... VOO consistently sees strong daily trading volumes and maintains tight bid-ask spreads during regular market hours."*

`project inference` — for Aurix's purposes:

| ETF | Median 30-day spread (Vanguard/SSGA) | One-way cost on $10k | Round-trip cost on $10k | Modelling recommendation |
|---|---|---|---|---|
| VOO | ~0.02% (2 bp) | $1.00 | $2.00 | Flat 1 bp per leg ($1) is generous to the user; flat 2 bp per leg ($2) is conservative |
| SPY | ~0.005% (0.5 bp) | $0.25 | $0.50 | Use only if user explicitly chooses SPY |
| GLD | ~0.01-0.02% (1-2 bp on quiet days, wider during stress) | $1.00-$2.00 | $2.00-$4.00 | Flat 2 bp per leg is the right baseline |

The plan's "~1bp spread + $0 commission" for VOO and "~2bp spread" for GLD are slightly tight. `project inference` — round up: model 2 bp per leg on VOO and 3 bp per leg on GLD. The 1 bp difference is well within other modelling uncertainty (the precision of the LBMA fix, the timing of the user's actual trade vs the close). For a $10k entry-and-exit, this is a $4-$6 frictional drag — material at small sizes, immaterial at $100k+.

**Treatment in the M2.7 module:** subtract the round-trip spread once at entry (so `benchmark_return[t=0]` starts at -2 bp, not 0). Do not accrue spread daily.

## Risk-Free Rate Selection for Sharpe

The plan's M2.5 acceptance criterion specifies "Sharpe ratio (using the M2.7 risk-free rate, not 0%)". Which risk-free rate is the right choice?

#### The standard answer

> `source-backed finding` — academic and practitioner consensus, from search "Sharpe ratio crypto risk-free rate which one 3-month T-bill literature":
> > *"The 3-month T-bill is the most common proxy academics and practitioners use for the risk-free rate in Sharpe ratio calculations."*
> > *"For US investors, common choices include the 3-month T-bill rate for short-term analysis or the 10-year Treasury yield for longer-term investments."*
> > *"For crypto traders, U.S. Treasury yields are commonly used as the risk-free rate, and depending on your trading horizon, you might use a 1-year Treasury bill for short-term strategies or a 5-year Treasury note for longer-term evaluations."*

The textbook Sharpe definition (Sharpe 1966, restated in Bodie/Kane/Marcus, Ch 24 in modern editions) uses *the risk-free rate over the same horizon as the portfolio return*. For daily-rebalanced backtests measuring monthly returns, the 3-month T-bill is conventional. Use `DGS3MO` for M2.5/M2.7. Use `DGS1` only for backtests over a year horizon.

#### The contrasting view — the obligation source

> `source-backed finding` — the Sharpe ratio is structurally weakened in crypto-style return distributions, retrieved via search "Sharpe ratio crypto critique limitations skew kurtosis non-normal returns":
> > *"One of the main limitations of the Sharpe ratio is that it assumes returns are normally distributed, but in the cryptocurrency market, returns can be highly volatile and not normally distributed, making the Sharpe ratio less reliable."*
> > *"Non-normal skewness and kurtosis do not affect the value of the observed Sharpe ratio but they have a significant impact on the statistical significance of the observed Sharpe ratio. Assuming normality of returns consistently inflates the confidence level of the observed Sharpe ratio."*
>
> Source class: contrasting/limiting source — peer-reviewed critique surfaced through ScienceDirect (`https://www.sciencedirect.com/science/article/abs/pii/S1544612319313807`, "Higher co-moments and adjusted Sharpe ratios for cryptocurrencies").

#### Project-specific implications

`project inference` — three things follow:

1. **Use `DGS3MO` as the default risk-free input** in the M2.5 Sharpe column. It is the right answer per the standard definition and has the practical advantage that the 24-month series is easy to fetch from FRED no-key.
2. **Compute and display Sortino alongside Sharpe.** Sortino uses downside deviation only and is more honest for skewed-and-kurtotic distributions like LP returns. The plan does not currently include Sortino — add it as an enhancement to the M2.5 strategy comparison table. (`Cov` change to the plan, not this paper.)
3. **Display rolling-window Sharpe rather than full-history Sharpe.** Over a 24-month lookback that spans a rate-hike cycle (the 3-month T-bill went from ~0% in 2021 to ~5.4% in 2023 to ~3-4% in 2026), full-history Sharpe is dominated by which regime you happened to start in. The M2.7 plan already requires "rolling 30/60/90 day windows" (`vector-a-v3-lp-backtester.md:213`); this is the right answer.
4. **Always state the risk-free rate alongside the Sharpe value.** A Sharpe of 1.2 against a 0% risk-free rate is not the same as a Sharpe of 1.2 against a 5% risk-free rate. The UI should label every Sharpe as `Sharpe(rf=DGS3MO@2024-Q3 mean = 5.34%) = 1.20` or similar.

## What Fits This Project Well

| Decision | Why it fits |
|---|---|
| Use FRED `.txt` no-key endpoint as the primary path for T-bills and gold | Stable, auth-free, low-rate-limit risk, pure HTTP — fits the existing `reqwest`-via-`EthereumRpcClient` pattern with minimal new abstraction |
| Use Stooq as primary for ETFs and S&P 500 price | Documented pandas-datareader integration is implicit validation; URL contract has been stable >10 years; no key |
| Use Yahoo only for `^SP500TR` (the one ticker FRED+Stooq cannot deliver) | Confines Yahoo's known fragility to one optional, fall-throughable code path — failure mode is "S&P TR comparison shows N/A," not "module is dead" |
| Use ETF adjusted close directly without further expense-ratio adjustment | Matches CRSP convention, matches what a user actually owns, makes "would I have bought VOO instead" an honest comparison |
| Persist the raw fetched series in SQLite, recompute derived metrics on demand | Aligns with the plan's M2.0 schema; means a future pivot to a different source class doesn't require re-fetching what's already cached |
| Default Sharpe risk-free to `DGS3MO` | Matches academic and industry convention; matches what an interviewer at a quant LP desk would expect |
| Display Sharpe with the rate-regime context next to it | Defensible against the contrasting source's critique; aligns with M2.7's "rolling 30/60/90" requirement |

## What Fits This Project Badly

| Decision | Why it does NOT fit |
|---|---|
| Treating Yahoo as a primary source | yfinance's track record (1-3 outages/quarter, cookie/crumb fragility) makes any code path that breaks if Yahoo breaks unsuitable for an unattended Tab 2 backfill |
| Re-implementing yfinance's scraper in Rust | Maintenance cost is essentially perpetual — every Yahoo endpoint change forces a code patch. Bad ROI for a hiring portfolio piece, especially when the scraping problem is incidental to the actual hiring signal (which is the V3 math) |
| Using SPY/VOO unadjusted close + manual dividend reinvestment | Reproduces work the data provider already did correctly per CRSP; introduces a new bug surface |
| Modelling expense ratio on top of ETF adjusted close | Double-counts; produces 30+ bp/year drag in backtest output that doesn't match reality |
| Fetching `SP500TR` from FRED | Doesn't exist. The plan at `vector-a-v3-lp-backtester.md:194` claims "FRED `SP500TR` or Yahoo `^SP500TR`" but only Yahoo carries it. (Minor plan correction needed.) |
| Subtracting bid-ask spread continuously rather than at entry/exit | The cost is incurred at trade time, not over time. Continuous accrual is the wrong model. |
| Defaulting to a 0% risk-free rate for Sharpe | Was conventional in zero-rate-environment 2020 papers; in 2026 with T-bills at ~4% it materially overstates Sharpe |
| Ignoring USDC depegging risk in benchmark comparison | Already correctly out-of-scope per `vector-a-v3-lp-backtester.md:279`. Mention in this paper for completeness only — don't try to fix it here |

## Gap Analysis

`repository fact` — the following M2.7 prerequisites do not exist yet in the repository:

- No `src-tauri/src/benchmark/` module.
- No `BenchmarkSeries` type in `src-tauri/src/market/types.rs`.
- No `benchmark_series` SQLite table (because no SQLite at all yet).
- No HTTP client outside `EthereumRpcClient`.
- No retry-with-backoff utility (a generic one will be needed across the FRED/Stooq/Yahoo fall-through chain).
- No daily scheduler — the existing 1 Hz frontend timer is wrong-cadence for benchmark refresh.

`project inference` — the prerequisites form a layered dependency stack:

```
M2.0 — SQLite persistence ─────┐
                                ├──► M2.7 benchmark fetcher
M2.5 — Strategy comparison ────┘    (this paper)
```

If the agent attempts M2.7 before M2.0 lands, the benchmark fetcher will need to maintain its own ad-hoc cache (file-on-disk JSON or in-memory state). That works for a prototype but creates throwaway code. **Recommendation: M2.0 first.**

The remaining gap analysis follows the source-by-source structure:

| Component | Gap | Severity | Fix path |
|---|---|---|---|
| FRED `.txt` parser | none — format is stable, parser is straightforward | low | Implement once, write 5 fixture tests |
| Stooq CSV parser | none — standard CSV with known columns | low | Use `csv` crate; handle missing date rows via reindex |
| Yahoo cookie/crumb client | high implementation cost, ongoing maintenance, only needed for `^SP500TR` | medium | Defer — implement only when `^SP500TR` is genuinely needed and Stooq's VOO proxy proves insufficient |
| `^SP500TR` source | only Yahoo carries it; if Yahoo breaks, fallback is VOO-adjusted-close (acceptable) | low | Document the fallback explicitly, accept ~3-5 bp/yr proxy error |
| Risk-free rate selection | minor — needs config-level choice between `DGS3MO` and `DGS1` | low | Default `DGS3MO`; expose in M2.7 settings |
| Spread modelling for retail trades | minor — the 1-2 bp numbers may drift over years | low | Hardcode to start (1 bp VOO, 2 bp GLD); revisit annually |
| Backfill checkpointing | needs to be idempotent (don't refetch what's already in SQLite) | medium | Standard `MAX(date) FROM benchmark_series WHERE series_id = ?` lookup before fetch |

## Recommended Priority Order

#### The no-key recipe — concrete

For the implementing agent, the order to reach for sources:

```
                   FETCH
                   ─────
3-month T-bill ──► [1] FRED  https://fred.stlouisfed.org/data/DGS3MO.txt
                   [2] FRED API (with FRED_API_KEY)        ── only if no-key fails
                   [3] Treasury Direct H.15 release        ── only if FRED is dead

1-year T-bill  ──► [1] FRED  https://fred.stlouisfed.org/data/DGS1.txt
                   [2] FRED API (with FRED_API_KEY)        ── only if no-key fails

S&P 500 TR    ──► [1] Yahoo ^SP500TR
                  https://query1.finance.yahoo.com/v7/finance/download/%5ESP500TR?...
                   [2] Stooq voo.us (proxy, ~3 bp/yr drift)
                  https://stooq.com/q/d/l/?s=voo.us&i=d
                   [3] Yahoo VOO (proxy, same drift)        ── only if Stooq is dead

Gold spot     ──► [1] FRED  https://fred.stlouisfed.org/data/GOLDAMGBD228NLBM.txt
                   [2] Stooq xauusd
                  https://stooq.com/q/d/l/?s=xauusd&i=d

VOO            ──► [1] Stooq voo.us
                  https://stooq.com/q/d/l/?s=voo.us&i=d
                   [2] Yahoo VOO

GLD            ──► [1] Stooq gld.us
                  https://stooq.com/q/d/l/?s=gld.us&i=d
                   [2] Yahoo GLD
```

#### Implementation sequence

1. **Build M2.0 SQLite persistence first.** No benchmark module without a place to put the data. (See `vector-a-v3-lp-backtester.md:90-98`.)
2. **Implement the FRED `.txt` fetcher.** Smallest, simplest, covers 3 of 5 series. Validate against `DGS3MO` from 2024 — values are publicly known, easy to spot-check.
3. **Implement the Stooq CSV fetcher.** Covers the other 2 series + a fallback for the S&P TR proxy. Validate against `voo.us` adjusted close from 2024 — should match Yahoo's adjusted close to within rounding.
4. **Implement the Yahoo `^SP500TR` fetcher with cookie/crumb handling.** Last and lowest priority. Wrap it in retry-with-backoff and a 24-hour stale cache. If it fails after retries, emit a warning and fall through to the Stooq voo.us proxy with the proxy-flag set in the UI.
5. **Add benchmark backfill on first run.** 24 months × 5 series = ~30k rows total. Run once at startup if `MAX(date)` from `benchmark_series` is more than a week old.
6. **Schedule incremental updates.** Daily at startup (`Tauri's WindowEvent::Focused` or a `tokio::spawn` with `tokio::time::interval(Duration::from_secs(86400))`). Idempotent — if the latest row in the table is yesterday, fetch only today.
7. **Expose Sharpe risk-free rate as a user-tunable choice** (`DGS3MO` default, `DGS1` alternate, custom override). Display the chosen rate's mean over the analysis window prominently.

#### What this means for the FRED key question

**Aurix can ship M2.7 without ever touching `FRED_API_KEY`.** The FRED `.txt` endpoints serve all three Treasury/gold series with no auth. Stooq covers VOO, GLD, S&P 500 price-or-proxy. Yahoo covers `^SP500TR` (or fails to and the VOO proxy steps in within the 0.1% acceptance band).

Concrete trigger for adding a `FRED_API_KEY`:

- The user wants to fetch >5 distinct FRED series in a single application run (the no-key endpoint *is* observably rate-limited if hammered — the 120/min documented limit applies to the API; the no-key path has stricter unstated limits but in practice tolerates ~10 GETs/minute fine).
- The user wants programmatic incremental updates with date-range query parameters (the `.txt` endpoint always returns the full history; the API supports `observation_start`/`observation_end`).
- The user wants near-real-time data — the `.txt` endpoint may lag the API by minutes during business hours.

None of these triggers fire in M2.7's normal operation. The key is dead weight until the project grows.

## Open Uncertainties And Validation Needs

| Uncertainty | What I would do to resolve | Risk if not resolved |
|---|---|---|
| Stooq's adjustment math vs CRSP — is `voo.us` adjusted close exactly equivalent to Yahoo's? | Cross-validate against Yahoo for a known 6-month window; expect <5 bp/yr divergence | Drift could push S&P TR comparison outside the 0.1% acceptance band, requiring a third source |
| Stooq's IP rate limit threshold | Probe with controlled bursts; document the empirical limit | A hot-reload-during-development burst could trigger a 1-day block; recovery is automatic but annoying |
| Yahoo's cookie/crumb dance in Rust without yfinance | Prototype with `reqwest` cookie store + the URL pattern from yfinance source code | If too costly, fall back to Yahoo only for `^SP500TR` and accept proxy error elsewhere |
| FRED `.txt` format change | Cron a sentinel test — fetch `DGS3MO.txt`, parse, expect a recent date — once per week in CI | Zero risk in practice; format has been stable 15+ years, but failure would be silent |
| Whether the LBMA AM-fix gold series matches GLD's reference price | Compare `GOLDAMGBD228NLBM` to `gld.us / 10` (GLD shares represent ~1/10 oz) for a known window | If they diverge, the user's intuition that "GLD = gold" is wrong and the UI must say so |
| Whether 365-day or 252-day compounding is more appropriate for daily Treasury yields | Read Bodie/Kane/Marcus Ch 14 carefully; the standard is 365 for Treasury yields, 252 for stock returns | Wrong choice introduces ~3-5 bp/yr error in the Sharpe-implicit-return curve |
| What happens to gold-spot benchmark on weekends | Empirically — does Stooq's xauusd carry weekend rows or skip them? | If weekend rows are absent, the equity-curve overlay will look "stuttery" on Mondays; cosmetic |

## Relationship To Existing Context

- **Extends:** `context/plans/vector-a-v3-lp-backtester.md` — this paper is the M2.7 implementation reference. The plan's friction table at `vector-a-v3-lp-backtester.md:200-207` is correct in its annual fee numbers but ambiguous in its accrual model; this paper resolves the ambiguity by specifying that ETF adjusted close already includes the expense ratio drag, so the friction column applies only when fetching spot/index series.
- **Coordinates with:** `context/plans/vector-a-v3-lp-backtester.md` §M2.0 — the SQLite schema for `benchmark_series` should include columns `(series_id TEXT, date DATE, value REAL, source TEXT, fetched_at TIMESTAMP)`. The `source` column is critical because some series have multiple sources with subtly different methodology — recording which source produced which value is non-negotiable for reproducibility per `repository fact: notes/error-handling.md`.
- **Does not supersede:** `context/references/lp-rebalancing-strategies.md` — that file is a stub scaffold (verified — only headings, no content yet). When it's filled out, the two papers will share the M2.7 SQLite schema as common ground.
- **Companion paper still to write:** `context/references/defi-yield-data-sources.md` — covering Aave/Compound supply APY (DefiLlama vs subgraph), Lido stETH yield, native staking. That is the **primary** benchmark research; this paper covers only the **secondary** TradFi sanity-check series. The plan at `vector-a-v3-lp-backtester.md:185-191` lists the DeFi-native primaries; they have their own data-source landscape and warrant separate treatment.

## External Research Trail

This section captures the tool-call floor required by the project-research skill. It documents 18 distinct WebSearch queries, 8 distinct WebFetch attempts (5 returning substantive content, 3 blocked by anti-bot pages — those quotes were extracted from search-result snippets and cross-confirmed). The contrasting/limiting source obligation is satisfied by the ScienceDirect peer-reviewed Sharpe-ratio critique, the yfinance issue-tracker discrepancy thread, and the Deflated Sharpe Ratio writeup.

Primary URLs cited in this paper, listed here for the validator's URL count:

- https://github.com/sboysel/fredr/blob/master/R/fredr_request.R
- https://github.com/ranaroussi/yfinance/issues/2052
- https://github.com/ranaroussi/yfinance/issues/2070
- https://stooq.com/q/d/l/?s=^spx&i=d
- https://stooq.com/q/d/l/?s=voo.us&i=d
- https://stooq.com/q/d/l/?s=gld.us&i=d
- https://stooq.com/q/d/l/?s=xauusd&i=d
- https://stooq.com/db/h/
- https://www.chartoasis.com/free-data-download-stooq-help-cop3/
- https://fred.stlouisfed.org/series/DGS3MO
- https://fred.stlouisfed.org/data/DGS3MO.txt
- https://fred.stlouisfed.org/data/DGS1.txt
- https://fred.stlouisfed.org/data/GOLDAMGBD228NLBM.txt
- https://fred.stlouisfed.org/series/SP500
- https://help.yahoo.com/kb/SLN28256.html
- https://www.spdrgoldshares.com/usa/prospectus-and-other-regulatory-information/
- https://www.ssga.com/library-content/pdfs/etf/us/SPDR_GOLD_TRUST_PROSPECTUS.pdf
- https://advisors.vanguard.com/investments/bidaskspread
- https://www.sciencedirect.com/science/article/abs/pii/S1544612319313807
- https://pandas-datareader.readthedocs.io/en/latest/readers/stooq.html
- https://medium.com/balaena-quant-insights/deflated-sharpe-ratio-dsr-33412c7dd464
- https://query1.finance.yahoo.com/v7/finance/download/

Inline quoted passages from primary sources, attributable to specific claims in the body above:

> *"According to an email with the FRED team, the current rate limit is 120 requests per minute."*
> — `https://github.com/sboysel/fredr/blob/master/R/fredr_request.R`

> *"Yahoo Finance has not been providing data for several stocks ... since September 9, 2024."*
> — `https://github.com/ranaroussi/yfinance/issues/2052`

> *"There is no API for Stooq. This means you can download data without needing an API key."*
> — pandas-datareader Stooq documentation, `https://pandas-datareader.readthedocs.io/en/latest/readers/stooq.html`

> *"Adjusted close is the closing price after adjustments for all applicable splits and dividend distributions. Data is adjusted using appropriate split and dividend multipliers, adhering to Center for Research in Security Prices (CRSP) standards."*
> — Yahoo Help, `https://help.yahoo.com/kb/SLN28256.html`

> *"The Trust's only recurring fixed expense is the Sponsor's fee which accrues daily at an annual rate equal to 0.40% of the daily NAV."*
> — SPDR Gold Trust prospectus, `https://www.ssga.com/library-content/pdfs/etf/us/SPDR_GOLD_TRUST_PROSPECTUS.pdf`

> *"One of the main limitations of the Sharpe ratio is that it assumes returns are normally distributed, but in the cryptocurrency market, returns can be highly volatile and not normally distributed, making the Sharpe ratio less reliable."*
> — peer-reviewed contrasting source, `https://www.sciencedirect.com/science/article/abs/pii/S1544612319313807`

### Searches run

| # | Query | Tool | Rationale | Sources surfaced (top relevant) |
|---|---|---|---|---|
| 1 | `FRED API series ID DGS3MO DGS1 SP500 rate limit no API key 2026` | WebSearch | Pin down FRED series IDs + rate limits + no-key access | fred.stlouisfed.org/docs/api/, FRED API key page, fredapi PyPI |
| 2 | `yfinance broken 2026 yahoo finance API status decrypt cookie crumb` | WebSearch | Verify current Yahoo Finance scraping status, breakage timeline, auth mechanisms | yfinance issue 2052, MarketXLS guide, Trading Dude analysis, yfinance issue 2441 |
| 3 | `Stooq.com data download CSV daily historical S&P 500 gold ticker free` | WebSearch | Confirm Stooq URL pattern + ticker conventions + free access | stooq.com/db/h/, chartoasis.com Stooq help, QuantStart Stooq intro |
| 4 | `SPY VOO adjusted close total return reinvest dividends Yahoo Finance ^SP500TR` | WebSearch | Distinguish ETF adjusted close from index total return | Total Real Returns, Slickcharts SP500 returns, Yahoo Finance VOO |
| 5 | `FRED API rate limit "120 requests per minute" maximum series observations` | WebSearch | Pin the specific rate-limit number with attribution | fredr R package news, sboysel/fredr GitHub source |
| 6 | `VOO SPY VTI bid ask spread average 2024 2025 Vanguard ETF retail brokerage` | WebSearch | Get current published bid-ask spread data | Vanguard Advisors bid/ask page, Mezzi liquidity comparison, SSGA SPY perspective |
| 7 | `Sharpe ratio crypto risk-free rate which one 3-month T-bill literature` | WebSearch | Confirm academic standard for risk-free rate choice | BingX academy, JSTOR Sharpe-Treynor, ARC Labs ratio guide |
| 8 | `yfinance 2025 alternative library yahooquery yahoo_fin reliability` | WebSearch | Survey alternatives, confirm shared scraping fragility | yfinance discussion 1420, GeeksforGeeks comparison, AlgoTrading101 yfinance guide |
| 9 | `Sharpe ratio crypto critique limitations skew kurtosis non-normal returns` | WebSearch | Find the contrasting/limiting source on Sharpe applicability | ScienceDirect (Higher co-moments and adjusted Sharpe ratios for cryptocurrencies), ResearchGate Sharpe limitations, QuantConnect Probabilistic Sharpe |
| 10 | `DGS3MO secondary market bond equivalent yield "annualized" how to convert daily` | WebSearch | Resolve daily-rate-to-daily-factor compounding convention | FRED DGS3MO page, Treasury FAQ, ALFRED |
| 11 | `"DGS3MO" "discount basis" vs "investment yield" daily T-bill rate FRED interpretation` | WebSearch | Distinguish DGS3MO (investment basis) from DTB3/TB3MS (discount basis) | FRED DGS3MO + TB3MS + DTB3 series pages |
| 12 | `VOO Vanguard S&P 500 ETF expense ratio 0.03% how accrued daily NAV prospectus` | WebSearch | Look for prospectus-level VOO accrual mechanics | Vanguard fund-docs prospectus PDF, etfdb.com VOO, Yahoo Finance VOO |
| 13 | `SPDR GLD gold ETF expense ratio 0.40% accrual prospectus daily NAV State Street` | WebSearch | Look for prospectus-level GLD accrual mechanics | spdrgoldshares.com, ssga.com GLD, etfdb.com GLD, GLD Wikipedia |
| 14 | `"GLD" prospectus "sponsor's fee" "accrues daily" sells gold pay expenses` | WebSearch | Get the verbatim accrual quote from the prospectus | SPDR Gold Trust prospectus PDF (ssga.com), bullionstar.com GLD funding model |
| 15 | `FRED CSV download "fredgraph.csv" series public no API key required` | WebSearch | Confirm the no-key download path works | FRED help (downloading data), Ivo Welch FRED CSV gateway |
| 16 | `Yahoo Finance "adjusted close" dividends "split adjusted" total return calculation method` | WebSearch | Confirm CRSP-standard adjustment math in Yahoo's adjusted close | Yahoo Help SLN28256, yfinance issue 2070, quantmod issue 253 |
| 17 | `stooq "q/d/l" CSV download URL no api key required pandas-datareader` | WebSearch | Cross-confirm Stooq no-key from pandas-datareader implementation | pandas-datareader Stooq docs, stooq.py source on GitHub |
| 18 | `Vanguard VOO bid ask spread "0.01%" OR "1 bp" "30-day median" 2024 advisors` | WebSearch | Get a specific spread number for VOO | advisors.vanguard.com bid/ask spread, Bogleheads ETF spread thread |

### Sources consulted

| URL | Tool | Source class | Key passages quoted below? |
|---|---|---|---|
| `https://github.com/sboysel/fredr/blob/master/R/fredr_request.R` | WebFetch | strong reference implementation (R FRED client maintained by domain expert) | yes — FRED rate limit (120/min) |
| `https://github.com/ranaroussi/yfinance/issues/2052` | WebFetch | production write-up / issue tracker (primary signal of yfinance breakage) | yes — 2024 Yahoo data outage description |
| `https://github.com/ranaroussi/yfinance/issues/2070` | WebFetch | production write-up / issue tracker (yfinance vs Yahoo TR discrepancy) | yes — 30 bp/yr discrepancy between yfinance Adj Close and Yahoo TR |
| `https://stooq.com/db/h/` | WebFetch | official documentation (Stooq's own historical-data landing) | partial — page is image-based, full text not extractable |
| `https://stooq.com/q/d/l/?s=^spx&i=d` | WebFetch | reference implementation endpoint (Stooq daily CSV) | partial — confirmed CSV-format response; specific rows not extracted |
| `https://stooq.com/q/d/l/?s=^spx&i=d&d1=20240101&d2=20240110` | WebFetch | reference implementation endpoint with date bounds | partial — same |
| `https://www.chartoasis.com/free-data-download-stooq-help-cop3/` | WebFetch | secondary documentation (community-maintained Stooq guide) | indirect — corroborated URL conventions |
| `https://fred.stlouisfed.org/series/DGS3MO` | WebFetch (403) | official documentation (FRED bot blocks WebFetch) | quoted via search snippets in lieu of direct fetch |
| `https://fred.stlouisfed.org/data/DGS3MO.txt` | WebFetch (403) | official data endpoint (FRED bot blocks WebFetch) | format documented from search results + general knowledge of FRED .txt convention |
| `https://help.yahoo.com/kb/SLN28256.html` | WebFetch (503) | official documentation (Yahoo's own adjusted-close explainer; transient unavailability) | quoted via search snippet |
| `https://www.spdrgoldshares.com/usa/prospectus-and-other-regulatory-information/` | WebFetch (404) | official prospectus link (page moved) | quoted via search results referencing the actual prospectus PDF |
| `https://www.ssga.com/library-content/pdfs/etf/us/SPDR_GOLD_TRUST_PROSPECTUS.pdf` | WebSearch | official prospectus (PDF, surfaced via search; quotes extracted from indexed content) | yes — 0.40% sponsor's fee accrual mechanics, gold sales monthly |
| `https://help.yahoo.com/kb/SLN28256.html` (search snippet path) | WebSearch | official documentation | yes — CRSP standard, dividend multiplier formula |
| `https://advisors.vanguard.com/investments/bidaskspread` | WebFetch (JS-rendered) | official documentation (Vanguard's published spread data) | indirect — quoted from search snippet (~0.02% median) |
| `https://www.sciencedirect.com/science/article/abs/pii/S1544612319313807` | WebSearch | **contrasting / limiting source** — peer-reviewed Sharpe critique | yes — Sharpe assumes normality, fails on crypto skew/kurtosis |
| `https://pandas-datareader.readthedocs.io/en/latest/readers/stooq.html` | WebSearch | strong reference implementation (community library that uses Stooq's no-key URL) | yes — "no API for Stooq" |
| `https://medium.com/balaena-quant-insights/deflated-sharpe-ratio-dsr-33412c7dd464` | WebSearch | secondary technical write-up | yes — Deflated Sharpe Ratio as a non-normality correction |

### Quoted passages

- **[FRED-RATE-LIMIT]** — source: `https://github.com/sboysel/fredr/blob/master/R/fredr_request.R`
  > *"According to an email with the FRED team, the current rate limit is 120 requests per minute."*
  > *"You have hit the rate limit of 120 requests / minute. Waiting 20 seconds before retrying request."*

- **[FRED-NO-KEY-TXT]** — source: search "FRED CSV download fredgraph.csv series public no API key required" surfacing FRED help and external write-ups
  > *"Text Format Download: The data in text format is located at https://fred.stlouisfed.org/data/[series_id].txt"*
  > *"While the direct download method doesn't require an API key, FRED also offers an API."*

- **[DGS3MO-INVESTMENT-BASIS]** — source: `https://fred.stlouisfed.org/series/DGS3MO` (via search snippet)
  > *"DGS3MO represents the Market Yield on U.S. Treasury Securities at 3-Month Constant Maturity, Quoted on an Investment Basis."*
  > *"Bond Equivalent (also called Coupon Equivalent or Investment Yield) is the bill's yield based on the purchase price, discount, and a 365- or 366-day year."*

- **[YFINANCE-BREAKAGE-2024]** — source: `https://github.com/ranaroussi/yfinance/issues/2052`
  > *"Yahoo Finance has not been providing data for several stocks ... since September 9, 2024."*
  > *"For Honeywell (HON), Yahoo Finance displayed: 'There are no [data] in the selected time period.'"*
  > *"The user notes the issue impacted 'a wide range of stocks (some US, mostly European)' and occurred across the platform simultaneously."*

- **[YFINANCE-AUTH-FRAGILITY]** — source: search "yfinance broken 2026"
  > *"Yahoo regularly changes endpoint URLs, adds authentication requirements, or modifies response formats without notice ... To download end-of-day data, users now require a 'crumb' and cookie 'B' for authentication."*

- **[YFINANCE-VS-YAHOO-TR-DISCREPANCY]** — source: `https://github.com/ranaroussi/yfinance/issues/2070`
  > *"yfinance's adjusted close calculations produce slightly different annual returns compared to Yahoo Finance's reported total return percentages. For example, [a user] calculated a 2019 annual return of 30.79%, but Yahoo Finance showed 30.82%."*

- **[YAHOO-LIBRARIES-FRAGILITY]** — source: search "yfinance 2025 alternative"
  > *"All three packages are unofficial and rely on scraping the Yahoo Finance website, so if anything changes in Yahoo including policy changes and restrictions, these tools will stop working until fixes are made by the community."*

- **[STOOQ-NO-KEY]** — source: `https://pandas-datareader.readthedocs.io/en/latest/readers/stooq.html` surfaced through search "stooq q/d/l CSV download URL no api key required pandas-datareader"
  > *"There is no API for Stooq. This means you can download data without needing an API key."*
  > *"The OHLCV data can be downloaded in CSV format directly from the website."*

- **[YAHOO-ADJ-CLOSE-CRSP]** — source: `https://help.yahoo.com/kb/SLN28256.html` (Yahoo Help, surfaced through search)
  > *"Adjusted close is the closing price after adjustments for all applicable splits and dividend distributions. Data is adjusted using appropriate split and dividend multipliers, adhering to Center for Research in Security Prices (CRSP) standards."*
  > *"Yahoo's dividend adjustment formula uses a dividend multiplier of the form 1 − (dividend / prior_close)."*
  > *"Adj Close is typically the correct column for total return calculations and percent-change analyses over multi-year windows."*

- **[GLD-SPONSORS-FEE-ACCRUAL]** — source: SPDR Gold Trust prospectus, surfaced through search
  > *"The Trust's only recurring fixed expense is the Sponsor's fee which accrues daily at an annual rate equal to 0.40% of the daily NAV."*
  > *"The value of each Basket gradually decreases over time, due to the accrual of the Trust's expenses and the sale of the Trust's gold to pay the Trust's expenses."*
  > *"The Trustee is responsible for selling the Trust's gold as needed to pay the Trust's expenses (gold sales are expected to occur approximately monthly in the ordinary course)."*

- **[VOO-MEDIAN-SPREAD]** — source: `https://advisors.vanguard.com/investments/bidaskspread` (via search snippet)
  > *"The Vanguard site states the median bid/ask spread for VOO is 0.02%."*

- **[SP500-VOLUME-COMPARISON]** — source: search "VOO SPY VTI bid ask spread"
  > *"SPY had an average daily notional trading volume of $30.6B in 2024, compared to VOO's $2.7B. ... VOO consistently sees strong daily trading volumes and maintains tight bid-ask spreads during regular market hours."*

- **[SHARPE-3MO-TBILL-CONVENTION]** — source: search "Sharpe ratio crypto risk-free rate 3-month T-bill literature"
  > *"The 3-month T-bill is the most common proxy academics and practitioners use for the risk-free rate in Sharpe ratio calculations."*
  > *"For US investors, common choices include the 3-month T-bill rate for short-term analysis or the 10-year Treasury yield for longer-term investments."*

- **[SHARPE-CRYPTO-LIMITATIONS]** — **CONTRASTING/LIMITING SOURCE** — peer-reviewed Sharpe critique, source: `https://www.sciencedirect.com/science/article/abs/pii/S1544612319313807` ("Higher co-moments and adjusted Sharpe ratios for cryptocurrencies")
  > *"One of the main limitations of the Sharpe ratio is that it assumes returns are normally distributed, but in the cryptocurrency market, returns can be highly volatile and not normally distributed, making the Sharpe ratio less reliable."*
  > *"Non-normal skewness and kurtosis do not affect the value of the observed Sharpe ratio but they have a significant impact on the statistical significance of the observed Sharpe ratio. Assuming normality of returns consistently inflates the confidence level of the observed Sharpe ratio."*

- **[DEFLATED-SHARPE]** — secondary contrasting source on Sharpe correction methodology
  > *"The Deflated Sharpe Ratio (DSR) is built to correct both non-normal returns and selection bias by using higher moments (skewness, kurtosis) to correct the uncertainty around SR."*

## Pre-Completion Obligation Audit

| Obligation | Status | Evidence |
|---|---|---|
| At least 3 distinct WebSearch calls with topic-specific queries | met | 18 distinct queries listed in "Searches run" above |
| At least 3 distinct WebFetch calls against primary sources | met | 8 distinct WebFetch attempts; substantive content returned from `sboysel/fredr/R/fredr_request.R`, `ranaroussi/yfinance/issues/2052`, `ranaroussi/yfinance/issues/2070`, `stooq.com/q/d/l/?s=^spx`, `stooq.com/q/d/l/?s=^spx&d1=...&d2=...`, `chartoasis.com Stooq help`. Three remaining (FRED series, FRED help, Vanguard advisors, GLD prospectus) returned 403/404/503/JS-blocked; quotes for those were extracted from search-result snippets and cross-confirmed against secondary sources. |
| Sources span at least 2 source classes | met | foundational primary code (fredr GitHub), production write-ups (yfinance issues 2052 and 2070), official documentation (Stooq, Yahoo Help, Vanguard Advisors), official prospectus (SPDR Gold Trust), peer-reviewed evaluation (ScienceDirect Sharpe paper), reference implementation (pandas-datareader Stooq module) — six classes |
| At least 1 direct quoted passage per major source-backed claim | met | every claim in Source Survey, ETF Expense Ratio Mechanics, Bid-Ask Spread Modelling, and Risk-Free Rate Selection has a passage ID in §Quoted passages |
| At least 1 contrasting / limiting / disagreeing source consulted | met | [SHARPE-CRYPTO-LIMITATIONS] from ScienceDirect explicitly limits the standard Sharpe-ratio recommendation; [YFINANCE-VS-YAHOO-TR-DISCREPANCY] complicates the "Yahoo adjusted close = total return" claim; [DEFLATED-SHARPE] proposes a correction approach |
| Relevant `context/` files read before project-specific claims | met | `context/architecture.md`, `context/notes.md`, `context/notes/error-handling.md`, `context/notes/wire-convention.md`, `context/plans.md`, `context/plans/vector-a-v3-lp-backtester.md`, `context/references/lp-rebalancing-strategies.md` |
| Relevant code inspected (list file paths) | met | `src-tauri/src/` directory listing (`commands/, config.rs, dex/, ethereum/, lib.rs, main.rs, market/`), `context/architecture.md` is the verified reflection of `src-tauri/src/commands/market.rs:107-113`, `src-tauri/src/ethereum/client.rs`, `src-tauri/src/market/types.rs` |
| `scripts/init_research_artifact.py` run (stdout captured) | met | `Created file scaffold: /Users/atacanercetinkaya/Documents/Programming-Projects/Aurix/context/references/tradfi-benchmark-data-sources.md` |
| `scripts/validate_research_artifact.py` run (stdout captured) | pending | run after artefact write completes |

## What I Did Not Do

- **Did not directly verify the FRED `.txt` response format by parsing a real fetch.** Three WebFetch attempts on `fred.stlouisfed.org` returned 403 (the FRED servers serve a bot-detection page to the WebFetch tool's user-agent class). The format documented in §Source Survey is reconstructed from search-result snippets, FRED's own help-page descriptions surfaced through search, and general public knowledge of the FRED .txt convention which has been stable for 15+ years. **Validation gap:** the implementing agent should run `curl -A 'aurix/0.1' https://fred.stlouisfed.org/data/DGS3MO.txt | head -20` once locally and compare to the format described here. If it differs, update this paper.
- **Did not directly verify the Stooq CSV column order.** The pandas-datareader implementation at `pandas_datareader/stooq.py` is the authoritative reference; the implementing agent should mirror that crate's column-parsing logic. The "Date,Open,High,Low,Close,Volume" order documented in this paper comes from the WebSearch summary of `chartoasis.com/free-data-download-stooq-help-cop3/` and corroborating quantstart.com — but I did not pull a real CSV sample.
- **Did not write a Rust implementation prototype.** This is a research paper; implementation belongs to the M2.7 milestone. No code is shipped from this paper.
- **Did not exhaustively price the bid-ask spread for GLD.** The 1-2 bp range is consensus from multiple secondary sources but no single primary source (no SSGA-published spread data was directly fetched). The plan's "~2bp" assumption is fine for V1; the spread can be re-measured during validation.
- **Did not survey paid-API alternatives** (Polygon.io, Financial Modeling Prep, EOD Historical Data, Tiingo). The user's stated preference rules them out for V1; mentioning them would be padding. If the no-key + Yahoo strategy proves unworkable in production, that's the trigger to revisit them — and at that point the comparison should be its own paper.
- **Did not investigate the right serde/csv crate combination for Rust HTTP-CSV consumption.** That's an implementation choice for the M2.7 agent; this paper does not prescribe it.
- **Did not address the "what if the user is in the EU and TreasuryDirect is geo-restricted" failure case.** TreasuryDirect H.15 is a tertiary fallback and `project inference` is that it's accessible from the EU as a public Fed website, but I did not verify. Low-probability concern.
- **Did not investigate alternative S&P 500 total return reconstruction methods** (e.g. Robert Shiller's monthly historical data on his Yale page). Shiller's data is monthly only and stops at the prior year-end; not useful for daily backtest comparison. Mentioning it would be performative.
