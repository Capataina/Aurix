# Arbitrage Analytics

## Scope / Purpose

- This document owns the in-session analytical layer for the current arbitrage feature: rolling history, summary metrics, chart-series derivation, event markers, and the rule-based insight model rendered by the frontend.

## Boundaries / Ownership

- This file owns the analytical responsibilities inside `src/features/arbitrage/ArbitragePage.tsx`, `src/features/arbitrage/components/MarketChart.tsx`, and `src/features/arbitrage/insights.ts`.
- It includes the logic that transforms a `MarketOverview[]` session history into chart values, summary metrics, insight cards, and event feed entries.
- It does not own backend data acquisition, DEX decoding, or purely presentational CSS concerns.

## Current Implemented Reality

- `ArbitragePage.tsx` maintains a 100-sample in-memory history and appends one new `MarketOverview` per successful refresh.
- The page computes current spread, median price, and a simple net spread estimate for the detail panel directly from the latest overview.
- `MarketChart.tsx` supports four mutually exclusive chart modes: raw prices, median-relative deviation, absolute venue spread, and a simple gas-adjusted estimate.
- The chart uses the same fixed `220,000` gas-unit assumption as the page-level net estimate and marks positive gas-adjusted values as event points when the events toggle is enabled.
- `insights.ts` derives a richer analytical view from the same history by computing per-sample median price, spread, gas cost, gas-adjusted value, richest venue, cheapest venue, and strongest deviations.
- The insight layer emits one primary card, up to four secondary cards, and up to four recent events based on persistence windows, baseline windows, and transition conditions.
- The current insight model is intentionally deterministic and rule-based rather than learned, probabilistic, or backed by persisted history.

## Key Interfaces / Data Flow

| Analytical stage | Owner | Current behaviour |
| --- | --- | --- |
| History accumulation | `ArbitragePage.tsx` | Keeps only the last 100 successful samples in memory |
| Detail metrics | `ArbitragePage.tsx` | Computes spread, median, and simple net spread estimate from the latest overview |
| Chart derivation | `MarketChart.tsx` | Recomputes mode-specific series and display metrics from the full session history |
| Insight derivation | `insights.ts` | Builds derived samples, primary/secondary insight cards, and recent events |
| Insight rendering | `InsightsPanel.tsx` | Displays the already-derived analytical view without mutating it |

- The same core concepts such as median, spread, and gas-adjusted value are currently re-derived in both `ArbitragePage.tsx`, `MarketChart.tsx`, and `insights.ts`.
- Positive-event markers in the chart and actionability language in insights both depend on the same simplified gas-adjusted estimate, even though they are implemented separately.

## Implemented Outputs / Artifacts

- `src/features/arbitrage/ArbitragePage.tsx` provides history accumulation and detail metric derivation.
- `src/features/arbitrage/components/MarketChart.tsx` provides chart-series derivation and SVG rendering inputs.
- `src/features/arbitrage/insights.ts` provides the rule-based analytical view model.
- `src/features/arbitrage/components/InsightsPanel.tsx` renders the resulting cards and event list.

## Known Issues / Active Risks

- Analytical primitives are duplicated across files, so a future formula change can drift between detail metrics, charts, and insight text.
- The fixed `220,000` gas-unit assumption is a coarse heuristic and should not be treated as execution-grade profitability logic.
- History is session-only, so baseline windows, persistence runs, and events reset on restart and do not represent long-lived market behaviour.
- The chart treats any positive gas-adjusted value as an event marker, which is useful for scanning but is not a full opportunity-classification model.
- There is no test coverage for median calculation, run-length logic, baseline comparisons, or event transitions.

## Partial / In Progress

- The analytics layer is already more than a simple live ticker, but it still lives entirely inside feature code rather than a reusable shared analytical core.
- The interpretation model is coherent enough for the current dashboard, but the formulas and thresholds are still shaped by convenience rather than by a validated longer-lived dataset.

## Planned / Missing / Likely Changes

- Extract shared analytical primitives so charting, detail metrics, and insight derivation stop recomputing the same concepts independently.
- Add persisted historical analytics before describing the repository as having durable baselines, opportunity frequency tracking, or restart continuity.
- Add configurable thresholds and richer event classes only after the current signals are validated against larger sample sets.
- Add tests around median, spread, deviation, and persistence behaviour once the formulas stop moving structurally.

## Durable Notes / Discarded Approaches

- The analytics layer is intentionally interpretive rather than prescriptive; its current wording aims to explain venue disagreement and gas-aware state, not to recommend trading actions with execution confidence.
- Raw price alone is not the whole analytical story in this repository; spread, deviation, venue ranking, and persistence are the more meaningful signals.

## Obsolete / No Longer Relevant

- Treating the current screen as a single raw-price display is no longer accurate.
- Treating the gas-adjusted estimate as a persisted opportunity log or robust execution model is still unsupported by the codebase.
