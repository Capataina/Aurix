# Exercise Guide

The exercises layer is where domain theory meets implementation reality. Reading concept files builds your model; doing exercises proves you can use it.

## Exercise Types

| Type | What it teaches | File extension |
|---|---|---|
| **Conceptual** | Work through math by hand; build intuition before code | `.md` |
| **Code drill** | Implement an isolated piece of math against a known reference | `.rs` or `.ts` |
| **Extension** | Add a feature to existing Aurix code | `.rs` or `.ts` |
| **Design** | Design schemas, interfaces, or architectures without coding | `.md` |
| **Debugging** | Find and fix a bug in existing code (Aurix has Gap 4 et al. for this) | `.rs` or `.ts` |

## Where To Start

Pick by your current confidence:

| You're at | Start with |
|---|---|
| New to DeFi/AMMs | `foundations/amm-constant-product-by-hand.md` |
| Comfortable with theory, want code | `core/decode-sqrtpricex96.rs` |
| Want project-grounded practice | `project/extend-insight-engine.md` |
| Preparing for Vector A | `core/decode-sqrtpricex96.rs` then `core/simulate-v2-swap.rs` |
| Preparing for Vector C | `foundations/sandwich-attack-economics.md` then `project/design-persistence-schema.md` |

The recommended sequence is in `EXERCISE_ORDER.md`.

## How To Use Hints

Each exercise file's header has a `Hints:` section with 2-3 hints staged from light to strong. The intent:

- **Try the exercise without hints first.** Sit with the problem for 15-30 minutes.
- **If you're truly stuck, use hint 1.** It's a directional nudge.
- **Hint 2 if hint 1 isn't enough.** Stronger pointer to where to look.
- **Hint 3 only if hint 2 isn't enough.** Near-complete direction.

Using hints isn't failure — it's calibration. But using them too early skips the most valuable learning (the productive struggle).

## Solutions

Solutions live in `solutions/` and mirror the exercise folder structure. A solution for `core/decode-sqrtpricex96.rs` lives at `solutions/core/decode-sqrtpricex96.rs`.

**Use solutions to verify, not to learn.** Read the solution AFTER you've made a serious attempt. Reading it first means you're learning from the solution rather than from the exercise.

`solutions/SOLUTION_INDEX.md` lists what's available.

## Time Expectations

Most exercises target 30-90 minutes of focused work. If you're spending more than 2 hours on an exercise without progress, either:

1. The exercise is too large — break it into sub-steps
2. You're missing a prerequisite — check the `Related Files` section in the exercise header

Both are signals to back off and recalibrate, not to push through with frustration.

## Adding New Exercises

If you find a piece of Aurix material that you want to practice but don't see an exercise for, write one. The `templates.md` reference (in the upkeep-learning skill) has the exercise file format. New exercises should:

- Be solvable in 30-90 minutes
- Have a clear "expected behaviour" the learner can verify
- Not contain the answer in the scaffolding (no comments revealing the bug, no variable names that encode the solution)
- Cross-link back to the relevant concept file
