# DEX Name Contract

## Current Understanding

The `dex_name` string on each `PriceSnapshot` is the implicit identity key used across the GUI. Two frontend tables key off exact-string equality:

| Table | File | Purpose |
| --- | --- | --- |
| `VENUES[]` | `src/features/arbitrage/ArbitragePage.tsx:10-35` | Venue card metadata (accent class, state label, summary copy); price binding uses `overview.venues.find(v => v.dexName === exchange.name)` |
| `SERIES_META{}` | `src/features/arbitrage/components/MarketChart.tsx:30-47` | Per-venue chart line colour, legend swatch class |

The four current values produced by the backend:

- `"Uniswap V3 5bps"` (`dex/uniswap_v3.rs:28`)
- `"Uniswap V3 30bps"` (`dex/uniswap_v3.rs:41`)
- `"Uniswap V2"` (`dex/uniswap_v2.rs:22`)
- `"SushiSwap"` (`dex/uniswap_v2.rs:36`)

## Guiding Principles

- Renaming any `dex_name` in the backend **requires** matching updates to both `VENUES` and `SERIES_META` in the frontend. The compiler will not catch the drift — string lookups return `undefined`.
- When the `SERIES_META` lookup returns undefined, `MarketChart.tsx` accesses `.accentClassName` on it and crashes the chart render. When the `VENUES` lookup price binding fails, the venue card shows `$0.00` from the `?? 0` fallback. Those two failure modes look very different — if only one shows up after a rename, the other hasn't been hit yet, not fixed.
- Adding a new venue requires three coordinated edits: a new adapter call in `commands/market.rs`, a new `VENUES` entry, a new `SERIES_META` entry (with a new accent CSS class if a new colour is needed).
- Do not introduce a fifth table of venue metadata. If a new surface needs venue labels/colours, it should import from `VENUES` or `SERIES_META` rather than define a third copy.

## Constraint

This is a fragile implicit contract. The durable fix is to move venue metadata into a single shared module (e.g. `src/features/arbitrage/venues.ts`) and import it from both sites. That change has not been made yet — it is listed in `systems/arbitrage-gui.md` Planned / Missing / Likely Changes as "move venue presentation metadata closer to the live data contract."
