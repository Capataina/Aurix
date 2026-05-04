# IPC Commands — Code Health Findings

**Systems covered:** `src-tauri/src/commands/{lp,market,telemetry}.rs`
**Finding count:** 3 (1 high modularisation, 1 medium perf, 1 low cleanup)

`commands/lp.rs` is the largest single file in the repository (628 lines) and the integration hub of the Vector A backend (fan-out 16 — it imports from backtest, ingest, headline, strategies, benchmarks, storage, math, config). It hit its current size in the 2026-05-03 sprint as commands were added one-by-one for each LP-pipeline stage. The file now mixes three structurally-distinct concerns: command registration / orchestration, synthetic-data scaffolding, and live HTTP fetching for benchmark and price feeds.

## Modularisation

### Split `src-tauri/src/commands/lp.rs` into three focused submodules

- [ ] Convert `commands/lp.rs` into `commands/lp/` with `mod.rs` + `synthetic.rs` + `prices.rs`. Move `run_lp_synthetic_ingest` + `synthetic_anchor_tick` + `synthetic_mock` into `synthetic.rs`. Move `lp_token_usd_prices` + DefiLlama types + `lp_fetch_benchmark_series` (HTTP-bearing commands) into `prices.rs`. Keep `lp_get_chain_head`, `run_lp_ingestion`, `run_lp_backtest`, `run_lp_grid`, `run_lp_headline`, `lp_query_*`, `lp_pool_metadata` in `mod.rs`.

**Category:** Modularisation
**Severity:** High
**Effort:** Medium
**Behavioural Impact:** None (mechanical move; all command symbols stay public, just under different module paths)

**Location:**
- `src-tauri/src/commands/lp.rs` (entire 628-line file)
- `src-tauri/src/lib.rs:52-72` (Tauri command handler list — paths update to `commands::lp::synthetic::run_lp_synthetic_ingest` etc.)

**Current State:**
The file packs:
- 71 lines of CommandError + parse helpers (lines 42-107)
- 25 lines of `lp_get_chain_head` (lines 71-95)
- 130 lines of pool metadata + DTO + `is_usd_stable` (lines 167-225)
- **115 lines of synthetic-ingest scaffolding** — `run_lp_synthetic_ingest` + `synthetic_anchor_tick` + `synthetic_mock` (lines 227-341). Self-contained: only depends on `MockArchiveSource`, `tick_to_sqrt_price_x96`, `Storage`, `SYNTHETIC_TX_HASH`, `EthLog`, `SWAP_TOPIC0`. Has no callers outside this file.
- 70 lines of small command wrappers (lines 343-414)
- 70 lines of `lp_fetch_benchmark_series` (lines 416-484, large match block)
- 60 lines of `lp_query_first_swap_price` + `FirstSwapInfo` (lines 486-543)
- 75 lines of `lp_token_usd_prices` + DefiLlama HTTP types (lines 552-625)

The synthetic block in particular is a complete dev-only sub-feature (sinusoidal mock data generation with a specific anchor tick and liquidity calibration). Reading the orchestration commands requires scrolling past 100+ lines of synthetic-data construction that is irrelevant to live-data flow.

**Proposed Change:**

```
src-tauri/src/commands/lp/
├── mod.rs           # CommandError + parse helpers + small command wrappers + ingestion path
├── synthetic.rs     # run_lp_synthetic_ingest + synthetic_anchor_tick + synthetic_mock
└── prices.rs        # lp_token_usd_prices + DefiLlama types + lp_fetch_benchmark_series
```

The synthetic submodule becomes self-explanatory: "this file owns the synthetic dev-only data path." The prices submodule becomes "this file owns the live HTTP price/benchmark fetches." Both isolated from the live ingestion + backtest orchestration.

In `lib.rs:52-72`, update the Tauri command handler list paths:
```rust
commands::lp::synthetic::run_lp_synthetic_ingest,
commands::lp::prices::lp_token_usd_prices,
commands::lp::prices::lp_fetch_benchmark_series,
// rest unchanged
```

**Justification:**
The three concerns are structurally orthogonal: synthetic data construction has zero overlap with live HTTP fetching, and live HTTP fetching has zero overlap with command orchestration. Each concern has its own dependency surface (synthetic uses `MockArchiveSource` + `tick_to_sqrt_price_x96`; prices uses `reqwest` + `DefiLlamaProvider`; orchestration uses `Engine`, `Ingester`, `HeadlineRunner`, `GridRunner`). Today the dependencies live in one giant `use` block (lines 11-40) at the top of `lp.rs`; the split makes each module's `use` block focused.

This is the classic Modularisation case from `analysis-categories.md` §3 — the file is significantly larger than peers (next largest backend file is 549 lines), the responsibilities are mixed (orchestration + dev-only scaffolding + HTTP), and the split would not introduce cross-module coupling (the three submodules don't need to import from each other).

**Expected Benefit:**
- `lp.rs` orchestration code (the most-read path) shrinks from ~628 to ~300 lines.
- Synthetic-data code becomes locatable by its module name rather than scattered alongside live commands.
- HTTP types and DefiLlama bindings become localised to one file, easier to swap when a new price provider is added.
- Module-level docs become focused: `synthetic.rs` doc-comments the dev-only contract; `prices.rs` doc-comments the rate-limit / no-key constraints.

**Impact Assessment:**
Zero functional change. The Tauri command names and signatures are unchanged; only the module path changes. The handler list in `lib.rs` updates the paths in lockstep. No consumer of these commands (frontend `api.ts`, tests, etc.) sees the change because `tauri::generate_handler!` resolves the paths at build time.

---

## Performance Improvement

### Reuse the `reqwest::Client` for `lp_token_usd_prices` instead of rebuilding per call

- [ ] In `src-tauri/src/commands/lp.rs:574-580`, `reqwest::Client::builder().timeout(...).build()` runs on every invocation of `lp_token_usd_prices`. Hoist the client into a `OnceCell<reqwest::Client>` initialised once at first use. The `ReqwestFetcher` used by the `lp_fetch_benchmark_series` path already follows the reuse pattern (see `benchmarks/http.rs`); `lp_token_usd_prices` is the outlier.

**Category:** Performance Improvement
**Severity:** Medium
**Effort:** Trivial
**Behavioural Impact:** None (reqwest's internal connection pool is exactly what reuse exists for)

**Location:**
- `src-tauri/src/commands/lp.rs:574-580` — per-call client construction

**Current State:**
```rust
let client = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(10))
    .build()
    .map_err(|e| CommandError {
        message: format!("token prices: client build failed: {e}"),
        key_required: None,
    })?;
```

Each call constructs a fresh `reqwest::Client`, which under the hood instantiates a fresh `reqwest::Client::builder()` configured pool plus the underlying `hyper` connection cache. The TLS handshake state is also fresh — even if the same TLS endpoint is hit on the next call, no connection-reuse happens.

`lp_token_usd_prices` is called by the frontend on each LP pipeline run for non-USD-quote pools, and by the LP page's auto-run when the user changes a setting. It's a hot-ish IPC, not a one-shot.

The benchmark commands route through `ReqwestFetcher` (`src-tauri/src/benchmarks/http.rs`), which presumably owns one `reqwest::Client`. The audit didn't read that file but the consistency note stands: if `ReqwestFetcher` reuses, `lp_token_usd_prices` should too.

**Proposed Change:**
Hoist the client into a `Lazy<reqwest::Client>` (using `once_cell::sync::Lazy`, already in use elsewhere in the project — see `math/q96.rs`). Or wrap it in a `OnceCell` initialised on first call. The 10s timeout is hard-coded today; if the implementing engineer wants per-call timeout flexibility, accept it as a parameter and apply via `RequestBuilder::timeout` instead.

```rust
static TOKEN_PRICE_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("default reqwest::Client builder cannot fail")
});

#[tauri::command]
pub async fn lp_token_usd_prices(...) -> Result<TokenPricesDto, CommandError> {
    let resp = TOKEN_PRICE_CLIENT.get(&url).send().await...
}
```

**Justification:**
Direct evidence from the `reqwest` documentation surfaced during research — `reqwest::Client` is designed to be cloned and shared, and the internal connection pool (`hyper::Client`) provides connection reuse across requests. Constructing a fresh client per call defeats the connection pool entirely. The `Lazy<Client>` pattern is exactly the Tauri-context-friendly way to share the client.

**Expected Benefit:**
Eliminates the per-call client construction cost (small but non-zero — TLS state setup is the largest piece). Enables connection reuse to `coins.llama.fi` across calls — the second and subsequent calls within a session benefit from pooled HTTPS connections. The DefiLlama API is one of the project's heaviest external dependencies (used by every non-USD-quote pool's per-run auto-fetch).

**Impact Assessment:**
Zero functional change. The HTTP behaviour is identical — same URL, same headers, same JSON body. Only the connection lifetime and reuse improves. The 10s timeout is preserved by the `Lazy::new` initialisation.

---

## Inconsistent Patterns

### Replace `eprintln!` fallthrough logging in `run_lp_ingestion` with the project's existing error-surfacing convention

- [ ] `src-tauri/src/commands/lp.rs:138, 150` use `eprintln!` to log the subgraph- and Alchemy-path failures during the tiered fallback. The rest of the project surfaces errors via the `thiserror::Error` enum + structured error types (per `notes/error-handling.md`); `eprintln!` is the only place stderr text appears in user-facing flows.

**Category:** Inconsistent Patterns
**Severity:** Low
**Effort:** Small
**Behavioural Impact:** Negligible (flagged) — removing the `eprintln!` lines changes what appears on the binary's stderr at runtime. For a desktop app this is rarely user-visible (Tauri does not surface stderr by default), but if the implementing engineer relies on `tail -f stderr` for debug, the messages disappear.

**Location:**
- `src-tauri/src/commands/lp.rs:138, 150` — fallthrough log lines

**Current State:**
The tiered ingestion fallback uses bare `eprintln!`:

```rust
match ingester.backfill(&pool_address, from_block, to_block).await {
    Ok(report) => return Ok(report),
    Err(e) => {
        eprintln!("[lp] subgraph backfill ({}) failed → user-rpc: {e}", chain.label());
    }
}
```

The project does not use `tracing` or `log` (per `Cargo.toml`); the convention is to wrap errors in `thiserror::Error` enums and surface them via `Result` to callers (per `notes/error-handling.md`). The `eprintln!` here is the only place in the IPC layer where text leaves through stderr.

The new `telemetry` recorder added in this sprint (`src/lib/telemetry.ts` + `src-tauri/src/commands/telemetry.rs`) is the project's chosen mechanism for capturing IPC events, errors, and lifecycle markers. The fallthrough information ("subgraph failed → trying user-rpc → trying public") is exactly the kind of structured event the telemetry recorder is designed for.

**Proposed Change:**
Either:
- Extend `IngestionReport` to carry a `fallback_path: String` field that names which tier ultimately succeeded ("subgraph" / "user-alchemy" / "public-rpc") and any tiers that errored along the way. The frontend reads this and surfaces it as a status line.
- Or emit a structured telemetry event inside the fallthrough using the existing `telemetry::record` mechanism (would require exposing the telemetry recorder to Rust, which doesn't exist today — this is the heavier path).

The audit recommends the lighter `IngestionReport.fallback_path` approach. The frontend already shows status messages from the pipeline ("Running live ingest…"); this is one more.

**Justification:**
Consistency — the project has chosen a structured error/status convention (notes/error-handling.md + the new telemetry recorder). `eprintln!` is the only inconsistent point. The signal it carries (which tier failed) is operationally useful and should not be discarded by removing the lines without replacement.

**Expected Benefit:**
Single error-surfacing convention across the IPC layer. Frontend gains visibility into which backfill tier succeeded — helpful for debugging connectivity-related dashboard issues. No more silent stderr messages that the user can't see anyway in a desktop app.

**Impact Assessment:**
Negligible-flagged. The current behaviour (stderr lines) is observable only when running from a terminal; removing it without replacement loses that signal. The proposed change preserves the signal in a more useful location (the frontend, via the IPC payload). Implementing engineer should keep one or the other — silently dropping is the wrong move.
