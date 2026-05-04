# Math Primitives — Code Health Findings

**Systems covered:** `src-tauri/src/math/{q96,tick,liquidity,fees,il,error}.rs` (1,095 lines, 7 files)
**Finding count:** 3 (1 high, 1 medium, 1 low)

The Aurix V3 math stack is a deliberate clean-room port of `FullMath.sol` + `TickMath.sol` + `LiquidityAmounts.sol` + `SqrtPriceMath.sol`, built on `num-bigint::BigUint` rather than a fixed-width type. Bit-for-bit correctness is verified by 30+ unit tests. The findings below are perf-class wins on the inverse-tick decode path that runs once per swap during ingestion and once per swap during backtest replay — i.e. on the dominant inner loop.

## Performance Improvement

### Pre-compute the 20 tick-magic constants once

- [x] Convert `MAGIC: [&str; 20]` plus the per-call `magic(bit) -> BigUint` parser in `src-tauri/src/math/tick.rs:21-49` into a `Lazy<[BigUint; 20]>` static so the BigUint allocations happen exactly once for the process lifetime instead of on every tick decode. *(implemented 2026-05-04 in commit b2e6863)*

**Category:** Performance Improvement
**Severity:** High
**Effort:** Trivial
**Behavioural Impact:** None (verified — the resulting BigUint values are the same hex literals; equivalence is by construction of `BigUint::parse_bytes` being deterministic on the same input)

**Location:**
- `src-tauri/src/math/tick.rs:21-49` — `MAGIC` array + `magic()` helper
- `src-tauri/src/math/tick.rs:67-79` — call sites inside `tick_to_sqrt_price_x96`

**Current State:**
The 20 Q128.128 magic constants for `getSqrtRatioAtTick` are stored as hex string literals in a `const MAGIC: [&str; 20]` array. Every call to `tick_to_sqrt_price_x96(tick)` invokes `magic(k)` for each set bit of `abs_tick`, and `magic()` calls `BigUint::parse_bytes` on the hex literal each time:

```rust
fn magic(bit: usize) -> BigUint {
    let lit = MAGIC[bit];
    let stripped = lit.trim_start_matches("0x");
    BigUint::parse_bytes(stripped.as_bytes(), 16)
        .expect("magic constant literal is valid hex (transcribed from TickMath.sol)")
}
```

For an `abs_tick` close to MAX_TICK, all 20 bits can be set — so up to 20 `BigUint::parse_bytes` calls happen per tick decode. `tick_to_sqrt_price_x96` is called from:
- `ingest::pipeline` once per swap event during backfill (per `backtest/engine.rs:231-232` for rebalance re-centring),
- `backtest::engine::Engine::simulate` per rebalance event,
- `backtest::engine::Engine::simulate` once at entry (line 76-77 via `parse_sqrt`),
- `commands::lp::lp_query_first_swap_price` per query.

For a 10,000-swap backtest with 100 rebalances, that's at least 200 tick decodes × up to 20 magic-parse calls = up to 4,000 transient BigUint allocations purely from magic-constant parsing. Each `BigUint::parse_bytes` allocates the digit vector freshly.

**Proposed Change:**
Replace the `const MAGIC: [&str; 20]` plus `magic()` helper with a `Lazy<[BigUint; 20]>`:

```rust
static MAGIC: Lazy<[BigUint; 20]> = Lazy::new(|| {
    const HEX: [&str; 20] = [
        "fffcb933bd6fad37aa2d162d1a594001",
        // ... 19 more lines, identical hex strings without "0x" prefix
    ];
    std::array::from_fn(|i| {
        BigUint::parse_bytes(HEX[i].as_bytes(), 16)
            .expect("magic constant is valid hex transcribed from TickMath.sol")
    })
});
```

Inside `tick_to_sqrt_price_x96`, replace `magic(k)` with `&MAGIC[k]` (note: also adjust the multiplication to `&ratio * &MAGIC[k]` since `BigUint` does not implement `Mul<BigUint>` for `&BigUint` in all positions). The arithmetic is unchanged; only the source of the BigUint changes from per-call alloc to once-per-process alloc.

The `once_cell::sync::Lazy` infrastructure is already in use — see `q96.rs:11-16` for the existing `Q96`, `Q128`, `Q160`, `Q192`, `U256_MAX`, `MIN_SQRT_RATIO`, `MAX_SQRT_RATIO` static definitions. The pattern is established.

**Justification:**
Direct evidence from research (the `once_cell` and `lazy_static` documentation) and from the existing precedent in `q96.rs`. The values are deterministic from their hex input, so the per-call → once-per-process change is mathematically equivalent. Per the num-bigint maintainer guidance surfaced during research, BigUint values that don't change are exactly the case for `Lazy<BigUint>`.

The existing `magic()` helper has a doc comment `"magic constant literal is valid hex (transcribed from TickMath.sol)"` — the constants are by definition immutable and shared. Computing them once is the correct lifetime.

**Expected Benefit:**
Eliminates up to 20 `BigUint::parse_bytes` calls per `tick_to_sqrt_price_x96` invocation. On a typical backtest run (thousands of swaps, hundreds of rebalances), this is hundreds to thousands of transient BigUint allocations removed from the inner loop. The win compounds on the ingest path (every persisted swap stores `tick`, but the inverse direction `sqrt_price_x96_to_tick` calls `tick_to_sqrt_price_x96` 5 times in its refinement loop — line 134-141).

A fair benchmark of the precomputed vs per-call versions is left to the implementing engineer, but the analytical claim — "no BigUint::parse_bytes per call" — is the load-bearing benefit.

**Impact Assessment:**
Zero functional change. The hex constants are unchanged; the resulting BigUint values are unchanged; the arithmetic chain is unchanged. The only observable difference is allocation rate. Verified by inspection: `BigUint::parse_bytes` is a pure function of its input, and the input (hex string) does not change between calls.

The existing test suite (`tick_zero_is_q96`, `tick_min_matches_min_sqrt_ratio`, `tick_max_matches_max_sqrt_ratio`, `tick_one_increases_above_q96`, `round_trip_at_zero`, `round_trip_at_min_returns_min_tick`, `round_trip_at_various_ticks`, `invariant_sqrt_at_tick_le_sqrt_at_next` — all in `tick.rs:172-265`) provides the baseline pin. Any drift is a regression these tests will catch.

---

## Algorithm Optimisation

### Simplify the redundant double-condition on dropped bits

- [x] Replace the redundant `if !dropped.bits().eq(&0) { if dropped > BigUint::from(0u8) { ... } }` block in `src-tauri/src/math/tick.rs:87-94` with a single `if !dropped.is_zero() { sqrt_price_x96 += 1u8; }`. *(implemented 2026-05-04 in commit b2e6863)*

**Category:** Algorithm Optimisation
**Severity:** Low
**Effort:** Trivial
**Behavioural Impact:** None (verified — the inner condition is structurally identical to the outer condition; both fire when `dropped > 0`, never independently)

**Location:**
- `src-tauri/src/math/tick.rs:87-94` — final round-up step in `tick_to_sqrt_price_x96`

**Current State:**
The function ends with:

```rust
let dropped: BigUint = &ratio % (BigUint::from(1u8) << 32);
let mut sqrt_price_x96 = ratio >> 32;
if !dropped.bits().eq(&0) {
    // any non-zero dropped bits → round up
    if dropped > BigUint::from(0u8) {
        sqrt_price_x96 += 1u8;
    }
}
```

The outer condition `!dropped.bits().eq(&0)` is true when `dropped` has any bits set, i.e. `dropped > 0`. The inner condition `dropped > BigUint::from(0u8)` is true when `dropped > 0`. **The two conditions are identical.** The inner block fires only inside the outer block, so the inner check is always-true dead code.

Additionally:
- `dropped.bits().eq(&0)` allocates nothing but is awkward (`u64`-eq-`&u64` via type inference). The `BigUint::is_zero()` trait (`num_traits::Zero`) — already in use elsewhere in the file (`prev_sqrt.is_zero()` in `engine.rs:189`) — is the idiomatic check.
- `BigUint::from(0u8)` allocates a fresh BigUint for the comparison, which is wasted work.

**Proposed Change:**
Replace the seven lines with:

```rust
if !dropped.is_zero() {
    sqrt_price_x96 += 1u8;
}
```

This requires `use num_traits::Zero;` (already imported at the top of `q96.rs` for the same trait). The function output is identical for all inputs.

**Justification:**
Analytical evidence — by inspection, the two conditions are tautologically the same. The simpler form removes one BigUint allocation per call (the `BigUint::from(0u8)` comparand) and eliminates the cognitive load of "why are there two checks here?".

**Expected Benefit:**
Five lines → two lines, one fewer BigUint allocation per `tick_to_sqrt_price_x96` call, idiomatic use of `is_zero()` matching the rest of the codebase.

**Impact Assessment:**
Zero functional change (verified by inspection). The two conditions evaluate to the same boolean for every BigUint value; the inner check is always satisfied when reached. Existing round-trip tests provide the baseline.

---

## Modularisation

### Verdict for `src-tauri/src/math/liquidity.rs` (322 lines, top-decile candidate)

**Verdict:** `leave-as-is`

**Justification:** Of the 322 lines, ~140 are inside the `#[cfg(test)]` block (lines 182-322). The non-test surface is ~140 lines of focused liquidity↔amounts conversion logic plus per-direction step deltas — each function (`liquidity_for_amounts`, `liquidity_from_amount0`, `liquidity_from_amount1`, `amounts_for_liquidity`, `amount0_delta`, `amount1_delta`, `bigint_to_u128`) maps directly to a named V3 reference function in `LiquidityAmounts.sol` or `SqrtPriceMath.sol`. Splitting would scatter the V3 reference correspondence across files and require either re-importing in lockstep or introducing a re-export layer that adds maintenance burden. The file is *long* because it carries its full V3-reference test surface, not because it owns multiple concerns.

This matches the test-heavy file precedent in `examples.md` §7 (`leave-as-is` for `src/db/embeddings.rs`).
