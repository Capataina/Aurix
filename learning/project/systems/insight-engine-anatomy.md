# Insight Engine Anatomy

## What This System Does

The insight engine (`src/features/arbitrage/insights.ts`, ~430 lines) is the rule-based interpretation layer that transforms raw market history into user-facing observations. Given a 100-sample rolling history of `MarketOverview`s, it produces a structured `InsightsViewModel` with a primary insight card, up to four secondary cards, and up to four recent events.

This file walks through the engine's logic so you can extend it (Vector C builds on it) and debug it.

## Where It Fits

The insight engine sits between the polling loop (which produces raw market data) and the React rendering layer (which displays interpreted insights). It runs synchronously in the React render pipeline:

```
fetchMarketOverview() → setHistory() → deriveInsightsView(history) → React renders
```

It's stateless across calls — every render computes everything from scratch from the current history. This is fine because the history is bounded at 100 samples (`HISTORY_LIMIT`) and the computation is sub-millisecond.

## Key Mechanics

### Constants

```typescript
const GAS_UNITS_ESTIMATE = 220_000;
const SHORT_WINDOW = 5;
const BASELINE_WINDOW = 20;
const PERSISTENCE_WINDOW = 4;
const EVENT_LIMIT = 4;
```

These four windows govern the engine's behaviour:
- **SHORT_WINDOW = 5** — used for "recent" comparisons within the latest 5 samples
- **BASELINE_WINDOW = 20** — the rolling baseline for comparing "current vs typical recent"
- **PERSISTENCE_WINDOW = 4** — the threshold for upgrading severity (a pattern persisting for 4+ samples gets escalated)
- **EVENT_LIMIT = 4** — maximum number of events surfaced

### The Sample Derivation

Every sample (one `MarketOverview`) is enriched into a `DerivedSample`:

```typescript
interface DerivedSample {
    overview: MarketOverview;
    medianPrice: number;
    spreadUsd: number;
    gasCostUsd: number;
    gasAdjustedUsd: number;
    richestVenue: PriceSnapshot;
    cheapestVenue: PriceSnapshot;
    strongestPositiveDeviation: { venue, deviationPct };
    strongestNegativeDeviation: { venue, deviationPct };
}
```

For each tick:
- `medianPrice` = median of the 4 venue prices
- `spreadUsd` = max venue price - min venue price
- `gasCostUsd` = `(gasPriceGwei × 220,000 × medianPrice) / 10^9`
- `gasAdjustedUsd` = `spreadUsd - gasCostUsd`
- `richestVenue` / `cheapestVenue` = the venues at the extremes
- Deviations = each venue's % difference from the median, separated by sign

This enrichment is per-tick. The full history is then a list of `DerivedSample`s.

### The Trailing Run Length

The engine's central trick is `trailingRunLength`:

```typescript
function trailingRunLength<T>(samples: T[], predicate: (sample: T) => boolean): number {
    let count = 0;
    for (let i = samples.length - 1; i >= 0; i--) {
        if (predicate(samples[i])) {
            count++;
        } else {
            break;
        }
    }
    return count;
}
```

It walks backward from the most-recent sample, counting how many consecutive samples satisfy the predicate. The first non-matching sample stops the count.

This is used to detect persistence:
- `positiveRunLength` = how many consecutive recent samples have positive gas-adjusted spread
- `richestRunLength` = how many consecutive recent samples have the same richest venue
- `cheapestRunLength` = same for cheapest
- `elevatedSpreadRunLength` = how many consecutive recent samples have spread ≥ 1.15× baseline

When any run length exceeds `PERSISTENCE_WINDOW = 4`, the corresponding insight is upgraded in severity.

### The Severity Levels

```typescript
export type InsightSeverity = "info" | "watch" | "notable" | "actionable";
```

- `info` — default; not flagging anything
- `watch` — worth keeping an eye on
- `notable` — pattern emerging
- `actionable` — would be theoretically profitable IF you ignore execution costs beyond gas

These map to UI styling — `actionable` cards get the most visual emphasis.

### The Primary Insight

The primary insight card is the engine's headline. The selection logic (in `buildPrimaryInsight`):

```typescript
if (positiveRunLength >= PERSISTENCE_WINDOW) {
    return "Positive setup holding";  // actionable
}
if (latest.gasAdjustedUsd > 0) {
    return "Positive setup emerging";  // watch
}
if (elevatedSpreadRunLength >= PERSISTENCE_WINDOW) {
    return ...;  // notable - elevated regime
}
// fallback - info
```

So:
- **"Positive setup holding"** fires when the gas-adjusted spread has been positive for at least 4 consecutive samples. This is the strongest signal.
- **"Positive setup emerging"** fires when the current sample alone has positive gas-adjusted spread but it hasn't persisted yet.
- **"Elevated spread regime"** fires when the spread has been elevated for 4+ samples but not necessarily positive after gas.
- The fallback is info-level commentary on the current state.

### The Secondary Insights

Four secondary cards always render (unless history is too short):

1. **Venue order** — current richest and cheapest, framed as a spread comparison
2. **Spread regime** — current spread vs baseline, with regime indicator
3. **Deviation leader** — which venue is most above the median, which is most below
4. **Gas-adjusted view** — explicit framing of what gas leaves on the table

When history < SHORT_WINDOW, a fifth card "Signal maturity" is added warning that the data is shallow.

### The Event Stream

Up to 4 recent events. Events are state transitions worth flagging:
- A spread crossing into "elevated" territory
- A change in the richest or cheapest venue
- A gas-adjusted spread crossing zero

Each event has a timestamp, severity, and human-readable summary.

## Why It's Designed This Way

The engine is **deterministic and rule-based**. Three reasons:

1. **Interpretability**: every insight card's logic is reading-grade. You can debug "why did this card fire?" by tracing back to specific predicates.

2. **No training data needed**: Aurix has no persistence, so there's no historical dataset to train on. Rule-based logic works from first principles.

3. **Scoped to current capabilities**: Aurix has 100 samples (~100 seconds) of context. ML on 100-sample windows isn't viable. Rule-based is the right tool for this scale.

When Vector C ships, the engine will be augmented (not replaced) with calibrated ML predictions. The rule-based layer becomes the explanation layer; the ML layer becomes the prediction layer. They coexist.

## Known Issues (from `context/systems/arbitrage-analytics.md`)

The engine has documented gaps:

- **Analytical primitive duplication** (Gap 4): `median`, `formatUsd`, `GAS_UNITS_ESTIMATE`, gas-adjusted formula are all duplicated across 3-4 files. `formatUsd` has already drifted (insights.ts uses `signDisplay: "exceptZero"`, others use default).

- **Fixed gas units estimate** (Gap 7): 220,000 is undocumented; should be configurable per venue.

- **Session-only history** (Gap 1): baseline windows reset on restart. With persistence, the engine could use much longer baselines for more robust regime detection.

- **No tests** (Gap 6): the run-length logic, baseline comparisons, and event detection have zero test coverage. Refactoring is unsafety-net.

## How To Extend (For Vector C)

Vector C augments the engine with predictions. The integration points:

1. **Per-tick feature extraction**: a Rust collector writes per-tick snapshots to SQLite. Features are derived from the snapshot + recent history.

2. **Prediction layer**: an ONNX model loaded in Rust outputs per-opportunity confidence scores. The Rust backend exposes a new IPC command (`get_predictions_for_tick`) returning predictions for the current state.

3. **Insight engine consumption**: the engine receives predictions as additional input; surfaces them in the existing card structure with a new "Confidence" field.

4. **New severity threshold**: an "actionable" card with confidence > 0.70 might be tagged "high-confidence actionable"; below that, it's "actionable but uncertain."

The rule-based layer doesn't go away. It explains the WHAT; the ML layer estimates the HOW LIKELY.

## Common Misunderstandings (For Code Readers)

❌ **"The 'actionable' label means the trade is profitable."** It means the gross spread minus assumed gas exceeds zero AND has persisted for 4+ samples. It does not account for slippage, MEV, or competition. Most "actionable" insights are still net-negative for actual execution.

❌ **"The 100-sample window is the analytical baseline."** The 20-sample BASELINE_WINDOW is the comparison baseline. The 100-sample HISTORY_LIMIT is the maximum stored history (mostly displayed in the chart). They serve different purposes.

❌ **"Persistence = 4 samples = 4 seconds."** True at 1 Hz polling. If polling rate ever changed, "4 samples" would no longer mean "4 seconds" — but it would still mean "4 consecutive ticks." The thresholds are tick-based, not time-based.

❌ **"The engine could be ported to Rust easily."** It could, but it's intentionally JS — the engine consumes the same rolling history that's used for chart rendering. Keeping it in TypeScript means the React render pipeline does one pass over history (not two) and avoids cross-runtime serialisation cost per render.

## Related Files

- `project/systems/what-aurix-observes.md` — what these insights actually mean
- `project/architecture/the-1hz-loadsnapshot-tick.md` — what feeds the engine
- `concepts/core/arbitrage-and-cross-venue-equilibrium.md` — the domain theory
- `context/systems/arbitrage-analytics.md` — implementation truth + known issues
- `context/plans/vector-c-ml-arbitrage-survival.md` — the ML extension plan
