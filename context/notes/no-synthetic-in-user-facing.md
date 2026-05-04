# No Synthetic Data in User-Facing Flows

## Current Understanding

Synthetic / mock / fabricated data is permitted in:
- **unit tests** (every `#[cfg(test)] mod tests` block uses synthetic data),
- **integration tests** (`backtest/mod.rs::tests::build_synthetic_swaps`, `validation/synthetic.rs`),
- **dev-only Tauri commands** (`run_lp_synthetic_ingest` is exposed as an IPC for local-dev only),
- **`#[ignore]`d live-RPC tests** (the gating prevents accidental use in the default test path).

Synthetic data is **never** permitted in:
- the LP page's auto-run pipeline (`LpBacktestPage`),
- the Tab 1 arbitrage live read path (`fetch_market_overview`),
- any path that produces numbers the user reads on the dashboard.

The frontend's auto-run pipeline ends in an explicit error banner ("Could not reach chain head — check network or configure a key") rather than falling through to synthetic. This was made explicit in commit 391eadd's "Synthetic data dropped from auto-run pipeline; live sources only" change.

## Why

**Hiring-portfolio framing.** Aurix's strategic positioning is the crypto-domain hiring project; depth-on-one-vector beats breadth-across-shallow-tabs. A dashboard that fabricates numbers when external services fail is a worse signal to a quant-LP audience than an honest empty state. The empty state communicates "this is real-data tooling that respects the data source"; the synthetic fallback would communicate "this is a demo that hides failure."

**Trust-in-numbers principle.** Every dashboard number a user sees should map back to a real on-chain or live-API source. If the auto-run pipeline's first step fails, the second step doesn't get to "guess" — the chain stops, the user sees the failure, and the engineer has a real bug to fix. Confused users and confused engineers are both better than confidently-fabricated numbers.

## How It's Enforced

- The synthetic-data Tauri command (`run_lp_synthetic_ingest`) is structurally separate from the live ingest command (`run_lp_ingestion`). The frontend never calls the synthetic command from the auto-run pipeline.
- Synthetic rows are tagged with `SYNTHETIC_TX_HASH = "0x...deadbeef"` (per [storage-conventions](storage-conventions.md)) so they can be wiped by `delete_synthetic_swaps_in_range` without touching live rows.
- The `LpBacktestPage` auto-run useEffect calls `runLpIngestion` (live) directly. There is no fall-through to `runLpSyntheticIngest` (dev-only).
- Per [free-data-fallback-chain](free-data-fallback-chain.md), the live ingest's tiered fallback ends in `CommandError`, not in synthetic fallback.

## Guiding Principles

- New IPCs that produce dashboard data must use live sources only (or follow the existing tiered fallback to empty state).
- The dev-only synthetic IPCs may be added when needed for local development, but should be separately named (`run_lp_synthetic_*`) so the separation is grep-visible.
- Tests are exempt — synthetic data is the right primary test fuel for an offline test suite.
- The `MockHttpFetcher` trait in `benchmarks/http.rs` is permitted for tests; the live `ReqwestFetcher` is the only path the production code uses.

## Verification Pattern

Every dashboard-facing IPC should have an answer to: "what does this command return when every data source fails?"

| IPC | Failure terminal |
|---|---|
| `run_lp_ingestion` | `CommandError` (no synthetic fallback) |
| `lp_get_chain_head` | `CommandError` (Alchemy → public RPC → error) |
| `lp_token_usd_prices` | `CommandError` (DefiLlama outage = error, not stub prices) |
| `lp_pool_metadata` | `CommandError` (subgraph outage = error) |
| `lp_fetch_benchmark_series` | `CommandError` (FRED/DefiLlama outage = error, not zeros) |
| `fetch_market_overview` (Tab 1) | error string; UI shows error banner instead of stubs |

## Cross-references

- Related: [free-data-fallback-chain](free-data-fallback-chain.md) (the source ordering that ends in empty state).
- Storage: [storage-conventions](storage-conventions.md) §"Synthetic-vs-live separation via SYNTHETIC_TX_HASH".
- Systems: [lp-backtest-gui](../systems/lp-backtest-gui.md), [ingest](../systems/ingest.md).
- Auto-memory record: `Aurix No Synthetic In User-Facing` (in user's MEMORY.md — this note formalises that preference into the project's context layer).
