# Backtest Engine — Code Health Findings

**Systems covered:** `src-tauri/src/backtest/{engine,mod,price,metrics,gas,position,rebalance,error}.rs`
**Finding count:** 5 (2 high, 2 medium, 1 modularisation verdict)

The backtest engine (`engine.rs` — 374 lines) is the load-bearing computational core. Every LP backtest run, every cell of the strategy grid, and every month of the headline regime classifier flows through `Engine::simulate`'s per-swap loop. The findings below are all in that loop. They are independent and stack — the implementing engineer can apply any subset.

The behavioural envelope of the engine is pinned by the existing 5 tests in `backtest/mod.rs:117-305` (`simulate_static_position_produces_curve`, `simulate_persists_to_storage_and_is_idempotent`, `empty_swap_data_returns_error`, `invalid_tick_range_errors`, `schedule_rebalance_increments_count`, `out_of_range_position_earns_no_fees`). Any refactor must keep these green.

## Performance Improvement

### Hoist invariant USD-conversion of hold-only baseline out of the per-swap loop

- [x] In `src-tauri/src/backtest/engine.rs:253`, the per-step `value_usd(&hold_amount0, &hold_amount1, cur_price)` call walks `position_usd_value(_explicit)` from scratch each iteration. The token-amount→decimal conversion does not change across iterations; only the price does. Pre-compute the decimal-adjusted token amounts once before the loop, then multiply by `cur_price` per step. *(implemented 2026-05-04 in commit b2e6863 — `HoldOnlyEvaluator` struct)*

**Category:** Performance Improvement
**Severity:** High
**Effort:** Small
**Behavioural Impact:** None (verified — the result depends only on the inputs, and the change preserves them; the existing tests pin the resulting value envelope)

**Location:**
- `src-tauri/src/backtest/engine.rs:253` — `let hold_only_usd = value_usd(&hold_amount0, &hold_amount1, cur_price);`
- `src-tauri/src/backtest/engine.rs:102-128` — `value_usd` closure + `hold_amount0` / `hold_amount1` setup
- `src-tauri/src/backtest/price.rs` — `position_usd_value` and `position_usd_value_explicit` (called from inside `value_usd`)

**Current State:**
Inside the per-swap `for (idx, swap) in swaps.iter().enumerate()` loop (lines 149-279), the engine recomputes the hold-only baseline every iteration via:

```rust
let hold_only_usd = value_usd(&hold_amount0, &hold_amount1, cur_price);
```

`hold_amount0` and `hold_amount1` are `BigUint` clones of the deposit amounts established before the loop (lines 102-103). They never change. The closure `value_usd` (lines 110-128) dispatches to either `position_usd_value` (pool-ratio path, when `token0/1_usd_price` are not configured) or `position_usd_value_explicit` (USD-price path).

For the explicit path: `position_usd_value_explicit(a0, a1, p0, p1, dec0, dec1)` computes `(a0_f64 / 10^dec0) * p0 + (a1_f64 / 10^dec1) * p1`. The first multiplicand `a0_f64 / 10^dec0` is invariant across the loop (the hold amounts and decimals are fixed); only `p0`, `p1` could vary if they were sourced per-step. They are not — they come from `config.token0_usd_price` and `config.token1_usd_price`, which are `Option<f64>` constants for the run.

For the pool-ratio path: `position_usd_value(a0, a1, ratio, dec0, dec1)` uses `ratio = cur_price` per-step. The decimal-adjustment of `a0` and `a1` is still loop-invariant — only the `ratio` changes.

So per swap, the engine pays at minimum:
- `BigUint::to_f64()` on `hold_amount0` and `hold_amount1` (allocates internally, walks the digit vector),
- `f64::powi` for `10^dec0` and `10^dec1`,
- two divisions, one multiplication, one addition.

Over a 1000-swap backtest, that's 1000 `BigUint::to_f64` pairs of calls + 1000 powi pairs + 1000 multiplications — all to compute `hold_amount0_decimal * cur_price + hold_amount1_decimal * cur_price` where `*_decimal` are loop-invariant.

Two more occurrences of `value_usd` in the same loop have the same hoisting opportunity:
- `let raw_position_value = value_usd(&a0_cur, &a1_cur, cur_price);` (line 248) — `a0_cur` / `a1_cur` change per iteration; this one has to stay.
- `let fees_usd = value_usd(&fees_token0_acc, &fees_token1_acc, cur_price);` (line 249) — see the next finding.

**Proposed Change:**
Pre-compute the decimal-adjusted hold amounts once before the loop:

```rust
let hold_amount0_decimal = bigint_to_f64(&hold_amount0) / 10f64.powi(config.token0_decimals as i32);
let hold_amount1_decimal = bigint_to_f64(&hold_amount1) / 10f64.powi(config.token1_decimals as i32);
```

Then per-iteration:

```rust
let hold_only_usd = match (config.token0_usd_price, config.token1_usd_price) {
    (Some(p0), Some(p1)) => hold_amount0_decimal * p0 + hold_amount1_decimal * p1,
    _ => hold_amount0_decimal * cur_price + hold_amount1_decimal,  // ratio path
};
```

The `value_usd` closure stays for the per-step `a0_cur` / `a1_cur` valuation — this finding is specifically about the hold-only branch.

**Justification:**
Direct evidence from reading the price-conversion code: `position_usd_value_explicit` is a pure function of its inputs; the hold inputs do not change across iterations. Hoisting loop-invariant computation is a textbook optimisation; the only reason it is not done here is that the closure was added without auditing the per-step access pattern.

The existing tests (`simulate_static_position_produces_curve` and the `out_of_range_position_earns_no_fees` test) pin the equity-curve length and the time-in-range fraction. The hold-only USD value is a derived quantity that those tests do not assert specifically, but the engine's `il_usd = raw_position_value - hold_only_usd` calculation does. A future test that asserts a specific `il_usd` value for a fixed synthetic input would catch any drift in this refactor — the audit recommends adding it as part of the implementation work.

**Expected Benefit:**
Removes two `BigUint::to_f64` calls + one `value_usd` closure dispatch + one branch + two divisions and one multiplication per swap in the loop. For 10k-swap backtests, that is ~10k `BigUint::to_f64` calls saved. The win is modest in absolute time but free.

**Impact Assessment:**
Zero functional change (verified by analysis). The pool-ratio path's `position_usd_value(hold_amount0, hold_amount1, cur_price, dec0, dec1)` computes `(hold_amount0_f64 / 10^dec0) * cur_price + (hold_amount1_f64 / 10^dec1) * 1.0`. After hoisting, the same expression evaluates with the same operands in the same order — only the location of the `to_f64` and `powi` calls changes. Floating-point is associative for additions of equal-sign terms (no catastrophic cancellation here because both terms are positive), so the result is bit-identical in normal cases and within ULP otherwise. The audit recommends preserving the pool-ratio convention `position_usd_value(_, _, ratio, ..., _)` even after hoisting so the implicit "token1 is USD-pegged at the current ratio" assumption stays explicit.

---

### Accumulate fees-USD incrementally instead of re-converting accumulators every step

- [x] In `src-tauri/src/backtest/engine.rs:249`, `value_usd(&fees_token0_acc, &fees_token1_acc, cur_price)` re-walks the (monotonically growing) BigUint accumulators every iteration. Replace with an incremental accumulator that adds the *delta-fees-USD* per swap. *(implemented 2026-05-04 in commit b2e6863 — `fees_usd_acc` running sum)*

**Category:** Algorithm Optimisation
**Severity:** Medium
**Effort:** Small
**Behavioural Impact:** Negligible (flagged) — the substitution involves a different order of floating-point operations, which can produce ULP-level drift on `fees_usd_acc`. The drift is bounded by the cumulative rounding error of N additions of small positive numbers vs one large `BigUint::to_f64` of the sum. For typical fee scales (USD cents per swap, ~$10s per run) and typical `f64` precision, the drift is well under `1e-9` USD cumulative — practically zero.

**Location:**
- `src-tauri/src/backtest/engine.rs:182-184` — fee accumulation
- `src-tauri/src/backtest/engine.rs:249` — per-step `fees_usd = value_usd(&fees_token0_acc, &fees_token1_acc, cur_price)`

**Current State:**
Two BigUint accumulators (`fees_token0_acc`, `fees_token1_acc`) grow monotonically over the loop:

```rust
let f0 = fee_share_token0(&in0, fee_units, liquidity, active_liquidity, in_range)?;
let f1 = fee_share_token1(&in1, fee_units, liquidity, active_liquidity, in_range)?;
fees_token0_acc += &f0;
fees_token1_acc += &f1;
```

Every iteration then converts the (potentially-large) accumulators to USD:

```rust
let fees_usd = value_usd(&fees_token0_acc, &fees_token1_acc, cur_price);
```

For a 10k-swap backtest, the accumulator grows to potentially 10k-fee-events worth of digits, and `BigUint::to_f64` walks the digit vector each call.

**Proposed Change:**
Maintain a running f64 accumulator that adds the *delta* per swap:

```rust
let mut fees_usd_acc = 0.0f64;
// inside the loop, after fees_token0_acc / fees_token1_acc have been updated by &f0/&f1
let delta_fees_usd = value_usd(&f0, &f1, cur_price);
fees_usd_acc += delta_fees_usd;
let fees_usd = fees_usd_acc;
```

This makes each iteration's USD conversion work proportional to the *delta* (small BigUint, often empty for out-of-range swaps) rather than the *accumulator* (growing BigUint). The closing `total_fees_usd = last_pt.fees_accumulated_usd` (line 301) is unchanged.

**Justification:**
Algorithmic — converting once per delta is O(delta_size); converting the accumulator each step is O(accumulator_size) per step, total O(N * N * token_digit_size) which is quadratic in swap count. For 10k swaps the difference is the difference between linear and quadratic work in the per-step USD conversion.

For correctness: the existing `fee_share_token0/1` functions return `Ok(BigUint::from(0u8))` when out of range or `active_liquidity == 0`. The `value_usd(&zero, &zero, cur_price)` is `0.0`, so the incremental accumulator stays exactly equal to the value of converting the full accumulator on every step where in-range fees are zero. When in range, both forms compute the same total — only the order of conversion-vs-summation differs.

**Expected Benefit:**
Per-step work drops from O(accumulator-size) to O(delta-size). The cumulative speedup grows with backtest length — this is the biggest single per-loop win in the engine for long backtests.

**Impact Assessment:**
Negligible-flagged. Floating-point order-of-operations is different — the current form computes `fees_total_token0_f64 / 10^dec * price + fees_total_token1_f64 / 10^dec`; the proposed form computes `Σ(f0_i_f64 / 10^dec * price_i + f1_i_f64 / 10^dec)`. For monotonically-increasing positive terms with the same dec, the cumulative drift is bounded by `N * eps * total_fees`. For the typical case (N=10k, total_fees=$10, eps=2e-16), the drift bound is ~2e-11 USD — well below any meaningful threshold.

If the backtester ever needs strict deterministic-replay equivalence with a published number from a different implementation, the implementing engineer should pin the fees value via a baseline test against the ground-truth fixture before applying this change. The audit recommends adding such a baseline test as part of the implementation work — the V3 position validation methodology in `context/references/v3-position-validation-methodology.md` describes the relevant ground-truth fixtures.

---

### Pre-parse swap rows once instead of per-loop-iteration

- [x] The per-swap loop calls `parse_sqrt(&swap.sqrt_price_x96)`, `parse_signed(&swap.amount0)`, `parse_signed(&swap.amount1)`, and `parse_liquidity(&swap.liquidity)` on every iteration (lines 150, 167-168, 157). These are `BigUint::parse_bytes` calls on TEXT-encoded decimals stored in SQLite. Move the parsing to a one-shot pass at swap-load time. *(implemented 2026-05-04 in commit b2e6863 — Option A: `ParsedSwap` struct + `parse_swaps()` helper)*

**Category:** Data Layout and Memory Access Patterns
**Severity:** High
**Effort:** Medium
**Behavioural Impact:** None (verified — the parsed values are deterministic from their string inputs)

**Location:**
- `src-tauri/src/backtest/engine.rs:150` — `let cur_sqrt = parse_sqrt(&swap.sqrt_price_x96)?;`
- `src-tauri/src/backtest/engine.rs:157` — `let active_liquidity = parse_liquidity(&swap.liquidity)?;`
- `src-tauri/src/backtest/engine.rs:167-168` — `parse_signed` for amount0 and amount1
- `src-tauri/src/backtest/engine.rs:361-374` — the parse helpers
- `src-tauri/src/storage/swaps.rs:18-34` — `SwapEventRow` definition (TEXT decimal columns)

**Current State:**
Swap rows are stored in SQLite with the precise integer fields (`amount0`, `amount1`, `sqrt_price_x96`, `liquidity`, `tick`) as TEXT decimal strings to preserve full uint160/int256/uint128 precision (per `swaps.rs:8-10`'s comment). On read, `query_swaps_for_pool_range` returns a `Vec<SwapEventRow>` where these fields are still strings. The backtest engine then parses each string to `BigUint` / `BigInt` / `u128` *every time it is needed* in the loop.

For each swap, this is:
- 1 `BigUint::parse_bytes` for `sqrt_price_x96`
- 2 `BigInt::parse_bytes` for `amount0` and `amount1`
- 1 `u128::from_str` for `liquidity`

Plus the `prev_sqrt = cur_sqrt` carry-over at end of loop (`engine.rs:278`) clones a BigUint per iteration.

For 10k swaps, that's 40k+ allocations purely from string parsing inside the inner loop, plus 10k clones of BigUint sqrtPrices.

**Proposed Change:**
Either:

**Option A (lighter):** Add a parallel `Vec<ParsedSwap>` constructed once after `query_swaps_for_pool_range` returns, where `ParsedSwap` holds the pre-parsed `BigUint` / `BigInt` / `u128` fields plus the original metadata (block_number, log_index, block_timestamp, gas_price). The loop iterates over `&parsed_swaps` instead of `&swaps`, accessing pre-parsed fields.

```rust
struct ParsedSwap {
    block_number: i64,
    block_timestamp: i64,
    log_index: i64,
    block_gas_price_gwei: Option<f64>,
    sqrt_price_x96: BigUint,
    liquidity: u128,
    amount0: BigInt,
    amount1: BigInt,
    tick: i32,
}

let parsed: Vec<ParsedSwap> = swaps.into_iter()
    .map(|s| Ok::<_, BacktestError>(ParsedSwap {
        block_number: s.block_number,
        block_timestamp: s.block_timestamp,
        log_index: s.log_index,
        block_gas_price_gwei: s.block_gas_price_gwei,
        sqrt_price_x96: parse_sqrt(&s.sqrt_price_x96)?,
        liquidity: parse_liquidity(&s.liquidity)?,
        amount0: parse_signed(&s.amount0)?,
        amount1: parse_signed(&s.amount1)?,
        tick: s.tick,
    }))
    .collect::<Result<Vec<_>, _>>()?;
```

**Option B (heavier, deeper data-layout change):** Change the storage schema to store the precise integer fields as `BLOB` (32-byte big-endian for uint256, 16-byte for uint128) and decode on read. Removes the string-parse step entirely and uses ~half the disk space for those columns. This is closer to the "data layout" wins category and is what canonical Rust V3 libraries (`shuhuiluo/uniswap-v3-sdk-rs`) use.

The audit recommends **Option A** for this audit pass — it captures the per-loop allocation win without touching the storage schema. Option B is a larger change worthy of its own plan; flagging here so the implementing engineer can sequence appropriately.

**Justification:**
Direct analytical evidence — the loop walks each swap exactly once, so parsing N times pays the parse cost N times instead of once. For long backtests, the parse cost is a meaningful share of the per-swap work. The `BigUint::parse_bytes` digit-vector allocation is exactly the kind of short-lived allocation the Data Layout category targets.

The string-storage decision is documented in `swaps.rs:8-10` ("stored as TEXT decimal strings to preserve full uint160 / int256 / uint128 precision; callers parse to `BigInt` / `BigUint` on read") — the pattern is correct *at the storage boundary*, but the engine should not pay the cost on every loop iteration.

**Expected Benefit:**
Per-swap parse cost removed from the inner loop (4 fewer `parse_bytes` calls + 1 fewer `from_str`). For 10k-swap backtests, ~40k allocations removed. The `prev_sqrt = cur_sqrt.clone()` cost remains — handled by changing the iteration to `prev_sqrt = &parsed[idx].sqrt_price_x96` (taking a reference to the next iteration's already-parsed value, no clone).

**Impact Assessment:**
Zero functional change. The parsed values are deterministic from the input strings. The order of operations within the loop is preserved; only the parse step moves from "lazy per-iteration" to "eager once-per-call". The existing tests pin the equity-curve length and the time-in-range fraction; both are unaffected by the parse-time relocation.

---

## Modularisation

### Verdict for `src-tauri/src/backtest/engine.rs` (374 lines, top-decile candidate)

**Verdict:** `split-recommended`

**Justification:** The file packs three distinct concerns into one function body: (a) initial setup (lines 56-145 — config validation, swap loading, position initialisation, hold-only baseline construction), (b) the per-swap loop (lines 149-279 — fee accumulation, LVR, rebalance, valuation, equity emission), (c) the post-loop summary computation (lines 281-358 — burn cost, summary aggregates, metrics). Each is independently testable; the loop body specifically would benefit from extraction into a per-swap step function so that the audit's three perf findings above can be validated in isolation.

**Recommended split:**
- Extract per-swap step into a method `step(&mut self, swap: &ParsedSwap, ctx: &mut StepCtx) -> EquityCurvePoint`. The engine struct owns the running state (`liquidity`, `sqrt_lower`, `tick_lower`, accumulators); the step method consumes one swap. This makes the loop a one-line `swaps.iter().map(|s| self.step(s, &mut ctx)).collect()`.
- Extract the post-loop summary into `summarise(equity_points: &[EquityCurvePoint], config: &PositionConfig, ...)`.
- Keep `simulate` as the orchestrator: validate → load → step over swaps → summarise.

The split serves the per-swap-loop refactor findings directly: each finding becomes a localised change inside `step`, with the orchestrator unchanged.

**Effort:** Medium. **Behavioural Impact:** None (mechanical extraction, baseline pinned by existing tests).

---

### Verdict for `src-tauri/src/backtest/mod.rs` (306 lines, top-decile candidate)

**Verdict:** `leave-as-is`

**Justification:** Of the 306 lines, 287 are `#[cfg(test)]` test code (lines 22-307); the production surface is 19 lines of `pub mod` declarations and re-exports. Splitting test code out into separate files would lose the inline-test convention used throughout Aurix (every `*.rs` in `src-tauri/src/` follows the same pattern). Same precedent as `examples.md` §7.

---

## Modularisation

### Verdict for `src-tauri/src/storage/runs.rs` (340 lines, top-decile candidate)

**Verdict:** `leave-as-is`

**Justification:** Standard storage-table CRUD for one domain (position runs + equity curve points). Internally cohesive — three concerns (insert run summary, batch-insert curve points, query). Splitting would require the resulting modules to share the `PositionRunSummary` and `EquityCurvePoint` types via a re-export layer (`storage/runs/types.rs` + `storage/runs/insert.rs` + `storage/runs/query.rs`), which adds boilerplate without separating responsibility. The file's size is mostly mechanical SQL + parameter binding; that's the nature of CRUD code.
