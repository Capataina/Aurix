# Headline

## Scope / Purpose

- The M2.8 capital-allocation verdict synthesis. Composes monthly LP backtest runs (best/naive/median) against multi-asset benchmarks (Aave / Lido / S&P 500 / gold / T-bill / HODL), classifies each month into a vol regime (adaptive-tercile classifier), counts the months LP beat each benchmark, and synthesises a verdict-prose paragraph the user reads first on the LP dashboard.
- Answers the question "should I have just held / staked / yielded instead?" — the canonical hiring-signal output Aurix is built around.

## Boundaries / Ownership

- Owns: monthly aggregation of [backtest](backtest.md) runs, vol-regime classifier (adaptive-tercile), per-month LP-vs-benchmark winner determination, beat-count tabulation, verdict-prose synthesis (the user-facing English paragraph).
- Does **not** own: the per-month backtest itself (delegated to [backtest](backtest.md)), the benchmark series ([benchmarks](benchmarks.md)), the visual rendering ([lp-backtest-gui](lp-backtest-gui.md)).

## Current Implemented Reality

```text
headline/
├── mod.rs              # HeadlineConfig + HeadlineRunner + HeadlineOutput
└── verdict.rs          # vol-regime classifier + verdict prose synthesis (414 lines)
```

**Adaptive-tercile vol regime classifier.** For the backtest window, computes realised volatility per month, then partitions months into low/mid/high terciles by the *observed* volatility distribution (not fixed thresholds). This adapts to the asset's historical vol regime — a "low-vol month" for ETH/USDC is different from a "low-vol month" for WBTC/USDC. The classifier output is one of {`low`, `mid`, `high`} per month.

**Monthly winner determination.** For each month, the runner runs a per-month sub-backtest with three LP variants:
- **Best** — the cell with the best in-month Sharpe across the strategies grid (selection-bias-adjusted via Deflated Sharpe in the future, see [strategies](strategies.md) Known Issues)
- **Naive** — fixed wide range (±100 ticks), Static rule, 50/50 deposit split
- **Median** — median-Sharpe cell across the grid

Each variant's monthly return is compared to the benchmarks' monthly returns:
- Aave V3 USDC supply APY (DefiLlama)
- Lido stETH APY (DefiLlama)
- S&P 500 (FRED)
- Gold (FRED LBMA)
- T-bill 3-month (FRED DGS3MO)
- HODL (price-only)

The `months_lp_beat_*` aggregate counts (added in `V002__multi_asset_headline.sql` migration) record how many months each LP variant beat each benchmark. The verdict prose synthesises this into an English paragraph: *"In low-vol months, naive LP beat Aave 7/12 times with a median spread of +1.4% APY. In high-vol months, naive LP underperformed S&P by 9/12 with a median spread of -2.1%. Best-cell LP beat all benchmarks across both regimes."*

## Key Interfaces / Data Flow

| Boundary | Direction | Shape | Notes |
|---|---|---|---|
| `commands::lp::run_lp_headline(config)` → headline runner | inbound | `HeadlineConfig` | from frontend |
| headline runner → [backtest](backtest.md) | outbound (call) | per-month `Engine::simulate` calls (3 variants × N months) | sequential today |
| headline runner → [benchmarks](benchmarks.md) | outbound (read) | per-month benchmark series via storage | requires benchmark series pre-fetched |
| headline runner → [storage](storage.md) | outbound (write) | `(HeadlineRunSummary, Vec<HeadlineMonthlyRow>)` | idempotent on `config_hash` |
| `commands::lp::lp_query_headline_monthly` → storage | outbound (read) | `Vec<HeadlineMonthlyRow>` | for re-render path |

**`HeadlineMonthlyRow` shape** (per `storage/headline.rs`): month start date, vol regime (low/mid/high), LP variant (best/naive/median), LP monthly return %, plus per-benchmark monthly return %s (Aave / Lido / SP500 / gold / T-bill / HODL). The frontend's `MultiAssetCompareBlock` and `RegimePanelBlock` consume these rows directly.

## Implemented Outputs / Artifacts

- `headline_runs` row + `headline_monthly` rows in [storage](storage.md).
- The verdict prose paragraph rendered in `HeadlineVerdictBlock`.
- `V002__multi_asset_headline.sql` migration adds the `months_lp_beat_sp500/gold/tbill` + per-month asset returns columns.

## Known Issues / Active Risks

- **Selection bias in "best-cell LP" variant.** Picking the best Sharpe out of N cells inflates the expected value of "best LP wins." The Deflated Sharpe correction from `references/backtest-statistical-methodology.md` is not yet applied; documented as a [strategies](strategies.md) Known Issue and as a future addition. Downstream impact: the verdict's "best-cell beat all benchmarks" claim is statistically optimistic.
- **Adaptive-tercile classifier on short windows.** For a 3-month backtest the adaptive-tercile partitioning has only 3 buckets (one per regime), making "month X is low-vol" almost meaningless. The classifier is most reliable on 12+ month windows. Recorded as a [potential issue](../plans/code-health-audit/potential-issues.md) candidate (not yet filed).
- **`headline/verdict.rs` is 414 lines** — top-decile candidate with a `leave-as-is` deferred verdict in the audit (not deep-read this pass). A Pass-2 deep-read should confirm the verdict prose generator + regime classifier are correctly co-located.

## Partial / In Progress

- None.

## Planned / Missing / Likely Changes

- Deflated Sharpe correction for the best-cell selection variant.
- Confidence intervals on the "LP beat benchmark X 7/12 times" aggregates (Wilson interval or similar).
- Per-regime verdict variants — currently the verdict prose treats vol regimes uniformly; future versions could synthesise distinct paragraphs per regime.

## Durable Notes / Discarded Approaches

- **Adaptive-tercile over fixed thresholds.** Earlier design used 0.5%/2.0% daily-vol thresholds; these were calibrated for ETH/USDC and break badly on lower-vol pairs (DAI/USDC) or higher-vol pairs (WBTC/ETH). Adaptive partitioning self-calibrates per asset.
- **Three-variant LP** (best/naive/median) over single-variant. Earlier design showed only "best LP" — the verdict was "LP wins" essentially always, which is uninformative because of selection bias. Adding naive (no tuning) and median (typical user) reframes the question to "would a typical LP have won?" — which is the more honest framing.
- **Verdict prose synthesis over stat-table-only output.** Earlier design surfaced just the win-counts; the prose paragraph reframes the same numbers as a recommendation, which is what the user actually wants ("you should/shouldn't have LP'd in this market"). Both representations are produced; the prose is the headline.

## Obsolete / No Longer Relevant

- The pre-V002 schema (without `months_lp_beat_sp500/gold/tbill` + per-month asset returns) is gone via the V002 migration. The migration is forward-only; rollback would require manual data surgery.

## Cross-references

- Caller: `commands::lp::run_lp_headline`.
- Consumer of: [backtest](backtest.md) (per-month sims), [benchmarks](benchmarks.md) (per-month series), [storage](storage.md), [strategies](strategies.md) (best-cell selection from grid).
- Producer for: `headline_runs` + `headline_monthly` rows; verdict prose.
- Related research: `references/backtest-statistical-methodology.md`, `references/v3-lp-profitability-literature.md`.
