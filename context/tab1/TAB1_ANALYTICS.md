# Tab 1 Analytics

## Scope / Purpose

- This document tracks the analytical concepts Tab 1 uses to turn venue prices into comparative signals rather than just raw numbers.

## Current Implemented System

- Tab 1 currently exposes raw venue prices, deviation-from-median, venue spread, and a simple gas-adjusted net estimate in the GUI.
- The chart currently operates in one mode at a time so each analytical view can be read independently.
- Event markers currently represent positive values in the simple gas-adjusted estimate.

## Implemented Outputs / Artifacts

- The chart component in `src/features/arbitrage/components/MarketChart.tsx` derives the current analytical views from the in-session market history.
- The detail panel in `src/features/arbitrage/ArbitragePage.tsx` surfaces spread, median price, and a simple gas-adjusted estimate.

## In Progress / Partially Implemented

- The current gas-adjusted estimate is intentionally simple and should not yet be treated as execution-grade profitability logic.
- The chart behaviour still needs runtime refinement and validation to make the analytical modes reliably legible.
- Historical analytics are session-only because no persistence layer exists yet.

## Planned / Missing / To Be Changed

- Move analytics calculations out of the chart component and into a more reusable shared core.
- Add opportunity logging, thresholds, and richer event classification.
- Add persisted historical analytics so spread-over-time and related views are useful beyond the current session.
- Introduce clearer comparative views where raw prices, spread, deviation, and actionability remain readable without visual clutter.

## Notes / Design Considerations

- Raw price is input context, not the main analytical story.
- Spread, deviation, and actionability are the more important Tab 1 signals.
- A chart should answer a question clearly rather than merely displaying movement.

## Discarded / Obsolete / No Longer Relevant

- Treating raw price alone as the main Tab 1 story is obsolete as a design direction.
