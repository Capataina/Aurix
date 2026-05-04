# Arbitrage GUI

## Scope / Purpose

- This document owns the current desktop presentation layer for the arbitrage feature: page composition, component boundaries, dashboard styling, and shell-level metadata that directly affects what the user sees.

## Boundaries / Ownership

- This file owns `src/App.tsx`, `src/features/arbitrage/ArbitragePage.tsx`, the presentational components in `src/features/arbitrage/components/`, and the CSS files in `src/styles/`.
- It also owns user-facing shell metadata still exposed through `index.html` and the Tauri window definition in `src-tauri/tauri.conf.json`.
- It does not own backend market acquisition or the rule logic that computes insights from session history, except where those outputs are rendered to the user.

| Area | Owned here | Excluded from this file |
| --- | --- | --- |
| Page composition | Screen layout, venue cards, detail panel, chart placement | Backend fetch orchestration |
| Presentational components | Price readout, chart rendering, insight/event rendering | Insight rule calculation internals |
| Visual system | Theme tokens, dashboard layout, class-based styling | Backend payload semantics |
| Shell metadata | Browser title, favicon reference, Tauri window title and size | RPC config and chain access |

## Current Implemented Reality

| Surface | Current implementation reality |
| --- | --- |
| App root | `src/App.tsx` is the multi-tab application root; routes between Tab 1 (Arbitrage) and Tab 2 (LP Backtest) via the `TopBar` shell. Tab 1 mounts `ArbitragePage`. |
| Page layout | `ArbitragePage.tsx` lays out a stacked-scrollable block grid (post-2026-05-03 restructure) instead of the older draggable grid. Block ordering is reorder-able by editing the registry. |
| Block grid | `src/components/blocks/arbitrage/` owns the per-block components: `PriceChartBlock` (540 lines, the main chart, restructured to TS rewrite + gradient + smooth + step marks), `ArbRouteBlock`, `BlockRegistry`, plus shared blocks in `src/components/blocks/shared/`. Folder layout: `blocks/{arbitrage,lp,shared}/` per the 2026-05-03 organisation commit (334e8ac). |
| Settings menu | `TopBar` carries a route-aware `SettingsMenu`: hero venue picker, buffer size, stale threshold, plus per-page overrides for the LP page. Shipped commit 611ee40. |
| Shell metadata | `index.html` references the proper Aurix metadata; the older "Tauri + React + Typescript" title and starter assets were updated as part of the multi-tab restructure. |
| Hero card | `PriceCard.tsx` shows the first returned venue as the primary market readout, plus gas price, timestamp, error banner, and manual refresh control |
| Chart modes | `PriceChartBlock` supports three modes: Raw (4-venue lines, no fill), Spread, Net P/L (fill when zero straddles the domain). Deviation mode was dropped in commit cd5f7e8 because it was visually identical to Raw — only the y-axis labels differed. |
| Insight surface | `InsightsPanel.tsx` renders the primary insight card, up to four secondary cards, and a recent-events list derived elsewhere |
| Visual system | `src/styles/theme.css` + per-component CSS in `src/styles/components/` (one file per major surface: `blocks.css`, `card.css`, `chart.css`, `lp-backtest.css`, etc.) implement a dark, dense monitoring layout using plain CSS — no component library, no utility framework. |

## Key Interfaces / Data Flow

| Surface | Inputs | Output to user |
| --- | --- | --- |
| `ArbitragePage` | Live `MarketOverview`, local `history`, UI state for chart mode and event toggle | Whole page composition and state orchestration |
| `PriceCard` | Primary `PriceSnapshot`, gas price, loading/error state | Hero-side live price readout and refresh affordance |
| `MarketChart` | Session history plus chart-mode state | SVG time-series visualisation and mode-specific metrics |
| `InsightsPanel` | `InsightsViewModel` | Textual interpretation and event feed |
| `theme.css` / `dashboard.css` | Global class names from the React tree | Layout, spacing, colour tokens, chart styling, responsive collapse |

- Rendering is prop-driven after the `ArbitragePage` boundary; lower-level components do not own data fetching.
- The current screen is deliberately chart-first and keeps copy compact, but most content is still hard-coded around one pair and one feature.

## Implemented Outputs / Artifacts

- `src/App.tsx` provides the single-screen root.
- `src/features/arbitrage/ArbitragePage.tsx` provides page orchestration and high-level layout.
- `src/features/arbitrage/components/PriceCard.tsx` provides the hero-side price readout.
- `src/features/arbitrage/components/MarketChart.tsx` provides the comparative chart surface.
- `src/features/arbitrage/components/InsightsPanel.tsx` provides the live interpretation surface.
- `src/styles/theme.css` and `src/styles/dashboard.css` provide the current visual system.
- `index.html` and `src-tauri/tauri.conf.json` provide browser-shell and desktop-window metadata.

## Known Issues / Active Risks

- Stale-copy drift around venue count lives in three places — `src/features/arbitrage/components/PriceCard.tsx:48` (`Three live Ethereum venue reads…`), `src/features/arbitrage/ArbitragePage.tsx:222` (fallback string `"3"` for active-venues detail row), and the backend rustdoc at `src-tauri/src/commands/market.rs:11`. All three should be updated together when this is addressed.
- The page chooses `overview.venues[0]` as the primary market without any explicit semantic contract beyond array order, so a backend ordering change would silently change the hero view (`ArbitragePage.tsx:86`).
- The static `VENUES` metadata in `ArbitragePage.tsx:10-35` and the `SERIES_META` table in `MarketChart.tsx:30-47` both key off `dexName` string equality. They are separate copies of the same venue-identity keyspace and must stay aligned with the backend labels or: (a) the venue card shows `$0.00` from the `?? 0` fallback in the price lookup, and (b) the chart crashes on `SERIES_META[venueName].accentClassName` because the lookup returns undefined.
- Error handling is whole-screen only because the backend cannot currently report per-venue health.
- Starter shell metadata remains visible: `index.html` still has title `Tauri + React + Typescript` and references `/vite.svg` as the favicon; `tauri.conf.json` uses lower-case `productName: "aurix"` and window title `aurix`; `Cargo.toml` has `description = "A Tauri App"` and `authors = ["you"]`.
- `PriceCard.tsx` falls back to `"Uniswap V3"` and `"WETH / USDC"` string literals when `snapshot` is null (`PriceCard.tsx:58,61`); if a future backend change renames the V3 5bps label, this fallback also drifts.

## Partial / In Progress

- The visual direction is coherent and responsive for one dashboard, but the broader product shell is still essentially scaffold-level.
- The screen already exposes several monitoring surfaces, but it is still a single-page monitor rather than a multi-feature local analytics desktop app.

## Planned / Missing / Likely Changes

- Replace starter browser-shell metadata and product naming once shell polish becomes part of active implementation rather than tolerated scaffolding.
- Add navigation or broader framing only when a second implemented feature exists; earlier shell abstraction would be premature.
- Move venue presentation metadata closer to the live data contract if venue count, labels, or states begin changing more often.
- Add source-health indicators and stale-state visuals once the backend can emit partial-success information.

## Durable Notes / Discarded Approaches

- The UI is intentionally read-only and interpretive; it is not designed as a trading terminal or wallet-connected execution surface.
- Centralising style in plain CSS keeps the current visual language coherent for a small app, but it also means many components depend on shared class contracts instead of isolated component-level styling.

## Obsolete / No Longer Relevant

- The default Tauri greeting UI is obsolete.
- Any assumption that the repository already contains a multi-tab or route-driven desktop shell is not current.
