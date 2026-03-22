# Insights

## Scope / Purpose

- This document defines Aurix's cross-tab insights layer: a rule-based system that interprets live or historical analytics and turns them into concise, user-facing observations, watch signals, and action-oriented summaries.
- This document exists at the shared context level because insights are intended to become a reusable product concept across multiple tabs rather than a one-off Tab 1 widget.

## Current Implemented System

- No dedicated insights pane, insight engine, or event feed currently exists in the product.
- Tab 1 currently exposes the raw ingredients an insights system would need: live venue prices, median-relative deviation, venue spread, and a simple gas-adjusted estimate.
- The current analytical interpretation remains implicit in the chart and detail panels rather than being surfaced as explicit textual observations.

## Implemented Outputs / Artifacts

- `src/features/arbitrage/components/MarketChart.tsx` currently derives raw-price, deviation, spread, and gas-adjusted chart views from in-session history.
- `src/features/arbitrage/ArbitragePage.tsx` currently surfaces current spread, median price, venue list, and simple net spread estimate in the detail panel.
- There is currently no shared insight model, no prioritisation layer, no event classification, and no dedicated insight presentation block.

## In Progress / Partially Implemented

- The project now has a defined cross-tab product direction for an insights layer, but it is still documentation-only.
- The current Tab 1 feed already provides enough live state to support a first narrow rule-based insights implementation without introducing new external data sources.
- The current system does not yet separate stable current-state interpretation from change/event interpretation, but that split is now part of the intended direction.

## Planned / Missing / To Be Changed

- Add a reusable `Insights` block to product surfaces so automated interpretation can complement the charts rather than forcing the user to infer everything visually.
- Treat the insights system as a rule-based interpretation layer over existing analytics outputs rather than as a free-form narrative generator.
- Split insights into two output forms:
  - `Live summary` for the current market state.
  - `Event feed` for noteworthy transitions or threshold crossings.
- Keep the first implementation conservative in wording so the product does not imply execution-grade trading certainty where only simple analytical estimates exist.
- Keep insight generation outside the chart component so later GUI and TUI surfaces can consume the same interpretation outputs.

- The first implemented slice should be Tab 1 only and should stay narrow:
  - Surface current highest-priced and lowest-priced venue.
  - Surface current venue spread and whether it is widening, narrowing, or stable over a short recent window.
  - Surface strongest positive and strongest negative deviation versus the current median.
  - Surface current gas-adjusted estimate and whether it is positive, negative, improving, or deteriorating.
  - Surface whether a meaningful condition has persisted for multiple samples instead of firing only on one-sample noise.
  - Surface watch-level messages only when a threshold or persistence rule is met.

- The Tab 1 insights catalogue should include these immediate categories:
  - `Snapshot insights`: current highest venue, lowest venue, spread, median-relative outlier, current gas-adjusted estimate.
  - `Delta insights`: venue moved up, venue moved down, spread widened, spread narrowed, gas-adjusted estimate improved, gas-adjusted estimate deteriorated.
  - `Persistence insights`: a venue stayed richest or cheapest for multiple samples, spread remained elevated for multiple samples, positive gas-adjusted estimate persisted beyond a short burst.
  - `Ranking insights`: richest venue changed, cheapest venue changed, spread-defining pair stayed the same, spread-defining pair flipped.
  - `Actionability insights`: spread exceeds gas estimate, visible spread is erased by gas, current best hypothetical buy venue, current best hypothetical sell venue, current setup is worth watching, current setup is not yet attractive.
  - `Caution insights`: signal is based on shallow session history, signal appears one-sample-only, current move is broad-based rather than venue-specific, current move is venue-specific rather than market-wide.

- The Tab 1 pane should be structured as:
  - one primary insight that summarises what matters most right now,
  - two to four secondary live insights,
  - a recent event feed of timestamped changes.

- The first implementation should enforce anti-noise controls from day one:
  - every insight should carry a type,
  - every insight should carry a severity or priority,
  - repeated insights should be cooled down instead of re-emitted every refresh,
  - some insights should require persistence across multiple samples before appearing,
  - stale insights should expire or downgrade cleanly.

- The first implementation should use conservative user-facing wording:
  - prefer `currently`, `appears`, `estimate`, `worth watching`, and `signal`,
  - avoid wording that implies guaranteed arbitrage or executable profitability.

- A practical first-release insight model should support at least:
  - `type`: spread, deviation, gas, price-move, ranking, persistence, caution,
  - `severity`: info, watch, notable, actionable,
  - `freshness`: new, ongoing, resolved,
  - `message`: concise user-facing copy,
  - `timestamp` or sample position for feed ordering,
  - optional metric payload so surfaces can show exact values without re-deriving text.

- Broader future-tab insights should extend the same product concept:
  - Tab 2 should generate insights around range activity, fee generation, impermanent loss pressure, and strategy comparison.
  - Tab 3 should generate insights around wallet exposure changes, position concentration, unrealised PnL movement, and protocol-specific risk.
  - Tab 4 should generate insights around gas spikes, cheap windows, pattern deviations, and timing recommendations.
  - Tab 5 should generate insights around rising correlation, concentration risk, volatility shifts, Value-at-Risk deterioration, and stress-test outcomes.

- The following ideas are intentionally deferred until later because they depend on persistence, longer-lived baselines, or stronger analytical confidence:
  - session superlatives such as `largest this session` where reset semantics are not yet fully productised,
  - long-horizon confidence scoring,
  - historical trend summaries across app restarts,
  - opportunity frequency analytics,
  - strongly prescriptive trade-style advice,
  - portfolio-style recommendations outside the scope of the current tab.

## Notes / Design Considerations

- The insight layer should interpret existing analytics; it should not duplicate venue reads, chart calculations, or protocol decoding.
- Stable state and discrete events should remain separate so the pane does not feel like a spammy log of minor fluctuations.
- The user should be able to scan the pane and understand both what the market looks like now and what changed recently.
- Anti-noise rules are mandatory because a one-second refresh cadence will otherwise produce repetitive or low-signal text.
- The first version should be deterministic and inspectable so the logic is easy to validate against the underlying chart.
- The future TUI should consume the same underlying insight outputs instead of inventing its own separate rule set.

## Discarded / Obsolete / No Longer Relevant

- Treating the insights concept as a Tab 1-only feature is no longer the intended direction.
- Treating the insight pane as a flat list of equally weighted messages is not the intended direction.
- Treating the insights system as execution-grade financial advice is not the intended direction.
