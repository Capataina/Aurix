# Exercise Order

Recommended progression. Tick checkboxes as you complete exercises; the file is preserved across upkeep passes.

## Foundations

Conceptual work — paper and pencil, no code. Builds intuition before you start writing Rust.

- [ ] `foundations/amm-constant-product-by-hand.md` — work through V2's `x * y = k` with concrete numbers
- [ ] `foundations/impermanent-loss-worked-example.md` — track an LP position through a price swing
- [ ] `foundations/sandwich-attack-economics.md` — compute SEV for a hypothetical victim swap

## Core (Rust drills)

Implementation drills against the actual math. Code files; you write Rust.

- [ ] `core/decode-sqrtpricex96.rs` — decode a known V3 sqrtPriceX96 to a USD price
- [ ] `core/simulate-v2-swap.rs` — apply `x * y = k` math to a swap and compute slippage

## Project Practice

Practice that touches Aurix's actual code or extends its design.

- [ ] `project/extend-insight-engine.md` — design (don't code) a new severity rule for `insights.ts`
- [ ] `project/design-persistence-schema.md` — design the SQLite schema for Vector A's M2.0 (foundation for all five tabs)

## Suggested Jumps

- After `foundations/amm-constant-product-by-hand.md`, read `concepts/core/amm-mechanics-v2-and-v3.md` again. Things land harder the second time.
- Before `core/decode-sqrtpricex96.rs`, read `concepts/advanced/uniswap-v3-tick-mathematics.md`. The sqrtPriceX96 encoding is non-obvious.
- After `project/design-persistence-schema.md`, read `context/plans/vector-a-v3-lp-backtester.md` and compare your schema to M2.0's specification.

## Vector-Specific Sequences

If you're prepping for Vector A:
- [ ] `foundations/amm-constant-product-by-hand.md`
- [ ] `foundations/impermanent-loss-worked-example.md`
- [ ] `core/decode-sqrtpricex96.rs`
- [ ] `core/simulate-v2-swap.rs`
- [ ] `project/design-persistence-schema.md`

If you're prepping for Vector B:
- [ ] `foundations/sandwich-attack-economics.md`
- [ ] `project/design-persistence-schema.md` (Vector B can share the schema)

If you're prepping for Vector C:
- [ ] `foundations/sandwich-attack-economics.md` (helps with feature ideation)
- [ ] `project/design-persistence-schema.md`
- [ ] `project/extend-insight-engine.md` (the ML predictions will integrate here)

## Notes

- **Exercise time** ≠ **calendar time.** A 90-minute exercise might span 2-3 sessions if you walk away to think.
- **Solutions exist** in `solutions/` for the code drills. Use them to verify, not to bypass.
- **No exercises for the advanced concepts yet** — V3 tick math, MEV mechanics, ML — because the right exercises for those are the actual vector implementations themselves. Trying to drill them in isolation tends to be too abstract.
