# Math

## Scope / Purpose

- Aurix's clean-room port of Uniswap V3's mathematics stack — `FullMath.sol`, `TickMath.sol`, `LiquidityAmounts.sol`, `SqrtPriceMath.sol` — built on `num-bigint::BigUint` rather than a fixed-width type. The deliberate design choice ("no `ethers-rs`") puts the V3 math under direct ownership for resume + audit clarity.
- Provides Q64.96 fixed-point arithmetic, tick ↔ sqrtPriceX96 conversion, liquidity ↔ amounts derivation, per-swap fee distribution, and impermanent-loss closed-form computations.

## Boundaries / Ownership

- Owns: every primitive operation on `sqrtPriceX96`, every tick-math conversion, every liquidity-vs-amounts conversion, the protocol-units convention for fee tiers, the IL formula.
- Does **not** own: the *use* of these primitives. [backtest](backtest.md) calls into math; [ingest](ingest.md)'s decoder produces the values; math itself is pure functions of `BigUint`/`BigInt`/`u128`/`i32`.
- Fan-in: 12 (math is the second-most-imported subsystem after storage). High structural importance.

## Current Implemented Reality

```text
math/
├── mod.rs              # re-exports + thin facade
├── error.rs            # V3MathError thiserror enum
├── q96.rs              # Lazy<BigUint> constants + mul_div / mul_div_round_up (FullMath.sol port)
├── tick.rs             # tick_to_sqrt_price_x96 / sqrt_price_x96_to_tick (TickMath.sol port)
├── liquidity.rs        # liquidity_for_amounts / amounts_for_liquidity / per-step deltas (LiquidityAmounts.sol + SqrtPriceMath.sol port)
├── fees.rs             # fee_share_token0 / fee_share_token1 / extract_in_amounts / bps_to_protocol_units
└── il.rs               # V2 + V3-concentrated impermanent-loss closed forms
```

**Q64.96 precision.** All sqrt-price arithmetic happens in `BigUint` Q64.96. The 2^96 normalisation factor + helper constants (`Q128`, `Q160`, `U256_MAX`, `MIN_SQRT_RATIO`, `MAX_SQRT_RATIO`) live as `Lazy<BigUint>` statics in `q96.rs` so they allocate once per process. `mul_div` is a direct port of Solidity `FullMath.mulDiv` — multiply at full precision, then divide.

**Tick math.** `tick_to_sqrt_price_x96` is a bit-by-bit port of `getSqrtRatioAtTick` using the 20 magic constants from `TickMath.sol` (Q128.128 representations of `1.0001^(2^k)` for `k = 0..19`). Currently parses each magic constant from a hex string per call — see [audit finding](../plans/code-health-audit/math.md) §"Pre-compute the 20 tick-magic constants once" for the recommended `Lazy<[BigUint; 20]>` precompute.

`sqrt_price_x96_to_tick` is the inverse. Implementation: f64 log-estimate followed by ±2-tick refinement using the exact `tick_to_sqrt_price_x96` to land on the unique tick satisfying `sqrt(t) ≤ sqrtPriceX96 < sqrt(t+1)`. The f64-then-refine pattern is correct and well-tested but is **not** a port of Solidity's exact bit-walking inverse — it is a faster approximation specific to this Rust implementation. See `references/v3-mathematics-deep-dive.md` for the algorithm comparison.

**Liquidity ↔ amounts.** The three V3 cases (current below range / inside range / above range) are implemented in `liquidity_for_amounts` exactly per the whitepaper. Per-step deltas (`amount0_delta`, `amount1_delta`) take a `round_up: bool` flag mirroring `SqrtPriceMath`'s per-direction rounding policy.

**Fee distribution.** `fee_share_token0/1` computes `swap_amount × fee_tier_bps / 1_000_000 × position_L / active_L` for in-range positions; out-of-range positions receive zero. The protocol uses fee tier in "hundredths-of-bps" (5bps = 500 in storage), and `bps_to_protocol_units` converts the friendlier UI unit. A defensive clamp `position_L ≤ active_L` is enforced (added in commit 391eadd's correctness fixes — out-of-band synthetic data calibration could produce position-share > 1 without the clamp).

**Impermanent loss.** `il.rs` provides:
- `il_v2(price_ratio: f64) -> f64` — closed-form V2 IL, `2 * sqrt(r) / (1+r) - 1`
- `il_concentrated_*` — V3-concentrated forms parameterised by tick range

## Key Interfaces / Data Flow

| Function | Inputs | Outputs | Errors |
|---|---|---|---|
| `mul_div(a, b, denom)` | `&BigUint × 3` | `BigUint` | `DivisionByZero` |
| `mul_div_round_up(a, b, denom)` | `&BigUint × 3` | `BigUint` | `DivisionByZero` |
| `tick_to_sqrt_price_x96(tick: i32)` | tick in `[MIN_TICK, MAX_TICK]` | `BigUint` (sqrtPriceX96 fits in uint160) | `TickOutOfBounds(tick)` |
| `sqrt_price_x96_to_tick(&BigUint)` | sqrtPriceX96 in `[MIN_SQRT_RATIO, MAX_SQRT_RATIO)` | `i32` tick | `SqrtRatioOutOfBounds` |
| `liquidity_for_amounts(sqrt_current, lo, hi, a0, a1)` | bounds + amounts | `u128` liquidity | `LiquidityOverflow` |
| `amounts_for_liquidity(sqrt_current, lo, hi, L)` | bounds + liquidity | `(BigUint, BigUint)` amounts | — |
| `amount0_delta(sa, sb, L, round_up)` | per-step | `BigUint` | — |
| `amount1_delta(sa, sb, L, round_up)` | per-step | `BigUint` | — |
| `fee_share_token0/1(amount, fee_tier, posL, activeL, in_range)` | swap amounts + position | `BigUint` fee in raw units | `DivisionByZero` |

**MIN_TICK / MAX_TICK** are `±887_272` (the protocol bounds). All magic constants are bit-exact transcriptions of the Solidity source.

## Implemented Outputs / Artifacts

- 30+ unit tests across `q96.rs`, `tick.rs`, `liquidity.rs`, `fees.rs`, `il.rs` — round-trip pins (tick → sqrtPrice → tick recovers original tick), bit-exact matches against Solidity reference values, V3-concentrated IL invariants, fee-share monotonicity, et al.

## Known Issues / Active Risks

- **Per-call magic-constant allocation in `tick_to_sqrt_price_x96`.** Each tick decode allocates up to 20 `BigUint` instances by re-parsing hex literals. Recorded as a high-severity perf finding in [audit findings](../plans/code-health-audit/math.md). Downstream: every persisted swap event during ingestion + every rebalance during backtest replay pays this cost.
- **f64 LVR precision.** `backtest::engine` casts sqrtPriceX96 to f64 for LVR computation; for sqrtPrice values close to MAX_SQRT_RATIO (rare on real ETH/USDC pools but possible on extreme price moves) the f64 mantissa loses precision. Recorded as a [potential issue](../plans/code-health-audit/potential-issues.md) §4. Math primitives themselves are unaffected — the precision loss is at the consumer boundary, not in math.

## Partial / In Progress

- None — math is code-complete and well-tested.

## Planned / Missing / Likely Changes

- The `Lazy<[BigUint; 20]>` precompute per the [audit finding](../plans/code-health-audit/math.md).
- Optional move from `num-bigint::BigUint` to `ruint::aliases::U256` (fixed-width, stack-allocated) — substantial refactor across all callers; not in scope for the current audit, recorded as a future research candidate.

## Durable Notes / Discarded Approaches

- **`num-bigint::BigUint` was chosen over `ruint::U256` for resume narrative.** The "no ethers-rs, clean-room port" narrative is stronger when the underlying types are general-purpose Rust primitives rather than crypto-domain-specific ones. `ruint` would be faster but the port-from-Solidity story is less direct.
- **f64 + iterative refinement was chosen over Solidity's bit-walking inverse for `sqrt_price_x96_to_tick`.** The Solidity inverse is bit-precise but slow in Rust BigUint arithmetic; the f64 estimate gets within 2 ticks, refinement closes the gap. The choice is documented in `references/v3-mathematics-deep-dive.md`.

## Obsolete / No Longer Relevant

- None.

## Cross-references

- Consumers: [backtest](backtest.md) (every swap step calls `tick_to_sqrt_price_x96` + `liquidity_for_amounts` + `fee_share_*`), [validation](validation.md) (replays through the same primitives), [strategies](strategies.md) (via backtest), [headline](headline.md) (via backtest), `commands::lp::lp_query_first_swap_price` (calls `sqrt_price_x96_to_tick`).
- Related research: `references/v3-mathematics-deep-dive.md`, `references/v3-position-validation-methodology.md`.
- Related notes: `notes/round-trip-fee-math.md` (fee math semantics).
