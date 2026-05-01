# Exercise: Extend the Insight Engine

## Goal

Design a NEW severity rule for `src/features/arbitrage/insights.ts` that surfaces a meaningful pattern. Don't write the code yet — design it carefully first, including the trigger condition, the severity tier, the card text, and what the user should do with it.

This is a design exercise. The code is straightforward once the design is clear; the design is the hard part.

## Estimated Time

45-60 minutes.

## Setup

Read `src/features/arbitrage/insights.ts` and `learning/project/systems/insight-engine-anatomy.md`. Understand:
- The existing severity levels (info, watch, notable, actionable)
- The existing windows (SHORT_WINDOW=5, BASELINE_WINDOW=20, PERSISTENCE_WINDOW=4)
- The existing primary insight types ("Positive setup holding", "Positive setup emerging", "Elevated spread regime")
- The existing secondary cards (venue order, spread regime, deviation leader, gas-adjusted view)

## Tasks

### Part 1 — Pick a pattern worth surfacing

Brainstorm 5 patterns the existing engine doesn't surface but could. Examples to get you started (don't pick exactly these — invent your own):

- "Sushi has been chronically below baseline for the last X minutes — V2 venues might be in a regime shift"
- "Gas just spiked above the 95th percentile of the last hour — actionable arb thresholds just doubled"
- "The cheapest venue has flipped 5 times in the last 10 samples — high venue churn, not a stable arb regime"

Write down 5 candidates. Pick ONE to design in detail.

### Part 2 — Design the rule

For your chosen pattern, specify:

- [ ] **Trigger condition**: a precise predicate over `derivedHistory` that returns true when the rule fires
- [ ] **Required windows**: do you need to look at the last 5 samples? 20? 50? Something else?
- [ ] **Severity tier**: info, watch, notable, or actionable — and why
- [ ] **Where it lives**: primary card (one of, must compete with existing primary candidates) or secondary (ranks among the four)
- [ ] **Card text**: the exact title, body text, and metric. Match the existing engine's voice — terse, descriptive, no recommendations.

### Part 3 — Edge cases

- [ ] What happens when history is too short? (The existing engine has a "Signal maturity" caution for this — does your rule need similar treatment?)
- [ ] What happens when ALL venues report the same price? (Your rule shouldn't crash on degenerate data)
- [ ] What happens when the pattern is true for 1 sample then false? (Should the card disappear instantly or have hysteresis?)

### Part 4 — Compare to existing rules

- [ ] Does your rule overlap with an existing one? If so, how do they differ?
- [ ] Does your rule make any existing rule redundant? (If yes, propose removing the redundant one)
- [ ] Where does your rule rank in priority vs the existing ones? (For primary cards, only ONE fires per tick)

### Part 5 — Write the prose-only version of the new code

In a separate file (or a long comment), write what the implementation would look like in pseudocode:

```typescript
// In deriveInsightsView, add:
const myPatternRunLength = trailingRunLength(
  derivedHistory,
  (sample) => /* your predicate */,
);

// In buildPrimaryInsight or in the secondary array, add:
if (myPatternRunLength >= MY_THRESHOLD) {
  return {
    id: "my-pattern",
    title: "...",
    severity: "...",
    metric: ...,
    body: "...",
  };
}
```

Don't actually write the code — just the structure.

## Hints

### Hint 1

Good rules surface patterns the user wouldn't notice from raw data. The existing rules already cover:
- "Things look profitable" (positive setups)
- "Things look unusual" (elevated spread regimes)
- "Venue ordering"
- "Specific deviations"

Less-covered territory:
- Cross-venue health (one venue degrading)
- Time-of-day effects
- Gas regime shifts
- Volatility regime shifts
- Repeated-pattern detection (this same pattern keeps appearing)

### Hint 2

The trickiest design decision is severity. The existing pattern:
- `info` = state description with no urgency
- `watch` = something to keep an eye on
- `notable` = pattern worth thinking about
- `actionable` = positive gas-adjusted route persisting

Does YOUR rule fit naturally into one of these, or does it need a new tier? If new tier, justify why.

### Hint 3

A genuinely good rule example: "Cheapest-venue churn detector"

- **Trigger**: count distinct cheapest-venue values across the last 10 samples; if ≥ 3, fire
- **Window**: 10 samples (looks at last 10 seconds)
- **Severity**: `notable` — high venue churn means no stable arb direction, the user should NOT trust persistence-based actionable insights as much
- **Where**: secondary card, ranks above "Venue order" when firing
- **Text**: 
  - Title: "High venue churn"
  - Body: "Cheapest venue has flipped {N} times in the last 10 samples — no stable cross-venue regime, persistence-based actionables should be treated with caution."
  - Metric: "{N} flips / 10s"
- **Edge**: requires history length ≥ 10; otherwise card doesn't render

This rule adds VALUE because it changes how the user should interpret the existing actionable insights when high churn is present. That's the test for a good rule — does it change behaviour, or just describe state?

## Expected Behaviour / Self-Check

A good design has:
- A precise, falsifiable trigger condition (no vague "when things are weird")
- A clear severity justification (you can explain why you picked watch vs notable)
- Real value-add (you can explain what the user does differently with this signal vs without)
- No conflict with existing rules (you've checked overlap)
- Edge cases handled (short history, degenerate data, oscillating signals)

If your rule is "the spread is bigger than usual" — that's already covered by the elevated spread regime. Pick something more interesting.

## What You Should Take Away

- The insight engine is rule-based by design; adding rules is the natural extension path
- Severity tiers map to UI styling; misuse them and the visual hierarchy breaks
- Good rules add information that changes user behaviour, not just description
- Designing rules well requires thinking about edge cases and rule interactions

## Relation to Vector C

Vector C augments (not replaces) the rule-based engine. The ML model adds calibrated probabilities to the existing rules. A "high venue churn" rule like the one in Hint 3 could become a model FEATURE — "high recent churn is a signal that this opportunity is less likely to persist" — which feeds the survival predictor.

Designing rules well is also designing features well. The skill transfers.

## Related Files

- `project/systems/insight-engine-anatomy.md` — anatomy of the existing engine
- `src/features/arbitrage/insights.ts` — the actual code
- `concepts/core/arbitrage-and-cross-venue-equilibrium.md` — domain theory
- `context/plans/vector-c-ml-arbitrage-survival.md` — where this leads
