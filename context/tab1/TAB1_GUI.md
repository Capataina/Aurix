# Tab 1 GUI

## Scope / Purpose

- This document tracks the current desktop GUI for Tab 1 and the way it presents live venue comparison.

## Current Implemented System

- Tab 1 currently renders as a Tauri desktop GUI with a chart-first hero surface.
- The GUI keeps supporting text compact and uses the chart as the primary explanatory surface.
- The GUI uses centralised theme and dashboard CSS rather than component-scoped utility styling.
- The GUI currently includes a primary price card, venue lane panel, detail panel, and chart mode controls.

## Implemented Outputs / Artifacts

- `src/features/arbitrage/ArbitragePage.tsx` composes the current page structure and live refresh loop.
- `src/features/arbitrage/components/PriceCard.tsx` renders the primary readout.
- `src/features/arbitrage/components/MarketChart.tsx` renders the current chart modes and event markers.
- `src/styles/theme.css` and `src/styles/dashboard.css` provide the shared visual system.

## In Progress / Partially Implemented

- The GUI direction is now aligned with the agreed style, but the chart behaviour still needs refinement.
- The GUI is currently focused on Tab 1 only and does not yet establish broader cross-tab desktop navigation.
- The GUI still lacks persisted historical views and richer comparative panels.

## Planned / Missing / To Be Changed

- Continue refining chart readability and mode switching until the visuals feel operational rather than merely attractive.
- Add richer multi-panel views for history, events, and comparative summaries once persistence exists.
- Extend the same visual system to later tabs without turning each tab into a wholly separate product style.

## Notes / Design Considerations

- The GUI should remain dark, restrained, and precise rather than bubbly or overloaded with gradients.
- Charts should dominate the explanatory flow, while numbers remain available for exact inspection.
- Product UI should not render roadmap content, milestone labels, or internal implementation commentary.

## Discarded / Obsolete / No Longer Relevant

- The default Tauri starter UI is obsolete.
- The earlier roadmap-heavy GUI direction is obsolete.
