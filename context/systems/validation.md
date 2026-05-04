# Validation

## Scope / Purpose

- Replays LP-position fixtures through the [backtest](backtest.md) engine and compares per-step fees / gas / position-value diffs against ground-truth values. Produces a `ValidationReport` rolling per-fixture pass/fail into a single test artefact.
- This is the "are we computing the right numbers?" harness — independent of the in-crate unit tests, which verify that individual primitives behave correctly. Validation answers a different question: do the *composed* engine outputs match what real-on-chain V3 positions earned?

## Boundaries / Ownership

- Owns: fixture loading, fixture-vs-engine diff computation, `ValidationReport` shape, the synthetic fixture generator (`validation/synthetic.rs`) used as a deterministic baseline before real-fixture data lands.
- Does **not** own: the engine itself (that's [backtest](backtest.md)), the V3 math primitives (that's [math](math.md)), persistence of validation results (currently in-memory only).

## Current Implemented Reality

```text
validation/
├── mod.rs              # ValidationReport + driver (170 lines)
├── error.rs            # ValidationError thiserror enum
└── synthetic.rs        # Synthetic fixture generator
```

**Synthetic fixtures.** Produces a deterministic per-block sinusoidal swap stream (anchored on a known tick) plus a known LP position; the engine replays it and the harness asserts that the resulting fees / gas / equity-curve final value match expected values within a tolerance. Test: `validation/mod.rs::tests::synthetic_fixtures_round_trip`.

**Real-on-chain fixtures (planned).** Per `references/v3-position-validation-methodology.md`, real validation requires:
1. A known mainnet LP position (mint tx → burn tx).
2. The full swap stream for that pool over the position's lifetime (from Uniswap subgraph or Alchemy).
3. The actual fees collected from the burn tx receipt (the ground truth).
4. The engine's computed fees over the same swap stream.

The diff between (3) and (4) is the validation pin. As of 2026-05-04, real-fixture data has not been ingested yet; the harness exists for synthetic fixtures only.

## Key Interfaces / Data Flow

| Boundary | Direction | Shape | Notes |
|---|---|---|---|
| validation harness → [backtest](backtest.md) | outbound (call) | `Engine::simulate(config, rule)` per fixture | uses the same engine as production |
| validation harness → fixture data | inbound | `(swap_stream, position_config, expected_outputs)` | synthetic today; real-fixture format TBD |
| validation harness → caller | outbound | `ValidationReport { fixture_count, passed, failed, diffs }` | not persisted to storage today |

## Implemented Outputs / Artifacts

- 1 test in `validation/mod.rs::tests::synthetic_fixtures_round_trip` — confirms the synthetic round-trip closes within tolerance.

## Known Issues / Active Risks

- **No real-on-chain fixtures yet.** The harness can validate synthetic fixtures but has not been exercised against real V3 LP positions. Until real-fixture diffs are run, the engine's correctness is supported by:
  - `math/*` unit tests (V3 primitive correctness),
  - `backtest/*` integration tests (composed engine behaviour on synthetic inputs),
  - the synthetic round-trip in this harness,
  but not by ground-truth fee numbers from real burn-tx receipts. Recorded as the highest-impact validation gap.
- **No persistence of validation results.** The `ValidationReport` is computed and returned but not written to [storage](storage.md). Future work could persist validation reports for regression tracking.

## Partial / In Progress

- Real-fixture infrastructure (subgraph query for known LP positions + burn-tx receipt parsing) — not started.

## Planned / Missing / Likely Changes

- Real-on-chain fixture set — at least 5 LP positions across different fee tiers and time windows.
- Persisted validation reports for regression detection.
- A `cargo test -- --ignored validation::real` test target gated on live RPC access (parallel to the existing `live_alchemy_*` ignored tests in [ingest](ingest.md)).

## Durable Notes / Discarded Approaches

- **Synthetic-first validation order.** Real fixtures require live RPC access + a curated set of mainnet LP positions; standing that up is its own project. Synthetic-first lets the harness exist before the real-fixture infrastructure lands, so the engine has *some* end-to-end validation surface from day one. Documented in the M2.4 plan in `context/plans/vector-a-v3-lp-backtester.md`.

## Obsolete / No Longer Relevant

- None.

## Cross-references

- Consumer of: [backtest](backtest.md) (drives `Engine::simulate`).
- Reference for design: `references/v3-position-validation-methodology.md`.
- Vector A milestone: M2.4 (validation harness — code-shipped, behavioural-acceptance pending real fixtures).
