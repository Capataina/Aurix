# Solution Index

Solutions live in `learning/exercises/solutions/` and mirror the exercise folder structure. They are working implementations meant for verification AFTER you've made a serious attempt at the exercise — not for bypassing the practice.

## Foundations

The foundations exercises are conceptual (paper-and-pencil math). They don't have code solutions because there's nothing to compile — work the math, check your answers against the expected values in each exercise's "Expected Behaviour / Self-Check" section.

- `foundations/amm-constant-product-by-hand.md` — answers in the exercise file's hint section
- `foundations/impermanent-loss-worked-example.md` — answers in the exercise file's hint section
- `foundations/sandwich-attack-economics.md` — answers in the exercise file's hint section

## Core (Rust drills)

- `solutions/core/decode-sqrtpricex96.rs` — working V3 sqrtPriceX96 decoder
- `solutions/core/simulate-v2-swap.rs` — working V2 swap simulator with fee

Both solutions are minimal — they do the math correctly without over-explaining. The teaching happens in the exercise file and the concept files; the solution is just verification.

## Project Practice

The project exercises (`extend-insight-engine.md`, `design-persistence-schema.md`) are design exercises. They don't have a single "right" answer — they have a quality bar. Compare your design to:

- `context/plans/vector-a-v3-lp-backtester.md` M2.0 for the persistence schema
- `src/features/arbitrage/insights.ts` for the insight engine extension (existing rules as a benchmark)

If your design satisfies the "Expected Behaviour" section in each exercise, you're done — there's no canonical solution to compare to.

## How To Use

1. Make a serious attempt at the exercise without consulting the solution
2. Use the hint sections (light → strong) to unstick yourself
3. Once you have a working answer, run it (for code drills) or check against the expected values (for conceptual)
4. THEN open the solution to compare approaches
5. Note any differences — sometimes the solution's approach is more elegant; sometimes yours is. Both are learnings.

Do NOT open the solution before attempting the exercise. The solution becomes worthless as a teaching tool the moment you read it first.
