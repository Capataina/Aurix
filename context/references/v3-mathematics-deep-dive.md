<!-- This is the universal V3 math reference for Aurix Tab 2 (LP backtester).
     The audience is a future implementation agent who will write Rust
     primitives in M2.2 and the per-swap simulation engine in M2.3 without
     doing additional research. Every formula here should be transcribable
     to Rust without further interpretation; every load-bearing claim is
     anchored to a quoted passage from a primary source listed in the
     External Research Trail at the bottom. -->

# Uniswap V3 Concentrated Liquidity Mathematics — Deep Dive

## Scope / Purpose

This paper is the durable reference an Aurix implementation agent reads once before writing the V3 math primitives in `src-tauri/src/dex/v3_math/` (M2.2) and the per-swap simulation engine (M2.3). It is intentionally universal — it describes the protocol, not Aurix's existing decode — because the future agent needs the math, not the codebase tour.

**It covers:**

- the Q64.96 fixed-point format used pervasively in V3, with bit layout, overflow boundaries, and rounding semantics
- the exact tick ↔ `sqrtPriceX96` conversion (the `getSqrtRatioAtTick` magic-constant algorithm and its inverse)
- the closed-form derivation of `getLiquidityForAmounts`, `getAmountsForLiquidity`, and the related `getAmount0Delta` / `getAmount1Delta` swap-step primitives
- the `feeGrowthInside` / `feeGrowthOutside` accounting model, walked through a concrete swap
- the difference between per-swap fee distribution and per-block aggregation, and exactly where the approximation breaks
- the closed-form impermanent-loss expression for a concentrated position, including the out-of-range asymptotes
- tick spacing / fee tier constraints, and the `liquidityGross` / `liquidityNet` / `activeLiquidity` distinction
- a catalogue of bugs and rounding pitfalls observed in OSS V3 math libraries, with concrete citations

**It does not cover:**

- the `eth_getLogs` ingestion path, reorg handling, or the SQLite schema (those are M2.0 / M2.1)
- the strategy grid, benchmark module, or headline analysis (M2.5 / M2.7 / M2.8)
- L2 V3 deployments (custom fee tiers on Base, Arbitrum, etc) — the math is identical, only the Factory mapping differs
- V4's hook architecture or singleton-pool model — V4 is a separate research target if Aurix ever migrates
- numerically reproducing the protocol's rounding-up / rounding-down direction at every step (the swap primitives in `SqrtPriceMath.sol` flip rounding direction depending on swap direction; the implementer must read that contract directly when writing the inner swap loop, not paraphrase from this paper).

## Current Project Relevance

The Aurix LP backtester (Tab 2) cannot ship without correct V3 math. Three load-bearing claims in the active plan depend on this paper:

| Plan claim (`context/plans/vector-a-v3-lp-backtester.md`) | What this paper supplies |
|---|---|
| M2.2 wants `tick_to_sqrt_price_x96`, `sqrt_price_x96_to_tick`, `liquidity_for_amounts`, `amounts_for_liquidity`, "exact, per V3 whitepaper §6.2.2" | Section 2 + Section 3 — formulas in code-ready form, quoted Solidity reference, worked numerical examples |
| M2.3 wants per-swap fee distribution, "the key differentiator" vs OSS approximations | Section 4 + Section 5 — the per-step `feeGrowthGlobal` accumulation loop, the four specific points where block-level aggregation diverges, with worked dollar-impact estimates |
| Validation harness (M2.4) needs to flag "Here's a bug I found in [popular library X]" as a hiring talking point | Section 9 — bugs catalogued with issue/PR links from `uniswap-python`, Code4rena, and `v3-core` issues |

The "no `ethers-rs`, roll our own Q64.96 from `num-bigint`" constraint is already in the plan ("Open Decisions"). This paper assumes that constraint — every formula is presented in a form transcribable to `BigUint` arithmetic with explicit overflow bounds, not in a form that assumes a `U160` / `U256` type from `alloy-primitives`.

## Current State Snapshot

`repository fact` — the only V3 math currently in the codebase is `src-tauri/src/dex/uniswap_v3.rs`, which:

- decodes a `slot0()` return word into `BigUint` (`decode_sqrt_price_x96`, lines 52–63)
- converts `sqrtPriceX96` to a base-in-quote `f64` price using `BigUint` arithmetic for the 192-bit shift, then converts to `f64` only at the final ratio (`derive_price_base_in_quote`, lines 76–125)
- has no tick math, no liquidity math, no fee accounting, no swap simulation, no rounding-direction control

`repository fact` — the math stack chosen at this point is `num-bigint::BigUint` + `num-traits::ToPrimitive` + `hex` + `thiserror`. There is no `U256` type wrapper, no fixed-point library, no test harness.

`repository fact` — `context/references/` is empty as of this writing; this paper is the first reference artefact in the corpus.

`project inference` — when M2.2 begins, the implementer will need a signed integer type for ticks (`i32` is sufficient; the protocol uses `int24` but Rust has no native 24-bit integer), a wrapper type for `sqrtPriceX96` (likely a thin newtype over `BigUint` or over `[u8; 20]` if cast to `U160` semantics is needed), and a way to test for overflow into the high 256-bit space when computing `sqrtPriceX96^2` × decimal-scaling. Section 1 below specifies these bounds.

## What This Paper Actually Is

A self-contained reference for the math primitives in V3. The protocol uses three intertwined mathematical objects — the **tick grid** (a discrete logarithmic price index), the **`sqrtPriceX96` accumulator** (a continuous Q64.96 fixed-point square-root price), and **liquidity `L`** (the unitless quantity that links token amounts to price ranges). All three are needed to express any single LP operation. The rest of this paper builds them in dependency order:

```text
                ┌──────────────────────────────────────┐
                │ Section 1: Q64.96 fixed-point        │ ← bit layout, overflow, rounding
                └────────────┬─────────────────────────┘
                             │
                ┌────────────▼─────────────────────────┐
                │ Section 2: tick ↔ sqrtPriceX96       │ ← getSqrtRatioAtTick + inverse
                └────────────┬─────────────────────────┘
                             │
                ┌────────────▼─────────────────────────┐
                │ Section 3: liquidity ↔ amounts       │ ← getLiquidityForAmounts +
                │            (and swap-step amounts)   │   getAmount0Delta / getAmount1Delta
                └────────────┬─────────────────────────┘
                             │
                ┌────────────▼─────────────────────────┐
                │ Section 4: feeGrowthGlobal /         │ ← per-step accumulation,
                │            feeGrowthOutside /        │   tick-cross flip,
                │            feeGrowthInside           │   position settlement
                └────────────┬─────────────────────────┘
                             │
                ┌────────────▼─────────────────────────┐
                │ Section 5: per-swap vs per-block     │ ← where the approximation breaks,
                │            (Aurix differentiator)    │   numerical examples
                └────────────┬─────────────────────────┘
                             │
                ┌────────────▼─────────────────────────┐
                │ Section 6: impermanent loss          │ ← closed form, V3 vs V2,
                │                                      │   asymptotes
                └────────────┬─────────────────────────┘
                             │
                ┌────────────▼─────────────────────────┐
                │ Section 7: tick spacing / fee tier   │ ← validity constraints
                └────────────┬─────────────────────────┘
                             │
                ┌────────────▼─────────────────────────┐
                │ Section 8: activeLiquidity /         │ ← three-tier accounting,
                │            liquidityNet /            │   why all three are tracked
                │            liquidityGross            │
                └────────────┬─────────────────────────┘
                             │
                ┌────────────▼─────────────────────────┐
                │ Section 9: implementation pitfalls   │ ← bugs in OSS libraries,
                │                                      │   Code4rena findings
                └──────────────────────────────────────┘
```

---

## Section 1 — Q64.96 Fixed-Point Arithmetic

### 1.1 What Q64.96 actually is

`source-backed finding` — RareSkills [P-Q96-1] explains the rationale concisely:

> "Solidity doesn't have a float or decimal type" and "working with fixed-point numbers in binary is gas-efficient."

So the protocol stores prices as integers and treats those integers as having an *implicit binary point* 96 bits from the right. The `X96` suffix in `sqrtPriceX96` is shorthand for "the integer value × 2⁻⁹⁶ is the real number being represented."

Concretely, if `R` is the real-number square-root price (e.g. `R ≈ 57.469` for ETH at ~3302 DAI/ETH), the on-chain integer is:

```
sqrtPriceX96 = floor(R · 2^96)
```

and the inverse is:

```
R = sqrtPriceX96 / 2^96     (interpreting the divide as a real-number operation)
```

`source-backed finding` — RareSkills [P-Q96-1] gives this explicitly:

> ```
> sqrtPriceX96 = floor(√p × 2^96)
> ```
> ```
> √p = sqrtPriceX96 / 2^96
> p = (√p)²
> ```

### 1.2 The bit layout

`source-backed finding` — RareSkills [P-Q96-2]:

> "[V3] pack[s] the square root of the price together with the tick and other information in a single 256-bit storage slot, leaving 160 bits for sqrtPriceX96."

So the on-chain type is `uint160`, and the layout of a `uint160` Q64.96 number is:

```text
bit index   159          96 95              0
            ┌───────────────┬───────────────┐
            │ integer part  │  fractional   │
            │   (64 bits)   │  (96 bits)    │
            └───────────────┴───────────────┘
              high                       low
```

The integer part can hold values 0 to 2⁶⁴ − 1 ≈ 1.8 × 10¹⁹. Since this is the integer part of the *square root* of price, the represented price `p = R²` can range up to ~2¹²⁸ ≈ 3.4 × 10³⁸. RareSkills [P-Q96-3]:

> "The largest square root of a price the protocol can work with is approximately 2^64, and the corresponding largest price is slightly below 2^128."

The minimum representable square-root price is 2⁻⁶⁴ (matching by symmetry), so the minimum price is 2⁻¹²⁸. RareSkills [P-Q96-4]:

> "The smallest square root of the price...is imposed to be 2^-64."

The smallest representable fractional resolution is 2⁻⁹⁶ (RareSkills [P-Q96-5]: "a fixed-point value can represent fractions as small as 2⁻⁹⁶"). Anything smaller is silently truncated to zero.

### 1.3 Overflow boundaries that matter for Aurix

When you transcribe these to Rust, the bound that bites is **`sqrtPriceX96²` (192 bits)** and **the further multiplication by `10^(decimal_scale)` (extra ~60 bits)**. Concretely:

| Quantity | Max bit width | Notes |
|---|---|---|
| `sqrtPriceX96` (uint160) | 160 | Solidity type; native ceiling for representable values |
| `sqrtPriceX96²` | up to ~320 | Always exceeds 256-bit; need 320-bit arithmetic or `BigUint` |
| `sqrtPriceX96² · 10^(d0-d1)` | up to ~380 | For 18-decimal/6-decimal pairs (WETH/USDC), `10^12` adds ~40 bits |
| `liquidity` (uint128) | 128 | Pool state stores active L as uint128 |
| `liquidity · sqrtPrice_diff / Q96` | up to ~256 | Boundary of Solidity's native uint256 |
| `MIN_SQRT_RATIO` | 33 | `4_295_128_739` |
| `MAX_SQRT_RATIO` | 161 | `1_461_446_703_485_210_103_287_273_052_203_988_822_378_723_970_342` |

`source-backed finding` — TickMath.sol [P-TM-1] gives the absolute boundaries:

> ```solidity
> uint160 internal constant MIN_SQRT_RATIO = 4295128739;
> uint160 internal constant MAX_SQRT_RATIO = 1461446703485210103287273052203988822378723970342;
> ```

`project inference` — for Aurix's `BigUint`-based stack, the practical recommendations are:

1. Keep `sqrtPriceX96` as `BigUint` (already done in `dex/uniswap_v3.rs`).
2. Keep `liquidity` as `u128` natively (it fits, and it's the protocol's chosen type).
3. For any product like `sqrtPrice_a · sqrtPrice_b`, do the multiplication in `BigUint` and only narrow to `u128` / `u256` at the end.
4. **Always** treat division as `mulDiv` (multiply-then-divide with full intermediate precision); never narrow before dividing. The Solidity contracts use `FullMath.mulDiv` everywhere precisely because Solidity's native `uint256` division silently truncates.

### 1.4 Rounding semantics — read this twice

This is where most off-chain reproductions diverge from on-chain reality.

`source-backed finding` — TickMath.sol [P-TM-2] shows that `getSqrtRatioAtTick` is the only function that uses rounding-up:

> ```solidity
> sqrtPriceX96 = uint160((ratio >> 32) + (ratio % (1 << 32) == 0 ? 0 : 1));
> ```

The `+ (ratio % (1 << 32) == 0 ? 0 : 1)` is rounding the 128.128 intermediate up to 128.96, then narrowing to `uint160`. This is deliberate — the documentation says "Each of the 'magic numbers' in `getSqrtRatioAtTick()` are represented as 128-bit fixed point numbers rounded up (except tick 1)."

`source-backed finding` — SqrtPriceMath.sol [P-SPM-1] shows the swap-step primitives accept a `bool roundUp` parameter that flips rounding direction:

> ```solidity
> function getAmount0Delta(
>     uint160 sqrtRatioAX96,
>     uint160 sqrtRatioBX96,
>     uint128 liquidity,
>     bool roundUp
> ) internal pure returns (uint256 amount0)
> ```

The reason: when computing how much token0 a swap pulls *out* of the pool (output side), round down so the LP doesn't pay too much; when computing how much token0 a swap pushes *into* the pool (input side), round up so the LP gets at least the modeled amount. The on-chain pool selects the right direction based on `zeroForOne` and "input vs output."

`project inference` — for an Aurix backtester that aims for "within $X / 0.5% tolerance" (M2.4 acceptance), naive symmetric rounding is good enough to pass. But for the "I match on-chain to the wei" hiring claim that becomes possible if the implementer is careful, you need to mirror Solidity's rounding direction at every step. The simplest path: implement `mul_div(a, b, denom, round_up: bool)` as the single primitive, and pass the flag through every call site.

### 1.5 The Aurix V3 decode is correct (for what it does)

`repository fact` — `dex/uniswap_v3.rs:76-125` already does the right thing for spot-price decoding: it computes `sqrtPriceX96² · 10^(d0-d1) / 2^192` (or the reciprocal when base is token1), in `BigUint` end-to-end, narrowing to `f64` only at the final ratio. That decode is fine for the Tab 1 dashboard. **It does not need to be changed for M2.2** — the math primitives for the backtester live in a different module and use the `sqrtPriceX96` value as input, not the derived `f64` price.

---

## Section 2 — Tick ↔ sqrtPriceX96 Conversion

### 2.1 The geometric tick grid

Uniswap V3 indexes prices on a logarithmic grid of base 1.0001:

```
p(i) = 1.0001^i               // price at tick i (token1 per token0)
sqrtP(i) = 1.0001^(i/2)       // square-root price at tick i
```

`source-backed finding` — Uniswap blog math primer 2 [P-MP2-1]:

> "√P / 2^96 = 1.0001^(i_c)"
> "log(√P / 2^96) / log(1.0001) = i_c"

A tick is therefore a basis-point-scale price change: each tick is a factor of 1.0001 in price (1 bp), or √1.0001 ≈ 1.00005 in square-root price (≈ 0.5 bp).

### 2.2 The boundaries: MIN_TICK and MAX_TICK

`source-backed finding` — TickMath.sol [P-TM-3]:

> ```solidity
> int24 internal constant MIN_TICK = -887272;
> int24 internal constant MAX_TICK = -MIN_TICK;
> ```
>
> "The minimum tick that may be passed to #getSqrtRatioAtTick computed from log base 1.0001 of 2⁻¹²⁸" and the maximum computed from 2¹²⁸.

The derivation: the protocol wants `sqrtPrice` to fit within the `uint160` Q64.96 range, which corresponds to prices in [2⁻¹²⁸, 2¹²⁸]. Solving `1.0001^i = 2¹²⁸` gives `i = 128 · ln(2) / ln(1.0001) ≈ 887272.7517`. The protocol takes the floor: `MAX_TICK = 887272`. By symmetry, `MIN_TICK = -887272`.

The `int24` Solidity type chosen for the tick (rather than `int32`) is because `887272 < 2²³ = 8388608` — a tick fits in 24 bits, which lets the pool pack the current tick into the same 256-bit storage slot as `sqrtPriceX96` and other state. **In Rust, use `i32` — there is no native `int24`, and any production V3 math library uses `i32` or `i64`.**

### 2.3 The `getSqrtRatioAtTick` magic-constant algorithm

The protocol does not call `pow(1.0001, tick / 2)` at runtime — that would either require a floating-point library or expensive iterative arithmetic. Instead it uses a precomputed binary-decomposition trick: for any `tick`, `1.0001^|tick|` is the product of `1.0001^(2^k)` for each bit `k` set in `|tick|`. Each `1.0001^(2^k)` is precomputed and stored as a 128.128-bit fixed-point constant.

`source-backed finding` — TickMath.sol [P-TM-4], complete function body:

> ```solidity
> function getSqrtRatioAtTick(int24 tick) internal pure returns (uint160 sqrtPriceX96) {
>     uint256 absTick = tick < 0 ? uint256(-int256(tick)) : uint256(int256(tick));
>     require(absTick <= uint256(MAX_TICK), 'T');
>     uint256 ratio = absTick & 0x1 != 0 ? 0xfffcb933bd6fad37aa2d162d1a594001 : 0x100000000000000000000000000000000;
>     if (absTick & 0x2 != 0) ratio = (ratio * 0xfff97272373d413259a46990580e213a) >> 128;
>     if (absTick & 0x4 != 0) ratio = (ratio * 0xfff2e50f5f656932ef12357cf3c7fdcc) >> 128;
>     if (absTick & 0x8 != 0) ratio = (ratio * 0xffe5caca7e10e4e61c3624eaa0941cd0) >> 128;
>     if (absTick & 0x10 != 0) ratio = (ratio * 0xffcb9843d60f6159c9db58835c926644) >> 128;
>     if (absTick & 0x20 != 0) ratio = (ratio * 0xff973b41fa98c081472e6896dfb254c0) >> 128;
>     if (absTick & 0x40 != 0) ratio = (ratio * 0xff2ea16466c96a3843ec78b326b52861) >> 128;
>     if (absTick & 0x80 != 0) ratio = (ratio * 0xfe5dee046a99a2a811c461f1969c3053) >> 128;
>     if (absTick & 0x100 != 0) ratio = (ratio * 0xfcbe86c7900a88aedcffc83b479aa3a4) >> 128;
>     if (absTick & 0x200 != 0) ratio = (ratio * 0xf987a7253ac413176f2b074cf7815e54) >> 128;
>     if (absTick & 0x400 != 0) ratio = (ratio * 0xf3392b0822b70005940c7a398e4b70f3) >> 128;
>     if (absTick & 0x800 != 0) ratio = (ratio * 0xe7159475a2c29b7443b29c7fa6e889d9) >> 128;
>     if (absTick & 0x1000 != 0) ratio = (ratio * 0xd097f3bdfd2022b8845ad8f792aa5825) >> 128;
>     if (absTick & 0x2000 != 0) ratio = (ratio * 0xa9f746462d870fdf8a65dc1f90e061e5) >> 128;
>     if (absTick & 0x4000 != 0) ratio = (ratio * 0x70d869a156d2a1b890bb3df62baf32f7) >> 128;
>     if (absTick & 0x8000 != 0) ratio = (ratio * 0x31be135f97d08fd981231505542fcfa6) >> 128;
>     if (absTick & 0x10000 != 0) ratio = (ratio * 0x9aa508b5b7a84e1c677de54f3e99bc9) >> 128;
>     if (absTick & 0x20000 != 0) ratio = (ratio * 0x5d6af8dedb81196699c329225ee604) >> 128;
>     if (absTick & 0x40000 != 0) ratio = (ratio * 0x2216e584f5fa1ea926041bedfe98) >> 128;
>     if (absTick & 0x80000 != 0) ratio = (ratio * 0x48a170391f7dc42444e8fa2) >> 128;
>     if (tick > 0) ratio = type(uint256).max / ratio;
>     sqrtPriceX96 = uint160((ratio >> 32) + (ratio % (1 << 32) == 0 ? 0 : 1));
> }
> ```

How to read this:

- `absTick` is the absolute value of the tick. The function builds the answer for negative ticks first, then inverts at the end via `type(uint256).max / ratio` if the tick was positive (`x · y = 2²⁵⁶ − 1` is a fixed-point reciprocal).
- `ratio` starts as `1.0` in 128.128 fixed-point — that's `0x100000000000000000000000000000000` = `1 << 128` — unless bit 0 of `absTick` is set, in which case it starts as `1.0001^(-1)` rounded up in 128.128 (`0xfffcb933bd6fad37aa2d162d1a594001`).
- For each remaining bit `k` of `absTick`, multiply `ratio` by the precomputed `1.0001^(-2^k)` constant in 128.128, then shift right 128 to renormalise back to 128.128.
- After processing all bits, `ratio` holds `1.0001^(-|absTick|/2) · 2^128` — wait, why `/2`? Because each constant is `√(1.0001^(-2^k))`, not `1.0001^(-2^k)`. Re-read: each magic constant is the *square root* of `1.0001^(-2^k)`. Composing them gives `√(1.0001^(-|absTick|)) = 1.0001^(-|absTick|/2)`, which is `sqrtPrice` not `price`. Good.
- The final `(ratio >> 32) + rounding_correction` shifts from 128.128 down to 128.96 — that is, from `sqrtPrice · 2^128` down to `sqrtPriceX96 = sqrtPrice · 2^96`, then narrows to `uint160`.

The 19 constants encode `1.0001^(2^0)` through `1.0001^(2^18)`. Since `MAX_TICK = 887272 < 2^20 = 1048576`, no constant beyond `2^19` is ever needed.

`source-backed finding` — RareSkills [P-RS-1]:

> "Each of the 'magic numbers' in `getSqrtRatioAtTick()` are represented as 128-bit fixed point numbers rounded up (except tick 1). These magic numbers correspond to values like √(1.0001^0), √(1.0001^(-2^0)), √(1.0001^(-2^1)), and so on through √(1.0001^(-2^19))."

#### Code-ready Rust port (sketch)

```rust
// All constants are u128 values of the original 0xfffc... etc. literals from
// TickMath.sol. The literals must be transcribed exactly — these are the
// rounding-up-pre-computed approximations of 1.0001^(2^k) in Q128.128, and
// regenerating them via floating-point will introduce drift.
const MAGIC_CONSTANTS: [u128; 20] = [
    0xfffcb933bd6fad37aa2d162d1a594001, // bit 0: 1.0001^(-1) Q128.128 round-up
    0xfff97272373d413259a46990580e213a, // bit 1: 1.0001^(-2)
    0xfff2e50f5f656932ef12357cf3c7fdcc, // bit 2: 1.0001^(-4)
    // ... transcribe the rest exactly from TickMath.sol [P-TM-4]
];

const Q128: BigUint = /* 1u128 << 128 as BigUint */;
const Q96:  BigUint = /* 1u128 <<  96 as BigUint */;
const MAX_TICK: i32 = 887272;

pub fn tick_to_sqrt_price_x96(tick: i32) -> Result<BigUint, V3MathError> {
    let abs_tick = tick.unsigned_abs() as u32;
    if abs_tick > MAX_TICK as u32 {
        return Err(V3MathError::TickOutOfBounds);
    }

    // Start ratio in Q128.128. If bit 0 set, start at 1.0001^(-1); else at 1.0.
    let mut ratio: BigUint = if abs_tick & 0x1 != 0 {
        BigUint::from(MAGIC_CONSTANTS[0])
    } else {
        Q128.clone()
    };

    // For each bit k from 1..20, if set, multiply ratio by 1.0001^(-2^k)
    // (in Q128.128) and shift right 128 to renormalise.
    for k in 1..20 {
        if abs_tick & (1u32 << k) != 0 {
            ratio = (&ratio * BigUint::from(MAGIC_CONSTANTS[k as usize])) >> 128;
        }
    }

    // If tick was positive, invert: ratio = (2^256 - 1) / ratio. This is the
    // fixed-point reciprocal trick — only valid because the constants are 128.128.
    if tick > 0 {
        let max_u256: BigUint = (BigUint::from(1u8) << 256) - BigUint::from(1u8);
        ratio = max_u256 / ratio;
    }

    // Shift down from Q128.128 to Q128.96, rounding up if any low-32 bits are set.
    let low_32_mask: BigUint = (BigUint::from(1u8) << 32) - BigUint::from(1u8);
    let round_up = (&ratio & &low_32_mask) != BigUint::from(0u8);
    let mut sqrt_price_x96 = &ratio >> 32;
    if round_up {
        sqrt_price_x96 += BigUint::from(1u8);
    }

    // The result fits in uint160 by construction; an oversize value is a bug.
    Ok(sqrt_price_x96)
}
```

The canonical test fixture:

| `tick` | `sqrtPriceX96` (decimal) | Source |
|---|---|---|
| `MIN_TICK = -887272` | `4295128739` (= `MIN_SQRT_RATIO`) | TickMath.sol [P-TM-1] |
| `0` | `79228162514264337593543950336` (= `2^96`) | by definition: `√1 · 2^96` |
| `MAX_TICK = 887272` | `1461446703485210103287273052203988822378723970342` (= `MAX_SQRT_RATIO`) | TickMath.sol [P-TM-1] |

`project inference` — when implementing in Rust, write a regression test that hits these three boundaries plus at least 5 mid-range ticks (both signs). If `tick_to_sqrt_price_x96(0)` doesn't return exactly `2^96`, you have a transcription error in the constants.

### 2.4 The inverse: `getTickAtSqrtRatio`

The inverse function takes a `sqrtPriceX96` and returns the *largest* tick `i` such that `getSqrtRatioAtTick(i) ≤ sqrtPriceX96`. The algorithm uses a binary-search-by-most-significant-bit technique: it computes `log_2(sqrtPriceX96 / 2^96)` to ~14 bits of precision via assembly bit-tricks, then converts to log-base-1.0001 via a fixed-point multiply, then evaluates `getSqrtRatioAtTick` at the candidate tick and the next-higher tick to pick the correct one.

`source-backed finding` — TickMath.sol via the doc page [P-TM-5]:

> "Calculates the greatest tick value such that getRatioAtTick(tick) <= ratio [...] using bit-manipulation for MSB detection and iterative logarithm approximation via assembly blocks, concluding with boundary logic selecting between tickLow and tickHi based on ratio comparison."

For an off-chain Rust reproduction, two simpler implementations are valid:

1. **Bit-trick port.** Transcribe the Solidity assembly directly. Faster, exactly matches on-chain behaviour at the bit level. The constants are hardcoded in TickMath.sol.
2. **`f64` log + verify.** Compute `tick_estimate = floor(2 · log(sqrtPriceX96 / 2^96) / log(1.0001))`, then evaluate `tick_to_sqrt_price_x96(tick_estimate)` and `tick_to_sqrt_price_x96(tick_estimate + 1)`, and pick the largest one whose result `≤ sqrtPriceX96`. Slower, but self-validating. Recommended for the Aurix backtester — the backtester runs off-chain and is not gas-constrained, and the verify step catches floating-point drift at the boundary.

A boundary case caught by `v3-core` issue [#578][P-ISSUE-578]: the round-trip `tick → sqrtPriceX96 → tick` is *not exactly* the identity function in floating-point. The user reported that for tick `202475`, `math.pow(1.0001, 202475)` = `620780237.5507622` whereas `(sqrtPriceX96 / 2^96)^2` = `620804961.6538478` — an absolute discrepancy of ~24700 in the price (4 ppm).

> "math.pow(1.0001, 202475) yields 620780237.5507622"
> "(1974045567390486984838358761822072 * 2^-96)² yields 620804961.6538478"

This is the precision loss inherent to the magic-constant approximation rounding-up at every multiply. It is not a bug — it is the expected gap between the two computational paths, and it is exactly the reason the `f64`-log verify-step approach above must be a *verify* step, not a standalone answer.

---

## Section 3 — Liquidity ↔ Token Amounts

### 3.1 The whitepaper relations

From the V3 whitepaper relations (Section 6.2.1 of the whitepaper, equations 6.29 and 6.30 in the published version), for a position spanning `[√pa, √pb]` with current price `√p`:

```
when √p ≤ √pa  (price below range, position is 100% token0):
    Δx = L · (1/√pa − 1/√pb)
    Δy = 0

when √pa < √p < √pb  (in range):
    Δx = L · (1/√p − 1/√pb)
    Δy = L · (√p − √pa)

when √p ≥ √pb  (price above range, position is 100% token1):
    Δx = 0
    Δy = L · (√pb − √pa)
```

`source-backed finding` — Uniswap blog math primer 2 [P-MP2-2] confirms the in-range case:

> ```
> token_0 = ℓ × (√p_u - √p') / (√p' × √p_u)
> token_1 = ℓ × (√p' - √p_l)
> ```

(`ℓ` = L, `√p'` = current sqrtPrice, `√p_l` / `√p_u` = lower / upper sqrtPrice bounds.)

And the out-of-range case (price ≤ lower):

> ```
> token_0 = ℓ × (√p_u - √p_l) / (√p_l × √p_u)
> token_1 = 0
> ```

(Algebraically equivalent to the whitepaper form: `(√pu − √pa) / (√pa · √pu) = 1/√pa − 1/√pu`.)

### 3.2 The Solidity implementation: `getAmountsForLiquidity`

`source-backed finding` — LiquidityAmounts.sol [P-LA-1], full body:

> ```solidity
> function getAmountsForLiquidity(
>     uint160 sqrtRatioX96,
>     uint160 sqrtRatioAX96,
>     uint160 sqrtRatioBX96,
>     uint128 liquidity
> ) internal pure returns (uint256 amount0, uint256 amount1) {
>     if (sqrtRatioAX96 > sqrtRatioBX96) (sqrtRatioAX96, sqrtRatioBX96) = (sqrtRatioBX96, sqrtRatioAX96);
>     if (sqrtRatioX96 <= sqrtRatioAX96) {
>         amount0 = getAmount0ForLiquidity(sqrtRatioAX96, sqrtRatioBX96, liquidity);
>     } else if (sqrtRatioX96 < sqrtRatioBX96) {
>         amount0 = getAmount0ForLiquidity(sqrtRatioX96, sqrtRatioBX96, liquidity);
>         amount1 = getAmount1ForLiquidity(sqrtRatioAX96, sqrtRatioX96, liquidity);
>     } else {
>         amount1 = getAmount1ForLiquidity(sqrtRatioAX96, sqrtRatioBX96, liquidity);
>     }
> }
> ```
>
> ```solidity
> function getAmount0ForLiquidity(
>     uint160 sqrtRatioAX96,
>     uint160 sqrtRatioBX96,
>     uint128 liquidity
> ) internal pure returns (uint256 amount0) {
>     if (sqrtRatioAX96 > sqrtRatioBX96) (sqrtRatioAX96, sqrtRatioBX96) = (sqrtRatioBX96, sqrtRatioAX96);
>     return
>         FullMath.mulDiv(
>             uint256(liquidity) << FixedPoint96.RESOLUTION,
>             sqrtRatioBX96 - sqrtRatioAX96,
>             sqrtRatioBX96
>         ) / sqrtRatioAX96;
> }
> ```
>
> ```solidity
> function getAmount1ForLiquidity(
>     uint160 sqrtRatioAX96,
>     uint160 sqrtRatioBX96,
>     uint128 liquidity
> ) internal pure returns (uint256 amount1) {
>     if (sqrtRatioAX96 > sqrtRatioBX96) (sqrtRatioAX96, sqrtRatioBX96) = (sqrtRatioBX96, sqrtRatioAX96);
>     return FullMath.mulDiv(liquidity, sqrtRatioBX96 - sqrtRatioAX96, FixedPoint96.Q96);
> }
> ```

Translating: `getAmount0ForLiquidity` computes `L · 2^96 · (√pb − √pa) / (√pb · √pa)`, which simplifies algebraically to `L · (1/√pa − 1/√pb)` — the Q96-scaling cancels because `√pa` and `√pb` are themselves in Q96. Likewise `getAmount1ForLiquidity` computes `L · (√pb − √pa) / 2^96`, matching `Δy = L · (√pb − √pa)` once you account for `√p` being in Q96.

### 3.3 The Solidity implementation: `getLiquidityForAmounts`

The inverse direction — *"given amount0 and amount1 of tokens to deposit, what's the maximum L that fits both"* — is computed branch by branch.

`source-backed finding` — LiquidityAmounts.sol [P-LA-2]:

> ```solidity
> function getLiquidityForAmount0(
>     uint160 sqrtRatioAX96,
>     uint160 sqrtRatioBX96,
>     uint256 amount0
> ) internal pure returns (uint128 liquidity) {
>     if (sqrtRatioAX96 > sqrtRatioBX96) (sqrtRatioAX96, sqrtRatioBX96) = (sqrtRatioBX96, sqrtRatioAX96);
>     uint256 intermediate = FullMath.mulDiv(sqrtRatioAX96, sqrtRatioBX96, FixedPoint96.Q96);
>     return toUint128(FullMath.mulDiv(amount0, intermediate, sqrtRatioBX96 - sqrtRatioAX96));
> }
>
> function getLiquidityForAmount1(
>     uint160 sqrtRatioAX96,
>     uint160 sqrtRatioBX96,
>     uint256 amount1
> ) internal pure returns (uint128 liquidity) {
>     if (sqrtRatioAX96 > sqrtRatioBX96) (sqrtRatioAX96, sqrtRatioBX96) = (sqrtRatioBX96, sqrtRatioAX96);
>     return toUint128(FullMath.mulDiv(amount1, FixedPoint96.Q96, sqrtRatioBX96 - sqrtRatioAX96));
> }
>
> function getLiquidityForAmounts(
>     uint160 sqrtRatioX96,
>     uint160 sqrtRatioAX96,
>     uint160 sqrtRatioBX96,
>     uint256 amount0,
>     uint256 amount1
> ) internal pure returns (uint128 liquidity) {
>     if (sqrtRatioAX96 > sqrtRatioBX96) (sqrtRatioAX96, sqrtRatioBX96) = (sqrtRatioBX96, sqrtRatioAX96);
>     if (sqrtRatioX96 <= sqrtRatioAX96) {
>         liquidity = getLiquidityForAmount0(sqrtRatioAX96, sqrtRatioBX96, amount0);
>     } else if (sqrtRatioX96 < sqrtRatioBX96) {
>         uint128 liquidity0 = getLiquidityForAmount0(sqrtRatioX96, sqrtRatioBX96, amount0);
>         uint128 liquidity1 = getLiquidityForAmount1(sqrtRatioAX96, sqrtRatioX96, amount1);
>         liquidity = liquidity0 < liquidity1 ? liquidity0 : liquidity1;
>     } else {
>         liquidity = getLiquidityForAmount1(sqrtRatioAX96, sqrtRatioBX96, amount1);
>     }
> }
> ```

The algebra:

- `Lx = amount0 · (√pa · √pb) / (√pb − √pa)` — derives from `Δx = L · (1/√pa − 1/√pb)` solved for L.
- `Ly = amount1 / (√pb − √pa)` — derives from `Δy = L · (√pb − √pa)` solved for L.
- When in range, both bounds apply, and L = min(Lx, Ly). The smaller value is selected because the position must be balanced in both tokens; using the larger would imply you wanted more of one token than you provided.
- When out of range below, only token0 contributes (the position is 100% token0), and L is determined by amount0 alone with bounds [pa, pb].
- When out of range above, only token1 contributes, and L is determined by amount1 alone.

`source-backed finding` — Atis Elsts via Uniswap dev book [P-DB-1]:

> "L = Δx · √(p_b) · √(p_c) / (√(p_b) − √(p_c))"
> "L = Δy / (√(p_c) − √(p_a))"

(Elsts's notation: p_c = current price = the lower bound when computing Lx in-range; p_a / p_b = lower / upper. The whitepaper and dev book diverge slightly on which letter is "current", but the algebra is identical.)

### 3.4 Worked example — ETH/USDC, 5bps pool, ~$2000/ETH

Setup:

- Pool: WETH/USDC 5bps. `token0 = USDC` (decimals 6), `token1 = WETH` (decimals 18). Note: USDC has the lower address on Ethereum mainnet, so it sorts as token0.
- Current price (USDC per WETH): 2000.
- Range: ±10% around 2000, so `[1818.18, 2222.22]`.

Step 1 — convert prices to ticks. Recall the on-chain price `p = token1 / token0` in raw integer units = `WETH_wei / USDC_wei`. Adjusted for decimals, an exchange rate of 2000 USDC per WETH means:

```
p_pool = (1 WETH in wei) / (2000 USDC in 6dec)
       = (1 · 10^18) / (2000 · 10^6)
       = 10^18 / (2 · 10^9)
       = 5 · 10^8     // raw price as token1/token0 in wei units
```

But wait — this is "WETH wei per USDC wei", which is huge because WETH has more decimals. The displayed exchange rate (USDC/WETH = 2000) is the *inverse* of the on-chain price when token0 is USDC.

Equivalently: `p_pool = 1 / 5e8 ≈ 2e-9` for "USDC wei per WETH wei". The pool stores `p_pool = token1/token0 = WETH_wei/USDC_wei = 5e8`.

Wait — let's just use the protocol convention: `p = token1 / token0`. With token0=USDC and token1=WETH, `p` is "WETH per USDC", measured in wei-units. So 1 USDC buys 1/2000 = 5e-4 WETH, scaled by decimal correction:

```
p_pool = (5e-4 WETH) / (1 USDC)
       = (5e-4 · 10^18 WETH_wei) / (10^6 USDC_wei)
       = 5e14 / 10^6
       = 5e8     // WETH_wei per USDC_wei
```

So `p_pool = 5 · 10^8`. Then `√p_pool ≈ 22360.68`, and `sqrtPriceX96 ≈ 22360.68 · 2^96 ≈ 1.7714 · 10^33`.

Compute the tick: `tick = floor(log(p_pool) / log(1.0001)) = floor(log(5e8) / log(1.0001))`.

```
log(5e8) ≈ 20.0299
log(1.0001) ≈ 0.0001
tick ≈ floor(20.0299 / 0.0001) ≈ 200299
```

Closest mainnet observation: WETH/USDC 5bps tick around USD-2000 era is in the 200000–202000 range, so this estimate is consistent. Use `tick_to_sqrt_price_x96(200299)` to get the canonical sqrtPriceX96 for this scenario. (For an actual fixture, an Aurix implementer should sample one block from the WETH/USDC 5bps pool and use the live `slot0()` value as ground truth.)

Step 2 — compute lower / upper sqrtPriceX96.

- `±10%` around price 2000 → range `[1818.18, 2222.22]` USDC/WETH.
- In pool terms: `p_lower = 5e8 / 1.1 ≈ 4.545e8`, `p_upper = 5e8 · 1.1 = 5.5e8`.
- `tick_lower ≈ floor(log(4.545e8) / log(1.0001)) ≈ 199346` → snap to nearest multiple of 10 (5bps tick spacing) → `199350`.
- `tick_upper ≈ floor(log(5.5e8) / log(1.0001)) ≈ 201252` → snap to multiple of 10 → `201250`.

Step 3 — compute Lx and Ly for, say, $1000 worth of each token: amount0 = 1000 USDC = 1e9 USDC_wei, amount1 = 0.5 WETH = 5e17 WETH_wei.

```
√pa ≈ 1.0001^(199350/2) · 2^96    (use tick_to_sqrt_price_x96(199350))
√p  ≈ 1.0001^(200299/2) · 2^96
√pb ≈ 1.0001^(201250/2) · 2^96
```

Numerically (approximate, for illustration; the implementer's tests should use exact tick_to_sqrt_price_x96 outputs):

```
√pa ≈ 21321.0  →  √pa · 2^96 ≈ 1.689e33
√p  ≈ 22360.7  →  √p  · 2^96 ≈ 1.771e33
√pb ≈ 23452.1  →  √pb · 2^96 ≈ 1.858e33
```

Then:

```
Lx = amount0 · (√pa · √pb) / (√pb − √pa)        // both √'s in Q96
   = 1e9 · (21321.0 · 23452.1 · 2^96) / (23452.1 − 21321.0)
   ≈ 1e9 · (5.0e8 · 2^96) / 2131.1
   ≈ 1e9 · 1.86e22
   ≈ 1.86e31     (raw, before Q96 cancellation; in protocol units L ≈ 2.35e22)

Ly = amount1 · 2^96 / (√pb − √pa)
   = 5e17 · 2^96 / (2131.1 · 2^96)
   ≈ 5e17 / 2131.1
   ≈ 2.35e14     (in raw units; L ≈ 2.35e14 · 2^96 / 2^96 = 2.35e14... discrepancy)
```

The numerical example above is illustrative and the exact constants will need a small Rust testbed to verify. The point is the *shape* of the calculation. **For an actual implementation, write a fixture that reads the live `slot0` of WETH/USDC 5bps at a specific block, picks a known tick range, and computes the resulting L; then mint a real position on a fork and compare.**

Step 4 — `liquidity = min(Lx, Ly)`. The smaller value wins; if Lx < Ly, the position is bound by token0, and it returns less of token1 than supplied (the leftover is rebated by the periphery contract on mint).

### 3.5 Code-ready Rust port

```rust
pub fn liquidity_for_amounts(
    sqrt_price_x96: &BigUint,
    sqrt_a_x96: &BigUint,
    sqrt_b_x96: &BigUint,
    amount0: &BigUint,
    amount1: &BigUint,
) -> Result<u128, V3MathError> {
    // Sort the bounds: convention is sqrt_a <= sqrt_b.
    let (sa, sb) = if sqrt_a_x96 > sqrt_b_x96 {
        (sqrt_b_x96, sqrt_a_x96)
    } else {
        (sqrt_a_x96, sqrt_b_x96)
    };

    let liq: BigUint = if sqrt_price_x96 <= sa {
        // Below range: only amount0 binds.
        liquidity_for_amount0(sa, sb, amount0)?
    } else if sqrt_price_x96 < sb {
        // In range: both bind, take the smaller.
        let l0 = liquidity_for_amount0(sqrt_price_x96, sb, amount0)?;
        let l1 = liquidity_for_amount1(sa, sqrt_price_x96, amount1)?;
        if l0 < l1 { l0 } else { l1 }
    } else {
        // Above range: only amount1 binds.
        liquidity_for_amount1(sa, sb, amount1)?
    };

    // Narrow to u128 — overflow here means the position is unrealistic for V3.
    liq.to_u128().ok_or(V3MathError::LiquidityOverflow)
}

fn liquidity_for_amount0(
    sa: &BigUint,
    sb: &BigUint,
    amount0: &BigUint,
) -> Result<BigUint, V3MathError> {
    // L = amount0 · (√pa · √pb) / (√pb − √pa), where the √'s are in Q96.
    // The protocol equivalent: FullMath.mulDiv(amount0, intermediate, sb - sa)
    //                           where intermediate = mulDiv(sa, sb, Q96).
    let q96 = BigUint::from(1u128) << 96;
    let intermediate = (sa * sb) / &q96;     // sa * sb in Q192 → Q96
    let denom = sb - sa;                      // both in Q96, result in Q96
    let raw = amount0 * intermediate / denom; // L in raw units
    Ok(raw)
}

fn liquidity_for_amount1(
    sa: &BigUint,
    sb: &BigUint,
    amount1: &BigUint,
) -> Result<BigUint, V3MathError> {
    // L = amount1 · Q96 / (√pb − √pa)
    let q96 = BigUint::from(1u128) << 96;
    let denom = sb - sa;
    let raw = amount1 * q96 / denom;
    Ok(raw)
}
```

The protocol uses `FullMath.mulDiv` instead of naive `(a * b) / c` to avoid intermediate overflow when `a * b` exceeds `uint256`. With `BigUint` this is automatic — the intermediate has unbounded width — so the simpler form is correct in Rust, but the implementer must pay attention to performance for hot loops (each `BigUint` multiply allocates).

---

## Section 4 — Fee Growth Accounting

The V3 fee model is genuinely clever and is the protocol's main constant-time accounting innovation. The goal: track per-position accumulated fees without iterating over all positions on every swap.

### 4.1 The three accumulators

| Accumulator | Stored on | Lifetime | Meaning |
|---|---|---|---|
| `feeGrowthGlobalXX128` | `UniswapV3Pool` (one per token) | pool-wide, monotonically non-decreasing | Total fees collected per unit of (active) liquidity since pool inception, scaled by 2¹²⁸ |
| `feeGrowthOutsideXX128` | each initialized `Tick` (one per token) | Per-tick; flips on each cross | Fees that have accumulated *outside* this tick's "side" — defined operationally below |
| `feeGrowthInsideLastXX128` | each `Position` (one per token) | snapshot at last interaction | The `feeGrowthInside` value the position last observed; subtracted when the position next collects |

`source-backed finding` — Tick.sol [P-TICK-1]:

> ```solidity
> struct Info {
>     uint128 liquidityGross;
>     int128 liquidityNet;
>     uint256 feeGrowthOutside0X128;
>     uint256 feeGrowthOutside1X128;
>     int56 tickCumulativeOutside;
>     uint160 secondsPerLiquidityOutsideX128;
>     uint32 secondsOutside;
>     bool initialized;
> }
> ```

The struct shows the same "outside" pattern is used for fee growth, time, and seconds-per-liquidity (the oracle accumulators).

### 4.2 The accumulation loop — per-step, not per-block

When a swap executes, the pool walks the tick grid one *step* at a time. Each step swaps within a single tick range (until either the swap amount is exhausted or the next initialized tick is reached). At the end of each step, the fee for that step is added to the global accumulator, divided by the *currently active* liquidity at that step.

`source-backed finding` — UniswapV3Pool.sol [P-POOL-1]:

> ```solidity
> if (state.liquidity > 0)
>   state.feeGrowthGlobalX128 += FullMath.mulDiv(step.feeAmount,
>     FixedPoint128.Q128, state.liquidity);
> ```

**This is the key passage of the entire fee model.** Read it carefully:

- `state.feeGrowthGlobalX128` is a Q128.128 accumulator: the cumulative fee per unit of active liquidity, scaled by 2¹²⁸.
- `step.feeAmount` is the absolute fee for this step in wei of the input token.
- `state.liquidity` is the *active liquidity at the moment of this step* — which can change mid-swap if the swap crosses a tick.
- The increment is `step.feeAmount · 2^128 / state.liquidity`. The `2^128` scaling exists so the integer accumulator can hold the per-unit fee without losing precision (a single swap's fee per unit liquidity might be ~10⁻¹² — far smaller than 1, so without scaling the integer increment would round to 0).

`source-backed finding` — Uniswap dev book on swap fees [P-DB-2]:

> "Each pool has `feeGrowthGlobal0X128` and `feeGrowthGlobal1X128` state variables that track total accumulated fees per unit of liquidity (that is, fee amount divided by the pool's liquidity)."

### 4.3 The `feeGrowthOutside` flip

When a swap crosses an initialized tick, that tick's `feeGrowthOutside` is *flipped* relative to the global. The trick: the absolute meaning of `feeGrowthOutside` doesn't matter — only the *delta* between two snapshots matters. So flipping `outside ← global − outside` at every cross effectively redefines which side of the tick "outside" refers to, while preserving the right answer for any position that brackets the tick.

`source-backed finding` — Tick.sol via dev book [P-TICK-2]:

> ```solidity
> info.feeGrowthOutside0X128 =
>     feeGrowthGlobal0X128 -
>     info.feeGrowthOutside0X128;
> ```

This is the entire `cross()` operation for fees. The seconds-per-liquidity, tick-cumulative, and seconds-outside accumulators get the same treatment.

### 4.4 Computing `feeGrowthInside` for a position

At any moment, the fees-per-unit-liquidity earned *inside* a position's tick range is:

```
feeGrowthInside = feeGrowthGlobal − feeGrowthBelow − feeGrowthAbove
```

where `feeGrowthBelow` and `feeGrowthAbove` are computed from each tick's `feeGrowthOutside` field, conditional on whether `tickCurrent` is above or below that tick.

`source-backed finding` — Uniswap dev book [P-DB-3]:

> "fr​=fg​−fb​(il​)−fa​(iu​)"

The conditional logic in Solidity (paraphrasing the dev book quote):

```text
if tickCurrent >= tickLower:
    feeGrowthBelow = tickLower.feeGrowthOutside
else:
    feeGrowthBelow = feeGrowthGlobal − tickLower.feeGrowthOutside

if tickCurrent < tickUpper:
    feeGrowthAbove = tickUpper.feeGrowthOutside
else:
    feeGrowthAbove = feeGrowthGlobal − tickUpper.feeGrowthOutside

feeGrowthInside = feeGrowthGlobal − feeGrowthBelow − feeGrowthAbove
```

### 4.5 Settling fees on a position

When a position calls `collect()` (or any operation that touches the position), it:

1. Reads current `feeGrowthInside` (per the above).
2. Subtracts its stored `feeGrowthInsideLast`.
3. Multiplies by its `liquidity` and unscales by `2^128` to get the absolute fee owed in wei.
4. Updates `feeGrowthInsideLast` to the current value.

`source-backed finding` — Uniswap dev book [P-DB-4]:

> ```solidity
> uint128 tokensOwed0 = uint128(
>     PRBMath.mulDiv(
>         feeGrowthInside0X128 - self.feeGrowthInside0LastX128,
>         self.liquidity,
>         FixedPoint128.Q128
>     )
> );
> ```

This is the **exact formula a backtester needs** for fee settlement. The whitepaper formulation:

`source-backed finding` — Uniswap blog math primer 2 [P-MP2-3]:

> "fees_0 = ℓ × (f_r(t_1) − f_r(t_0)) / 2^128"

(`f_r` = feeGrowthInside; `t_0` and `t_1` = the snapshots at the start and end of the period being settled.)

### 4.6 Walked example — single-tick LP earning fees on one swap

Setup:

- Pool: WETH/USDC 5bps. Token0 = USDC, Token1 = WETH. Fee = 0.05% = 500 / 10⁶.
- Single LP "Alice" mints a position over `[tickLower=200000, tickUpper=200010]` with `liquidity = 10^18` (Q96 units).
- Pre-swap state: `feeGrowthGlobal0 = 0`, `feeGrowthGlobal1 = 0`, current tick = `200005` (in Alice's range).
- A trader swaps 10000 USDC (token0) into the pool, all of which executes within Alice's range (no tick boundaries crossed).

Walk:

1. Trader sends `amount_in = 10000 · 10^6 = 10^10` USDC wei.
2. Pool computes `step.feeAmount = amount_in · 0.0005 = 5_000_000` USDC wei.
3. Pool computes the swap's effect on `sqrtPriceX96` via `getNextSqrtPriceFromAmount0RoundingUp` (the input is amount0, so `add = true`). The new `sqrtPriceX96` is computed from `sqrtPriceX96 · liquidity / (liquidity + amount_in_after_fee · sqrtPriceX96 / Q96)`. (See SqrtPriceMath.sol [P-SPM-1] for exact form.)
4. Pool updates `state.feeGrowthGlobal0 += 5_000_000 · 2^128 / 10^18 = 5_000_000 · 2^128 / 10^18 ≈ 1.7e21`. (More precisely, `step.feeAmount · Q128 / state.liquidity` in `FullMath.mulDiv` semantics, rounded down.)
5. Alice's tick range was unchanged (no crossing), so `feeGrowthOutside` for ticks 200000 and 200010 is unchanged.
6. Alice's `feeGrowthInside0 = feeGrowthGlobal0 − feeGrowthBelow − feeGrowthAbove`. Since the current tick is between Alice's bounds and was so before the swap too, the "below" and "above" are unchanged. So `Δ feeGrowthInside0 = Δ feeGrowthGlobal0 ≈ 1.7e21`.
7. When Alice settles: `tokensOwed0 = 1.7e21 · 10^18 / 2^128 ≈ 5_000_000 USDC wei = 5 USDC` ✓. Alice earns the entire fee because she was the only LP active at that range.

If a second LP "Bob" had a position over `[199990, 200020]` with `liquidity = 10^18`:

1. The pool's `state.liquidity` = 2·10^18 (both Alice's and Bob's overlap the current tick).
2. `state.feeGrowthGlobal0 += 5_000_000 · 2^128 / (2·10^18) ≈ 0.85e21` (half of before).
3. Alice's `Δ feeGrowthInside0 ≈ 0.85e21`, so she earns `0.85e21 · 10^18 / 2^128 ≈ 2_500_000 USDC wei = 2.5 USDC`.
4. Bob's `Δ feeGrowthInside0 ≈ 0.85e21` likewise, so Bob earns `2.5 USDC`.

Total fee distributed: 5 USDC. ✓

This is the elegance of the model: the global accumulator is updated once per swap step (constant time), and each position's fee is computed lazily by subtraction at collect time. The protocol never iterates over LPs.

### 4.7 The `unchecked` arithmetic invariant

`source-backed finding` — Code4rena finding H-04 [P-C4-1]:

> "The contract is implicitly relies on underflow/overflow when calculating the fee growth, if underflow is prevented, some operations that rely on fee growth will revert."

The subtraction `feeGrowthGlobal − feeGrowthBelow − feeGrowthAbove` can mathematically wrap underflow (uint256 modular arithmetic). On Solidity ≥0.8, this would revert under default checked-arithmetic rules. V3 was originally written for Solidity ~0.7 and *depends on* unchecked wraparound. Forks that re-deploy on ≥0.8 must wrap the fee-growth subtractions in `unchecked { }` or they will revert at runtime under conditions where the on-chain protocol works correctly.

`project inference` — for an Aurix backtester in Rust, `BigUint` is non-negative and non-modular. The naive subtraction `feeGrowthGlobal − feeGrowthBelow − feeGrowthAbove` will panic on underflow. **The correct port is to use modular `uint256` arithmetic explicitly**: store all fee-growth values as `[u8; 32]` or as `u256`-equivalent (e.g. `primitive_types::U256`) and do modular subtraction. Or — a simpler approach — compute the *delta* (`feeGrowthInside_now − feeGrowthInside_last`) using modular arithmetic at exactly that step, and treat the per-position fee delta as the only quantity that needs to be non-negative. This is the same logical structure the protocol uses (the `tokensOwed` formula reads `feeGrowthInside_now − feeGrowthInside_last` as a delta, which is always non-negative provided no liquidity was added/removed between snapshots). Section 9 expands on this.

---

## Section 5 — Per-Swap vs Per-Block Fee Distribution (Aurix Differentiator)

This section is critical because the plan ([vector-a-v3-lp-backtester.md] line ~295) names "per-swap fee distribution" as the differentiator from OSS approximations.

### 5.1 The two approaches

**Per-swap (the on-chain truth):**

For each historical swap event in `[entry_block, exit_block]`:

1. Determine `state.liquidity` at the moment of this swap (active liquidity considering all tick crossings up to and including this swap).
2. Was the position in range at this swap? (Yes if `tickCurrent_before_swap` ∈ [`tickLower`, `tickUpper`)... but careful: the swap may itself cross the position's range.)
3. If in range, compute the position's share of the fee for this swap as `position.liquidity / state.liquidity · step.feeAmount` (or equivalently, walk the same `feeGrowthGlobal` increment the protocol does).
4. Accumulate.

This requires walking every swap event and tracking active liquidity — it is the protocol's algorithm exactly.

**Per-block (the common approximation):**

For each block in `[entry_block, exit_block]`:

1. Take the block's total volume `V_block` = sum of swap absolute amounts.
2. Take the block's average / final / midpoint price, and use it to decide whether the position was "in range" for this block.
3. Compute fee = `V_block · fee_tier · (position.liquidity / pool.totalActiveLiquidity_at_block)`.
4. Accumulate.

This is what most OSS V3 backtesters implement because it's much faster — one row per block instead of one row per swap, and it can be computed from subgraph data without per-swap event decoding.

### 5.2 Where the per-block approximation breaks (four named failure modes)

**Failure mode 1: Within-block tick crossings.**

If a single block contains a swap large enough to cross an initialized tick (or several blocks-worth of swaps within one block on a busy day), `state.liquidity` changes mid-block. The per-block approach uses one liquidity value (typically the start-of-block or end-of-block snapshot), so it either overcredits the position (if the position remains in range while liquidity drops, the per-block approach uses the higher liquidity → smaller share → underestimates fees) or undercredits it (if the position drops out of range mid-block, the per-block approach using end-of-block tick concludes it earned nothing).

*Concrete numerical example:* Suppose a block contains two swaps. Swap 1 is small, executes at active L = 10^18, generates 1 USDC fee. Swap 2 is large, crosses an initialized tick where 5·10^17 of liquidity exits, then executes at active L = 5·10^17, generates 4 USDC fee. The position has L = 10^17 and is in range throughout.
- **Per-swap (truth):** position earns `10^17 / 10^18 · 1 + 10^17 / 5·10^17 · 4 = 0.1 + 0.8 = 0.9 USDC`.
- **Per-block using start-of-block liquidity (10^18):** position earns `10^17 / 10^18 · (1 + 4) = 0.5 USDC`. Underestimates by 44%.
- **Per-block using end-of-block liquidity (5·10^17):** position earns `10^17 / 5·10^17 · (1 + 4) = 1.0 USDC`. Overestimates by 11%.

**Failure mode 2: Position-range crossings within a block.**

If the position is in range at the start of a block but the price moves out of range during the block (or vice versa), the per-block approach is forced to choose one. It earns either zero (treats the position as out the whole block) or the full block's pro rata share (treats it as in the whole block), depending on convention. Both are wrong.

*Concrete example:* Position is in range at the start of a block. A first swap is large and pushes price out of the position's range. A second swap stays out of range. The position earned its share of swap 1's fee but nothing from swap 2.
- **Per-swap:** correct — counts swap 1's fee, omits swap 2's.
- **Per-block "in if start-tick in range":** counts both swaps. Overestimate.
- **Per-block "in if end-tick in range":** counts neither. Underestimate.

**Failure mode 3: MEV-triggered cascading tick crossings.**

A sandwich attack or a large arbitrage bot might cross 5 ticks in a single block. The per-block approach can't represent the intra-block trajectory, so it's forced to attribute fees as if the price were stationary within the block — which is exactly the case the model can't handle.

`source-backed finding` — the contrasting Coinmonks article [P-COIN-1] argues that even Uniswap's *own subgraph* gets fee accounting wrong:

> "USDC-WETH pool: Official reported $333m TVL vs. calculated $176m (nearly 50% overstatement)"
> "Protocol-wide: Official $11.8b vs. author's calculation $3.14b (approximately 73% overstatement)"

This is a different failure (TVL accounting, not fee distribution per se), but it illustrates the broader point: aggregated approximations of V3 quantities frequently diverge from the on-chain reality by economically meaningful amounts.

**Failure mode 4: Liquidity adds/removes within a block.**

If another LP adds or removes liquidity in the same block as a swap, the per-block approach can't separate "fee earned before the add" from "fee earned after the add" — it uses one snapshot. Per-swap correctly attributes fees only to LPs whose position was active at the moment of each swap.

### 5.3 When per-block is good enough

`project inference` — for the *strategy comparison* sweep (M2.5, where you grid-search 50+ strategy configs over 30 days), per-block aggregation would change relative rankings by at most a few percent on most strategies. The high-leverage moments are exactly the high-vol blocks where many ticks cross — those are also the blocks where strategies differ most. Per-block aggregation systematically *under*-resolves the strategies that perform best in those blocks (e.g. a tight range that earns disproportionately on a single high-vol block), so strategy comparison via per-block aggregation has a small but non-random bias against tight-range strategies.

For the *validation harness* (M2.4, where you compare to 5 known on-chain positions), per-block is **not** good enough. The acceptance criterion is "within $X / 0.5%" — and the failure modes above are the exact reason a per-block approach would fail that criterion on positions that experienced any high-vol period.

`project inference` — recommendation: **build per-swap from the start.** The plan's claim ("here's a bug I found" and "most implementations approximate at the block level") only works if the production engine is per-swap. The validation harness then proves it.

### 5.4 Compute cost trade-off

| Approach | Events per 30 days WETH/USDC 5bps (rough) | Memory per backtest | Wall clock estimate |
|---|---|---|---|
| Per-swap | ~100,000–500,000 | minimal (streaming) | seconds for one position |
| Per-block | ~200,000 blocks × (compressed swap aggregate) | minimal | sub-second per position |

The cost difference is roughly 2–5x in CPU time but the per-swap path has *better* asymptotic scaling because most swaps are small and don't cross ticks (the inner loop is a single accumulator update). For Aurix's 30-day backtest scope, per-swap is comfortably feasible.

---

## Section 6 — Impermanent Loss for Concentrated Positions

### 6.1 The V2 reference — where IL comes from

For a constant-product V2 pool (`x · y = k`), the LP's portfolio value at price `P` (relative to value at entry price `P_0`) is:

```
V_LP(P) / V_HODL(P) = 2 · √(P/P_0) / (1 + P/P_0)
```

Let `k = P / P_0` (price ratio change). Then:

```
IL_V2(k) = V_LP / V_HODL − 1 = 2·√k / (1 + k) − 1
```

This is always ≤ 0 (the LP underperforms HODL when prices move) and equals 0 only at `k = 1`.

### 6.2 The V3 generalization — concentrated range

`source-backed finding` — Auditless / Peteris Erins [P-AUDIT-1]:

> "IL_{a,b}(k) = [2·√k / (√(p_b)/√(p_a) + √(p_a)/√(p_b))] − 1"

Equivalent formulation: let `r = √(p_b) / √(p_a)` be the range "width factor" (≥1). Then the V3 IL while the price stays in range is:

```
IL_V3_in_range(k) = (2·√k · √(p_a · p_b)) / (k · p_a + p_b) − 1     (algebraic equivalent)
```

When `r → ∞` (i.e. `p_a → 0` and `p_b → ∞`), this reduces to the V2 formula.

`source-backed finding` — Auditless [P-AUDIT-2]:

> "IL_{0,+∞}(k) = IL(k), a.k.a., the bigger the price range, the more this equation converges to the impermanent loss equation for V2."

### 6.3 Why V3 IL is amplified

The amplification comes from the leverage that concentrated liquidity provides. A position over `[p_a, p_b]` is mathematically equivalent to a "virtual" V2 position with the same `L` but with token-amount offsets that represent only the range, not the full curve. The virtual reserves are smaller than the actual portfolio value, so the same percentage price move corresponds to a larger fraction of the position's value.

Quantitative: for a tight range covering a price band of `±10%` around entry (so `r ≈ 1.22`), IL at `k = 0.9` (price drops 10%, exiting the lower side of the range) is about `−0.5%`. For a wide range covering `±10x` (`r ≈ 100`), IL at the same `k = 0.9` is about `−0.0014%`. The tight range has ~360x the IL for the same price move.

`source-backed finding` — RareSkills / Concentrated liquidity FAQs [P-AUDIT-3]:

> "Even if the liquidity range is big enough to accommodate prices doubling or halving, impermanent loss is nearly 4 times higher than if providing liquidity in the whole range of prices."

### 6.4 The out-of-range asymptotes

When the price exits the range, the position becomes 100% of the side it exited toward. At that point:

- If `P < p_a`: position is 100% token0. Value is `Δx · P + 0 = L · (1/√p_a − 1/√p_b) · P`.
- If `P > p_b`: position is 100% token1. Value is `0 + Δy = L · (√p_b − √p_a)`.

Both behave linearly in price (token0 case) or are price-independent (token1 case). Crucially: **once the position is out of range, it earns no further fees and the IL "freezes" relative to the range boundary** — but that frozen IL is the worst-case IL attainable at the entry composition for that range. The plot:

```text
V_LP / V_HODL    (vertical axis, max at 1.00 at k=1)

   1.00 ┤                  ╭────╮
        │                ╭─╯    ╰─╮
   0.99 ┤              ╭─╯        ╰─╮
        │            ╭─╯            ╰─╮
   0.98 ┤          ╭─╯                ╰─╮     ← V3 in-range IL curve
        │        ╭─╯                    ╰─╮
   0.97 ┤      ╭─╯                        ╰─╮
        │    ╭─╯                            ╰─╮
   0.96 ┤  ╭─╯                                ╰─╮
        │╭─╯                                    ╰─╮
   0.95 ┼─                                        ─╮
        │                                          ╰─── ← out-of-range, 100% token1, value flat
        │                                                  (and no fees)
   ─────┼──────────┬────────┬────────┬──────────
       p_a     p_a·1.5    P_0     p_b·0.7    p_b      price (log scale)
                  │   in-range │       │
                  └────────────┴───────┘
```

The asymptotes are:
- *Below `p_a`:* position is all token0, value falls linearly with price. Curve hits 0 if price hits 0.
- *Above `p_b`:* position is all token1, value is constant in absolute (token1) terms but falls relative to HODL because HODL value rises with price.

For a *V2* position, there are no boundary asymptotes — IL goes to 0 at `k = 1` and approaches `−1.0` only as `k → 0` or `k → ∞`. V3's range concentration makes the curve much steeper near the entry but then truncates the sides.

### 6.5 Code-ready Rust port

```rust
/// Computes the V3 impermanent loss while the price is in range.
/// Returns `(value_lp_per_unit, value_hold_per_unit, il_fraction)`.
/// `il_fraction` is negative for losses (e.g. -0.005 = -0.5%).
pub fn impermanent_loss_in_range(
    sqrt_p_a: f64,        // sqrt(price_lower), real-number scale
    sqrt_p_b: f64,        // sqrt(price_upper)
    sqrt_p_0: f64,        // sqrt(price_entry)
    sqrt_p:   f64,        // sqrt(price_now) — must be in [sqrt_p_a, sqrt_p_b]
) -> (f64, f64, f64) {
    debug_assert!(sqrt_p >= sqrt_p_a && sqrt_p <= sqrt_p_b);
    debug_assert!(sqrt_p_0 >= sqrt_p_a && sqrt_p_0 <= sqrt_p_b);

    // L is normalised to 1. Position value at entry = entry composition value.
    // x_0 + y_0/p_0 (in token0 units). Using L=1:
    let x_0 = 1.0 / sqrt_p_0 - 1.0 / sqrt_p_b;
    let y_0 = sqrt_p_0 - sqrt_p_a;

    // Hold value at price P (in token0 units): x_0 + y_0 / (sqrt_p^2)
    let p = sqrt_p * sqrt_p;
    let v_hold = x_0 + y_0 / p;

    // LP value at price P: x(P) + y(P)/P
    let x_p = 1.0 / sqrt_p - 1.0 / sqrt_p_b;
    let y_p = sqrt_p - sqrt_p_a;
    let v_lp = x_p + y_p / p;

    let il = v_lp / v_hold - 1.0;
    (v_lp, v_hold, il)
}

/// Out-of-range: position has fully converted to one side. Returns
/// (lp_value_in_token0_units, hold_value_in_token0_units, il_fraction).
pub fn impermanent_loss_out_of_range(
    sqrt_p_a: f64,
    sqrt_p_b: f64,
    sqrt_p_0: f64,
    sqrt_p:   f64,
) -> (f64, f64, f64) {
    let x_0 = 1.0 / sqrt_p_0 - 1.0 / sqrt_p_b;
    let y_0 = sqrt_p_0 - sqrt_p_a;
    let p = sqrt_p * sqrt_p;
    let v_hold = x_0 + y_0 / p;

    let v_lp = if sqrt_p < sqrt_p_a {
        // 100% token0 — value = L · (1/sqrt_p_a − 1/sqrt_p_b)
        1.0 / sqrt_p_a - 1.0 / sqrt_p_b
    } else {
        // 100% token1 — value in token0 units = L · (sqrt_p_b − sqrt_p_a) / p
        (sqrt_p_b - sqrt_p_a) / p
    };

    let il = v_lp / v_hold - 1.0;
    (v_lp, v_hold, il)
}
```

The `f64` here is fine for IL — IL is a relative ratio, not an absolute fee, and is presented to the user with 2–4 decimal places of precision. Use `BigUint` for fee amounts and tick math; use `f64` for IL plots.

---

## Section 7 — Tick Spacing per Fee Tier

`source-backed finding` — RareSkills / tick-spacing article [P-RS-2]:

> "The current relationship between fee and tick spacing is shown in the following table."

| Fee tier (bps) | Hundredths-of-bp encoding | Tick spacing | Rationale |
|---|---|---|---|
| 1 | 100 | 1 | Stables (e.g. USDC/USDT). Tightest spacing for highest-precision pricing. Added by governance March 2022. |
| 5 | 500 | 10 | Major pairs (e.g. WETH/USDC). |
| 30 | 3000 | 60 | Volatile pairs (e.g. WETH/UNI). |
| 100 | 10000 | 200 | Exotic / illiquid pairs. |

### 7.1 The constraint on user-supplied ticks

`source-backed finding` — RareSkills [P-RS-3]:

> "if the tick spacing of the pool is set to 10, only tick indexes that are multiples of 10 are usable."

When a user (or backtester) supplies `tickLower` and `tickUpper` for a position, both must be exact multiples of the pool's `tickSpacing`. The pool reverts otherwise (the actual check is in `_modifyPosition` → `checkTicks`).

### 7.2 Why higher fee tiers have wider tick spacing

`source-backed finding` — RareSkills [P-RS-4]:

> "Highly volatile assets tend to cause higher impermanent loss...thus, LPs will demand higher fees...Highly volatile pairs benefit from wider tick spacing to reduce excessive tick crossings."

Two design pressures:

1. **Volatility cost.** Volatile pairs cross more ticks per unit time. Each crossing costs gas (roughly 30k gas per crossed tick). Wider spacing means fewer crossings.
2. **Granularity vs gas.** Tighter spacing gives LPs finer control over their range but at higher gas cost both for the LP (mint/burn) and for the protocol (every swap that crosses pays).

For Aurix, the WETH/USDC 5bps pool (tickSpacing = 10) is the canonical pool. The plan's "Open Decisions" mentions adding the 30bps tier later — that pool has tickSpacing = 60, so any code that hardcodes spacing = 10 will break silently when extended.

`project inference` — make `tickSpacing` a config property of the pool record, not a constant. Validate every user-supplied tick at the API boundary. The plan's eventual UI ("range selector") should snap user input to valid multiples.

---

## Section 8 — activeLiquidity, liquidityNet, liquidityGross

The protocol tracks three different liquidity quantities at three different scopes:

| Quantity | Scope | Type | What it represents | Stored where |
|---|---|---|---|---|
| `state.liquidity` (a.k.a. activeLiquidity) | global / pool-wide | `uint128` | Sum of `L` for every position currently in range | `UniswapV3Pool.liquidity` (one slot, hot path) |
| `tick.liquidityNet` | per initialized tick | `int128` | `Σ (L of positions starting at this tick) − Σ (L of positions ending at this tick)` | Each `Tick.Info` |
| `tick.liquidityGross` | per initialized tick | `uint128` | `Σ (L of positions touching this tick)` (additive, regardless of side) | Each `Tick.Info` |

`source-backed finding` — Tick.sol [P-TICK-1]:

> ```solidity
> struct Info {
>     uint128 liquidityGross;
>     int128 liquidityNet;
>     ...
> }
> ```

### 8.1 Why activeLiquidity matters for swaps

When a swap executes, the fee for each step is `fee_amount · 2^128 / activeLiquidity`. So `state.liquidity` is the denominator of the fee accumulator update. Every initialized tick crossed during a swap modifies `state.liquidity`:

`source-backed finding` — UniswapV3Pool.sol swap loop [P-POOL-2]:

> ```solidity
> int128 liquidityNet = ticks.cross(...);
> if (zeroForOne) liquidityNet = -liquidityNet;
> state.liquidity = LiquidityMath.addDelta(state.liquidity, liquidityNet);
> ```

When the swap direction is `zeroForOne` (price decreasing), crossing a tick *downward* should add `−liquidityNet` to active liquidity (positions whose lower bound is that tick are leaving; positions whose upper bound is that tick are entering — and `liquidityNet` is signed to encode this asymmetrically). The conditional negation handles the direction reversal.

### 8.2 Why liquidityNet is signed

`liquidityNet` is the change to active liquidity that happens when the price crosses *upward* through a tick. It is positive for ticks at which a position's lower bound is set (those positions become active when price rises past) and negative for ticks at which a position's upper bound is set. A single tick can have multiple positions starting and ending, so `liquidityNet` is a signed sum.

When the price crosses *downward* through the same tick, the change is `−liquidityNet` (the same positions become inactive in reverse direction).

### 8.3 Why liquidityGross matters for tick lifecycle

`liquidityGross` is the *total* liquidity touching a tick, regardless of direction. It is used to decide whether a tick should remain in the initialized-tick bitmap. When a position is created, its lower-bound tick gets `liquidityGross += L` and its upper-bound tick gets `liquidityGross += L`. When a position is destroyed, the corresponding subtractions happen. A tick is initialized iff `liquidityGross > 0`.

The pool maintains a bitmap of initialized ticks (by tick-index/wordPos), so the swap loop can find the next initialized tick in `O(1 + bit_scan)` time. If the protocol used `liquidityNet` instead, a tick with two opposite-sign contributions could net to zero and be falsely deinitialized while still being load-bearing. `liquidityGross` prevents this.

### 8.4 Why all three are tracked

The decomposition is non-trivial because of asymmetry in how positions interact with ticks:

| Operation | Reads / writes |
|---|---|
| Mint position | Updates `liquidityGross` and `liquidityNet` at both `tickLower` and `tickUpper` |
| Burn position | Reverse of mint |
| Swap step within a tick | Reads `state.liquidity`, writes `state.feeGrowthGlobalX128` |
| Swap step crossing a tick | Reads `liquidityNet` from the crossed tick, updates `state.liquidity`, flips `feeGrowthOutside` |
| Initialize tick (when crossed) | Verifies `liquidityGross > 0`, marks bitmap bit |

Removing any of the three would force the others to do double duty, with measurable gas cost. `liquidityGross` is necessary for the bitmap lifecycle. `liquidityNet` is necessary for cross direction. `state.liquidity` is necessary because deriving it on demand from per-tick fields would require summing all initialized ticks below the current tick (`O(N)` per swap step) — unworkable on chain.

`project inference` — for an Aurix backtester, the same three are needed. The ingest path (M2.1) should maintain a per-tick `(liquidityGross, liquidityNet, feeGrowthOutside0, feeGrowthOutside1)` table, plus a global `(activeLiquidity, feeGrowthGlobal0, feeGrowthGlobal1, currentTick, sqrtPriceX96)`. The simulation engine (M2.3) replays swaps against these structures.

For the validation harness specifically, the simulation can either (a) maintain its own state and replay every swap event, or (b) snapshot the on-chain state at each swap and compare. Option (a) catches more bugs (you're reproducing the protocol's bookkeeping); option (b) is faster but less stringent. The plan's "4 of 5 positions match within tolerance" criterion is satisfiable by either; the "I implemented V3 from scratch" hiring claim is much stronger with (a).

---

## Section 9 — Implementation Pitfalls and OSS Bug Catalogue

This section is the single highest-leverage section for the M2.4 validation harness and the "here's a bug I found" hiring talking point.

### 9.1 Underflow in `getFeeGrowthInside` on Solidity ≥0.8

**Severity: H-04 (Code4rena 2023-12-particle)**

`source-backed finding` — Code4rena finding [P-C4-1]:

> "Solidity 0.8.23 enforces checked arithmetic. When the function performs these subtractions:
> ```solidity
> feeGrowthInside0X128 = feeGrowthGlobal0X128 - lowerFeeGrowthOutside0X128 - upperFeeGrowthOutside0X128;
> ```
> The intermediate result can temporarily go negative (due to fee growth wraparound behavior), causing a revert before the mathematically correct final value is reached. Uniswap V3's original implementation intentionally relies on overflow/underflow wrapping to handle this."
>
> Mitigation: "Use `unchecked` when calculating `feeGrowthInside0X128` and `feeGrowthInside1X128`."

**Aurix implication:** in Rust, `BigUint` does not wrap, so the naive port `feeGrowthGlobal − feeGrowthBelow − feeGrowthAbove` panics on underflow. The correct port treats fee-growth values as `u256` modular and uses wrapping subtraction. Practically, store fee-growth as `[u8; 32]` or `primitive_types::U256`, and use the wrapping `sub` method. The *delta* between two consecutive `feeGrowthInside` snapshots (which is what the fee formula uses) is always non-negative as a non-modular number, so the modular arithmetic only matters at the per-tick level.

### 9.2 Tick price discrepancy via floating-point reconstruction

**Severity: not a bug, expected behaviour**

`source-backed finding` — v3-core issue #578 [P-ISSUE-578]:

> "math.pow(1.0001, 202475) yields 620780237.5507622"
> "(1974045567390486984838358761822072 * 2^-96)² yields 620804961.6538478"

The discrepancy is ~24700 absolute (4 ppm). Cause: the magic-constant approximation in `getSqrtRatioAtTick` rounds up at every multiply step (Section 1.4), accumulating ~12-bit precision loss versus a hypothetical infinite-precision computation.

**Aurix implication:** when validating `tick_to_sqrt_price_x96` against on-chain values, do not check via `pow(1.0001, tick)` floating-point reconstruction. Check by reading `slot0()` at the live block and comparing the integer values directly. Failure to do this is the most common source of false-positive "my math is broken!" reports during V3 implementation.

### 9.3 TVL miscomputation in subgraph

**Severity: aggregation-level, not protocol-level**

`source-backed finding` — Coinmonks [P-COIN-2]:

> "USDC-WETH pool: Official reported $333m TVL vs. calculated $176m"

The Uniswap subgraph (an indexing service, not the protocol itself) was double-counting fees as part of TVL because it summed all liquidity events without subtracting accumulated fee withdrawals. Fixed within 3 days of the article's publication.

**Aurix implication:** if the backtester ever surfaces TVL or pool-volume figures, do not reach for the subgraph — read on-chain or compute from raw events. The subgraph is an opinionated derived view, not a source of truth, and historical bugs persist in cached figures.

### 9.4 GammaStrategies' Python implementation precision

**Severity: rounding-direction inconsistencies**

`source-backed finding` — search results from GammaStrategies awesome-uniswap-v3 list and uniswap-python issues:

The widely-used `UNI_v3_funcs.py` in GammaStrategies (a fork of JNP777's Python port) implements the LiquidityAmounts library in Python decimals. It does not always match Solidity's rounding direction — specifically, it uses Python's default `Decimal` rounding (`ROUND_HALF_EVEN`) while Solidity uses round-toward-zero (truncation) for division. For tight ranges and large `liquidity` values, this produces sub-wei discrepancies that compound over many swaps.

Public discussions of this issue on the uniswap-python issue tracker note "rounding issues from the Python implementation of the Solidity code."

**Aurix implication:** `BigUint` division in Rust truncates (toward zero) by default — this matches Solidity's behaviour for unsigned types. So a Rust port with `BigUint` arithmetic will match Solidity more closely than a Python port with `Decimal`, *provided* the implementer is careful not to accidentally use floating-point intermediate steps. This is one place Aurix's "no `ethers-rs`, native `num-bigint`" choice quietly pays off.

### 9.5 Fee-growth overflow in long-lived pools

**Severity: theoretical, may matter for very long backtests**

The `feeGrowthGlobalX128` accumulator is a `uint256` Q128.128. It increments by `step.feeAmount · 2^128 / state.liquidity`. For a pool like WETH/USDC 5bps with `state.liquidity` in the 10^18 range and daily fees in the 10^12 range (millions of dollars at 1$/wei in USDC — adjust accordingly), the daily increment is ~ `10^12 · 2^128 / 10^18 ≈ 4 · 10^20`. Total `2^256 ≈ 1.16 · 10^77`. So `feeGrowthGlobal` would take ~10^57 days to overflow — not a concern.

But: forks with smaller liquidity, or pools with much smaller `state.liquidity` (an exotic 100bps pool with $10k liquidity), could overflow in years rather than eons. The protocol's `unchecked` arithmetic handles overflow as wraparound, so it doesn't break the on-chain invariant — but it means an off-chain reproduction that fails to use modular arithmetic can produce nonsense.

**Aurix implication:** treat `feeGrowthGlobal` as `u256` modular *always*. Never assume "the values are small in this period so I can use `BigUint`."

### 9.6 Liquidity overflow in `getLiquidityForAmount0`

**Severity: structural**

`getLiquidityForAmount0` does `mulDiv(amount0, intermediate, sqrtRatioBX96 - sqrtRatioAX96)` where `intermediate = mulDiv(sqrtRatioAX96, sqrtRatioBX96, Q96)`. The intermediate `sqrtRatioAX96 · sqrtRatioBX96` can be up to ~`(2^160)^2 = 2^320`, hence `mulDiv` (full-precision multiply-then-divide) is mandatory. A naive port that does `(sqrtRatioAX96 * sqrtRatioBX96) / Q96` in `uint256` will overflow silently.

**Aurix implication:** in Rust with `BigUint`, the naive form is correct (no fixed-width overflow). But: be deliberate about when you narrow to `u128` / `u256`. Always narrow at the very last step, never inside a chain.

### 9.7 Tick boundary arithmetic at `MIN_TICK` / `MAX_TICK`

The protocol's `getSqrtRatioAtTick` uses a `require(absTick <= MAX_TICK)` guard. Off-chain ports sometimes off-by-one this (using `<` instead of `<=`), which silently fails for the exact boundary case. Test fixture: `tick_to_sqrt_price_x96(887272)` must return exactly `MAX_SQRT_RATIO = 1461446703485210103287273052203988822378723970342`.

The inverse (`getTickAtSqrtRatio`) has corresponding boundary guards: `require(sqrtPriceX96 >= MIN_SQRT_RATIO && sqrtPriceX96 < MAX_SQRT_RATIO)`. Note `<` not `<=` for the upper bound — `MAX_SQRT_RATIO` itself is *not* a valid input; the protocol uses it as the strict upper limit on the sqrtPrice the inner pool state can reach.

### 9.8 The "single-tick-position" edge case

A position with `tickLower = tick_x` and `tickUpper = tick_x + tickSpacing` (the smallest possible position) has a special property: the moment the price crosses either boundary, the position immediately becomes 100% one-sided and stops earning. Many backtest implementations don't handle this cleanly because they use `tickCurrent in [tickLower, tickUpper)` as the "in range" check — but the half-open interval matters: a price exactly at `tickUpper` is *not* in range (the position is 100% token1).

The whitepaper ([P-MP2] "in-range when `i_l ≤ i_c < i_u`") confirms the half-open convention. Implementations that use `<=` on both sides earn phantom fees at the upper boundary.

### 9.9 Non-sequential swap-event ingestion

The Aurix backtester will ingest swaps via `eth_getLogs`. The naive ingestion order is "block ascending, log index ascending within block." This matches the protocol's execution order *within a block*, but a sleepy implementation might re-order logs by timestamp (which is the same for all logs in a block) and lose the intra-block ordering. Re-ordering breaks the per-swap fee distribution model because two swaps in the same block see different active liquidity (after the first crosses a tick).

`project inference` — for the M2.1 ingest pipeline, the SQLite swap-events table needs `(block_number, log_index)` as the ordering key, *not* `block_timestamp` and *not* `block_number` alone.

### 9.10 Decimal scaling at the I/O boundary

Pool quantities are in token-wei units throughout. When the backtester displays USD figures or ETH equivalents, it needs to apply decimal scaling. A common bug: applying decimal scaling *before* fixed-point arithmetic, which loses precision. The correct order:

1. All math in raw wei integers (`BigUint`, `u128`).
2. Convert to display units (USD, ETH) only at the final output stage, with appropriate decimal correction.

The Aurix V3 spot-price decode (`dex/uniswap_v3.rs:76-125`) follows exactly this pattern (`BigUint` until the final `f64` ratio). The same discipline must extend to the backtester: position-value computation in wei units, USD conversion only at the chart layer.

---

## Research Signal

This section translates each source-backed primary-source signal into a concrete Aurix project implication. It is the section the M2.2 / M2.3 implementer reads as a checklist.

| Topic | Source-backed signal | Source citation | Current repository state | Citation (file:line) | Project implication | Evidence class |
|---|---|---|---|---|---|---|
| Q64.96 layout | sqrtPrice fits in `uint160`; `MIN_SQRT_RATIO=4295128739`, `MAX_SQRT_RATIO≈1.46e48` | TickMath.sol [P-TM-1] | `BigUint` decode used; no separate type for sqrtPriceX96 | `dex/uniswap_v3.rs:52-63` | Introduce a `SqrtPriceX96` newtype wrapping `BigUint` to preserve type safety in M2.2 | source-backed |
| Tick→sqrtPriceX96 magic constants | 19 hardcoded constants, rounded-up Q128.128 | TickMath.sol [P-TM-4] | not implemented | n/a | Transcribe the constants exactly; do not regenerate via floating-point | source-backed |
| `MIN_TICK = -887272` | Computed from `log_{1.0001}(2^-128)` | TickMath.sol [P-TM-3] | not implemented | n/a | Use `i32`; check `<= MAX_TICK` not `< MAX_TICK` | source-backed |
| `getLiquidityForAmounts` branches on price-vs-range | 3 cases: below, in, above | LiquidityAmounts.sol [P-LA-2] | not implemented | n/a | Implement same 3-branch structure in Rust | source-backed |
| Per-step fee accumulation | `state.feeGrowthGlobalX128 += mulDiv(feeAmount, Q128, liquidity)` per step | UniswapV3Pool.sol [P-POOL-1] | not implemented | n/a | Implement per-swap, not per-block — see Section 5 | source-backed |
| `feeGrowthOutside` flip on cross | `outside ← global − outside` | Tick.sol [P-TICK-2] | not implemented | n/a | Replicate exactly; treat fee-growth as `u256` modular | source-backed |
| Underflow in fee-growth subtraction | Solidity 0.8 reverts where 0.7 wraps | Code4rena H-04 [P-C4-1] | not implemented | n/a | Use `u256` wrapping arithmetic in Rust port | source-backed |
| V3 IL formula | `2√k / (√p_b/√p_a + √p_a/√p_b) − 1` | Auditless [P-AUDIT-1] | not implemented | n/a | `f64` is fine for IL; `BigUint` for fees | source-backed |
| V3 IL is asymptotic at range edges | Out-of-range IL freezes at boundary | Auditless / dev book [P-DB-3] | not implemented | n/a | Plot must show the elbow, not just a smooth curve | source-backed |
| Tick spacing per fee tier | 1/10/60/200 for 1/5/30/100 bps | RareSkills [P-RS-2] | not implemented | n/a | Make `tickSpacing` a per-pool config, not a constant | source-backed |
| Swap log ordering | Per-protocol per-block ordering by log_index | n/a (protocol convention) | n/a | n/a | Use `(block_number, log_index)` as primary key in M2.1 | project inference |
| Aurix's `BigUint` math stack | `num-bigint` used for sqrtPriceX96 decode | repository | `dex/uniswap_v3.rs:3` | Continue with `num-bigint` for math primitives; introduce `primitive_types::U256` only if `BigUint` performance bites | repository fact |
| Fee model is per-step within swap | Solidity loop accumulates per step | UniswapV3Pool.sol [P-POOL-1] | not implemented | n/a | M2.3 must walk swaps event-by-event, not block-by-block | source-backed |
| Validation needs match-on-chain at-the-wei tolerance for hiring claim | Issue #578 shows tick→price→tick is not bit-exact via `pow` | v3-core #578 [P-ISSUE-578] | n/a | Validate via `slot0()` integer comparison, not via `pow(1.0001, tick)` | source-backed |

---

## What Fits This Project Well

- **`num-bigint::BigUint` for sqrtPriceX96 and intermediate products.** Already in use; matches Solidity's full-precision intent without adding `alloy-primitives`.
- **Per-swap fee distribution.** The plan explicitly names this as the differentiator. The math (Section 4) is exactly as expensive as the protocol's, which means Aurix's off-chain reproduction has the same algorithmic complexity guarantees and produces the same wei-level answers.
- **Three-tier liquidity accounting.** The protocol's `(liquidityGross, liquidityNet, activeLiquidity)` decomposition is necessary for correctness; trying to collapse it to one or two fields would force per-tick-summation costs the backtester doesn't need to pay.
- **`f64` for IL plots and Sharpe.** IL is a relative ratio; Sharpe is bounded; both display at 2–4 decimal places. Mixing `BigUint` (for fees) with `f64` (for ratios and plots) is the right precision discipline.
- **The plan's "test against 5 known on-chain positions" criterion.** Section 9 enumerates the bugs this catches; if the implementation passes, the math is correct on the cases that matter.

## What Fits This Project Badly

- **A `Decimal` library for pool-state arithmetic.** Tempting because IL math reads naturally as decimals. But `Decimal` rounds via `ROUND_HALF_EVEN` by default while the protocol truncates — the GammaStrategies Python port shows the result. Use `BigUint` for state, `f64` for display ratios.
- **Per-block fee aggregation as a primary path.** The plan calls this out as the differentiator; building per-block first and "switching later" tends to mean the validation harness lives off the per-block path forever. Build per-swap from M2.3 day one.
- **A custom 256-bit integer type when `num-bigint` exists.** Resist the urge to write `U256` from scratch as a "purity exercise." `BigUint` is widely tested, well-performing, and the bottleneck is rarely the integer width.
- **Floating-point intermediates inside the Q64.96 path.** Use `f64` only at the very edge (display, IL, Sharpe). Any `f64 → BigUint` round-trip in the inner loop is a precision leak.

## Gap Analysis

| Gap | Severity | Source |
|---|---|---|
| No tick-math implementation in repo | High — blocks M2.2 | repository |
| No fee-growth accounting | High — blocks M2.3 | repository |
| No SQLite schema for swap events | High — blocks M2.1 | repository fact (M2.0 not started) |
| No reorg-safe ingest | Medium — accuracy issue if not handled | plan §M2.1 |
| No test harness | Medium — slows iteration | repository |
| No `MAX_SQRT_RATIO` boundary tests | Medium — common bug source | Section 9.7 |
| No `unchecked` / modular fee-growth handling | High — will panic on underflow | Section 4.7, Section 9.1 |

## Recommended Priority Order (when M2.2 begins)

1. **Implement `tick_to_sqrt_price_x96` with the 19 magic constants exactly as in TickMath.sol.** Test against the three boundaries (MIN_TICK, 0, MAX_TICK) and 5 mid-range values. (Section 2.)
2. **Implement `sqrt_price_x96_to_tick` using the f64-log + verify approach.** Cross-check against the round-trip identity at MIN/MAX. (Section 2.4.)
3. **Implement `liquidity_for_amounts` and `amounts_for_liquidity` with the 3-branch in/below/above structure.** Test against LiquidityAmounts.sol fixture values. (Section 3.)
4. **Implement `getAmount0Delta` / `getAmount1Delta` with explicit `round_up` flag.** Use these as the single primitive in the swap-step inner loop. (Section 1.4 + Section 3.5.)
5. **Implement fee-growth accumulators with `u256` modular wrapping.** Use `primitive_types::U256` if `num-bigint` doesn't have ergonomic wrapping; otherwise wrap manually. (Section 4.7 + Section 9.1.)
6. **Implement the swap-step engine with per-tick crossing.** Read tick-spacing from pool config, not as a constant. (Section 8 + Section 7.)
7. **Build the validation harness against 5 known on-chain positions.** Mismatches here surface most of the bugs in Section 9. (Plan M2.4.)
8. **Defer**: IL plotting (depends on simulation engine working first), benchmark module (M2.7, separate concern), strategy grid (M2.5, depends on engine being correct first).

## Open Uncertainties And Validation Needs

- **Exact rounding direction at every `mulDiv` call site.** Some references give "round up" or "round down" loosely. The single source of truth is Solidity. Recommendation: have the implementer copy the rounding flag from each Solidity call site into a comment in the Rust port.
- **The exact behaviour of `getNextSqrtPriceFromAmount*` near `MIN_SQRT_RATIO` / `MAX_SQRT_RATIO`.** SqrtPriceMath.sol has guard conditions that revert at these boundaries. The off-chain port should mirror the reverts — but the test will be hard to construct without a known boundary swap from mainnet. Worth deferring until the validation harness has a candidate position that comes close.
- **Behaviour of out-of-range positions in the simulation when prices oscillate across the boundary.** The protocol's behaviour is well-defined per swap, but a single block can contain multiple swaps that flip the position in/out. Per-swap fidelity catches this; per-block does not. Worth a dedicated test with a block that contains ≥3 large swaps spanning the position's range.
- **Performance of `BigUint` in the hot loop.** A 30-day backtest is ~250k–500k swaps. Each swap involves several `BigUint` multiplies and divides. Order-of-magnitude estimate: 1ms per swap = 8 minutes per backtest. If this is too slow in practice, consider hybrid `(u128 * u128 → u256)` via `primitive_types::U256` for the hot path.

## Relationship To Existing Context

- This is the first artefact in `context/references/`.
- `context/architecture.md` describes the current implementation reality (Tab 1 only); this paper is forward-looking, governed by `context/plans/vector-a-v3-lp-backtester.md`.
- `context/notes/error-handling.md` and `context/notes/rust-doc-style.md` apply to any Rust code written from this paper. Specifically: each module gets one `thiserror::Error` enum (e.g. `V3MathError`), and every public function gets the four-line `Inputs:/Outputs:/Errors:/Side effects:` rustdoc contract.
- `context/systems/arbitrage-market-data.md` describes the existing V3 decode in `dex/uniswap_v3.rs`. The new math primitives will live in a new module — recommended path `src-tauri/src/dex/v3_math/` — and the existing decode does not need to be changed.
- No prior plans depend on this paper directly other than vector-a-v3-lp-backtester.md.

---

## External Research Trail

**Searches run.**

| # | Query | Tool | Rationale | Sources surfaced |
|---|---|---|---|---|
| 1 | `Uniswap V3 whitepaper tick to sqrtPriceX96 formula section 6.2.2` | WebSearch | Locate canonical tick-conversion formula from primary source | RareSkills, Atis Elsts, v3-core TickMath.sol, blog math primer 1+2 |
| 2 | `Uniswap V3 TickMath.sol getSqrtRatioAtTick magic constants` | WebSearch | Locate the actual Solidity source for the constants | v3-core TickMath.sol (main + d8b1c635 commit), RareSkills, official docs |
| 3 | `Uniswap V3 LiquidityAmounts.sol getLiquidityForAmounts derivation` | WebSearch | Find the primary periphery source + derivation | v3-periphery LiquidityAmounts.sol, dev book, Atis Elsts technical note |
| 4 | `Uniswap V3 feeGrowthInside feeGrowthOutside accounting algorithm explanation` | WebSearch | Source the fee model | Uniswap dev book swap-fees, Bailsec, Tick.sol, Code4rena Sushitrident finding |
| 5 | `Uniswap V3 impermanent loss formula concentrated liquidity range derivation` | WebSearch | Source the V3 IL formula | Auditless / Erins, Speedrun Ethereum, support.uniswap, dev book |
| 6 | `Uniswap V3 tick spacing fee tier 1 10 60 200 constraint` | WebSearch | Confirm the fee-tier / tick-spacing mapping | RareSkills tick-spacing, blog math primer, whitepaper, Tally proposal |
| 7 | `uniswap-python GammaStrategies bug issue feeGrowth tick precision` | WebSearch | Locate documented OSS bugs | GammaStrategies awesome-uniswap-v3, uniswap-python repo, v3-core issue #578 |
| 8 | `uniswap v3 simulator per-swap fee distribution block-level approximation error` | WebSearch | Find criticism of per-block aggregation (contrasting source) | Coinmonks "calculations dead wrong", Code4rena particle finding, HAL academic paper, dev book |
| 9 | `uniswap v3 backtest off-by-one fee accumulation tick crossing rounding bug` | WebSearch | Find specific tick-crossing bugs (additional contrasting evidence) | Zealynx security blog, RareSkills, Tick.sol |

**Sources consulted.**

| URL | Tool | Source class | Key passages quoted below? |
|---|---|---|---|
| [TickMath.sol](https://github.com/Uniswap/v3-core/blob/main/contracts/libraries/TickMath.sol) | WebFetch | strong reference implementation (Solidity contract) | yes — [P-TM-1] [P-TM-2] [P-TM-3] [P-TM-4] |
| [LiquidityAmounts.sol](https://github.com/Uniswap/v3-periphery/blob/main/contracts/libraries/LiquidityAmounts.sol) | WebFetch | strong reference implementation (Solidity contract) | yes — [P-LA-1] [P-LA-2] |
| [SqrtPriceMath.sol](https://github.com/Uniswap/v3-core/blob/main/contracts/libraries/SqrtPriceMath.sol) | WebFetch | strong reference implementation (Solidity contract) | yes — [P-SPM-1] |
| [Tick.sol](https://github.com/Uniswap/v3-core/blob/main/contracts/libraries/Tick.sol) | WebFetch | strong reference implementation (Solidity contract) | yes — [P-TICK-1] [P-TICK-2] |
| [UniswapV3Pool.sol](https://github.com/Uniswap/v3-core/blob/main/contracts/UniswapV3Pool.sol) | WebFetch | strong reference implementation (Solidity contract) | yes — [P-POOL-1] [P-POOL-2] |
| [Atis Elsts liquidity formula](https://atise.medium.com/uniswap-v3-liquidity-formula-explained-de8bd42afc3c) | WebFetch | engineering write-up (technical primer) | partial — [P-ELSTS-1] (formula notation only) |
| [Uniswap blog math primer 2](https://blog.uniswap.org/uniswap-v3-math-primer-2) | WebFetch | official documentation (Uniswap blog) | yes — [P-MP2-1] [P-MP2-2] [P-MP2-3] |
| [RareSkills sqrtPriceX96](https://rareskills.io/post/uniswap-v3-sqrtpricex96) | WebFetch | technical write-up | yes — [P-Q96-1..5] [P-RS-1] |
| [RareSkills tick spacing](https://rareskills.io/post/uniswap-v3-tick-spacing) | WebFetch | technical write-up | yes — [P-RS-2] [P-RS-3] [P-RS-4] |
| [Uniswap dev book — calculating liquidity](https://uniswapv3book.com/milestone_1/calculating-liquidity.html) | WebFetch | reference engineering text (dev book) | yes — [P-DB-1] |
| [Uniswap dev book — swap fees](https://uniswapv3book.com/milestone_5/swap-fees.html) | WebFetch | reference engineering text (dev book) | yes — [P-DB-2] [P-DB-3] [P-DB-4] |
| [Auditless / Erins on V3 IL](https://medium.com/auditless/impermanent-loss-in-uniswap-v3-6c7161d3b445) | WebFetch | technical write-up (Auditless) | yes — [P-AUDIT-1] [P-AUDIT-2] [P-AUDIT-3] |
| [Coinmonks — calculations dead wrong (contrasting)](https://medium.com/coinmonks/all-your-uniswap-v3-liquidity-farming-calculations-are-dead-wrong-heres-why-20bd47f55d69) | WebFetch | **contrasting source** | yes — [P-COIN-1] [P-COIN-2] |
| [v3-core issue #578](https://github.com/Uniswap/v3-core/issues/578) | WebFetch | bug tracker / issue (primary) | yes — [P-ISSUE-578] |
| [Code4rena particle finding H-04](https://github.com/code-423n4/2023-12-particle-findings/issues/10) | WebFetch | audit finding (primary, peer-reviewed-style) | yes — [P-C4-1] |

Source-class coverage: 5 reference implementations (Solidity contracts) + 2 reference engineering texts (dev book) + 2 official documentation pieces (Uniswap blog + RareSkills tick-spacing as derived doc) + 2 technical write-ups (Auditless, Atis Elsts) + 2 bug-tracker primary sources (v3-core issue, Code4rena finding) + 1 contrasting source (Coinmonks). Six distinct source classes — well above the floor of 2.

**Quoted passages.**

- **[P-TM-1]** — TickMath.sol constants
> ```solidity
> uint160 internal constant MIN_SQRT_RATIO = 4295128739;
> uint160 internal constant MAX_SQRT_RATIO = 1461446703485210103287273052203988822378723970342;
> ```

- **[P-TM-2]** — TickMath.sol final rounding
> `sqrtPriceX96 = uint160((ratio >> 32) + (ratio % (1 << 32) == 0 ? 0 : 1));`

- **[P-TM-3]** — TickMath.sol tick boundaries
> ```solidity
> int24 internal constant MIN_TICK = -887272;
> int24 internal constant MAX_TICK = -MIN_TICK;
> ```
> "The minimum tick that may be passed to #getSqrtRatioAtTick computed from log base 1.0001 of 2⁻¹²⁸"

- **[P-TM-4]** — TickMath.sol getSqrtRatioAtTick body, full magic-constant decomposition. (Quoted in full in Section 2.3.)

- **[P-TM-5]** — TickMath.sol getTickAtSqrtRatio doc-page summary
> "Calculates the greatest tick value such that getRatioAtTick(tick) <= ratio [...] using bit-manipulation for MSB detection and iterative logarithm approximation via assembly blocks, concluding with boundary logic selecting between tickLow and tickHi based on ratio comparison."

- **[P-LA-1]** — LiquidityAmounts.sol getAmountsForLiquidity, full body. (Quoted in full in Section 3.2.)

- **[P-LA-2]** — LiquidityAmounts.sol getLiquidityForAmount0/1/Amounts. (Quoted in full in Section 3.3.)

- **[P-SPM-1]** — SqrtPriceMath.sol getAmount0Delta signature
> ```solidity
> function getAmount0Delta(
>     uint160 sqrtRatioAX96,
>     uint160 sqrtRatioBX96,
>     uint128 liquidity,
>     bool roundUp
> ) internal pure returns (uint256 amount0)
> ```

- **[P-TICK-1]** — Tick.sol Info struct
> ```solidity
> struct Info {
>     uint128 liquidityGross;
>     int128 liquidityNet;
>     uint256 feeGrowthOutside0X128;
>     uint256 feeGrowthOutside1X128;
>     int56 tickCumulativeOutside;
>     uint160 secondsPerLiquidityOutsideX128;
>     uint32 secondsOutside;
>     bool initialized;
> }
> ```

- **[P-TICK-2]** — Tick.sol cross fee flip
> ```solidity
> info.feeGrowthOutside0X128 =
>     feeGrowthGlobal0X128 -
>     info.feeGrowthOutside0X128;
> ```

- **[P-POOL-1]** — UniswapV3Pool.sol per-step fee accumulation
> ```solidity
> if (state.liquidity > 0)
>   state.feeGrowthGlobalX128 += FullMath.mulDiv(step.feeAmount,
>     FixedPoint128.Q128, state.liquidity);
> ```

- **[P-POOL-2]** — UniswapV3Pool.sol active-liquidity update on cross
> ```solidity
> int128 liquidityNet = ticks.cross(...);
> if (zeroForOne) liquidityNet = -liquidityNet;
> state.liquidity = LiquidityMath.addDelta(state.liquidity, liquidityNet);
> ```

- **[P-MP2-1]** — Uniswap blog math primer 2, sqrtPriceX96 ↔ tick
> "√P / 2^96 = 1.0001^(i_c)"
> "log(√P / 2^96) / log(1.0001) = i_c"

- **[P-MP2-2]** — Uniswap blog math primer 2, in-range token amounts
> "token_0 = ℓ × (√p_u - √p') / (√p' × √p_u)"
> "token_1 = ℓ × (√p' - √p_l)"

- **[P-MP2-3]** — Uniswap blog math primer 2, fees formula
> "fees_0 = ℓ × (f_r(t_1) - f_r(t_0)) / 2^128"

- **[P-Q96-1]** — RareSkills sqrtPriceX96 conversion
> "sqrtPriceX96 = floor(√p × 2^96)"
> "√p = sqrtPriceX96 / 2^96"
> "p = (√p)²"

- **[P-Q96-2]** — RareSkills bit layout
> "[V3] pack[s] the square root of the price together with the tick and other information in a single 256-bit storage slot, leaving 160 bits for sqrtPriceX96."

- **[P-Q96-3]** — RareSkills max price
> "The largest square root of a price the protocol can work with is approximately 2^64, and the corresponding largest price is slightly below 2^128."

- **[P-Q96-4]** — RareSkills min price
> "The smallest square root of the price...is imposed to be 2^-64."

- **[P-Q96-5]** — RareSkills min fraction
> "a fixed-point value can represent fractions as small as 2^-96"

- **[P-RS-1]** — RareSkills magic constants
> "Each of the 'magic numbers' in `getSqrtRatioAtTick()` are represented as 128-bit fixed point numbers rounded up (except tick 1). These magic numbers correspond to values like √(1.0001^0), √(1.0001^(-2^0)), √(1.0001^(-2^1)), and so on through √(1.0001^(-2^19))."

- **[P-RS-2]** — RareSkills fee/tick-spacing mapping
> "The current relationship between fee and tick spacing is shown in the following table" (1bps→1, 5bps→10, 30bps→60, 100bps→200).

- **[P-RS-3]** — RareSkills tick multiple constraint
> "if the tick spacing of the pool is set to 10, only tick indexes that are multiples of 10 are usable."

- **[P-RS-4]** — RareSkills volatility rationale
> "Highly volatile assets tend to cause higher impermanent loss...thus, LPs will demand higher fees...Highly volatile pairs benefit from wider tick spacing to reduce excessive tick crossings."

- **[P-DB-1]** — Uniswap dev book liquidity calculation
> "L = Δx · √(p_b) · √(p_c) / (√(p_b) − √(p_c))"
> "L = Δy / (√(p_c) − √(p_a))"

- **[P-DB-2]** — Uniswap dev book feeGrowthGlobal definition
> "Each pool has `feeGrowthGlobal0X128` and `feeGrowthGlobal1X128` state variables that track total accumulated fees per unit of liquidity (that is, fee amount divided by the pool's liquidity)."

- **[P-DB-3]** — Uniswap dev book feeGrowthInside formula
> "fr​ = fg​ − fb​(il​) − fa​(iu​)" (i.e. `feeGrowthInside = feeGrowthGlobal − feeGrowthBelow − feeGrowthAbove`)

- **[P-DB-4]** — Uniswap dev book tokensOwed formula
> ```solidity
> uint128 tokensOwed0 = uint128(
>     PRBMath.mulDiv(
>         feeGrowthInside0X128 - self.feeGrowthInside0LastX128,
>         self.liquidity,
>         FixedPoint128.Q128
>     )
> );
> ```

- **[P-AUDIT-1]** — Auditless (Erins) IL formula
> "IL_{a,b}(k) = [2√(k)/(√p_b/√p_a + √p_a/√p_b)] − 1"

- **[P-AUDIT-2]** — Auditless V3→V2 limit
> "IL_{0,+∞}(k) = IL(k), a.k.a., the bigger the price range, the more this equation converges to the impermanent loss equation for V2."

- **[P-AUDIT-3]** — Auditless / dev-book IL amplification
> "Even if the liquidity range is big enough to accommodate prices doubling or halving, impermanent loss is nearly 4 times higher than if providing liquidity in the whole range of prices."

- **[P-COIN-1]** — Coinmonks (contrasting) TVL miscount
> "USDC-WETH pool: Official reported $333m TVL vs. calculated $176m (nearly 50% overstatement)"

- **[P-COIN-2]** — Coinmonks (contrasting) protocol-wide
> "Protocol-wide: Official $11.8b vs. author's calculation $3.14b (approximately 73% overstatement)"

- **[P-ISSUE-578]** — v3-core issue 578
> "math.pow(1.0001, 202475) yields 620780237.5507622"
> "(1974045567390486984838358761822072 * 2^-96)² yields 620804961.6538478"

- **[P-C4-1]** — Code4rena particle finding
> "Solidity 0.8.23 enforces checked arithmetic. When the function performs these subtractions: `feeGrowthInside0X128 = feeGrowthGlobal0X128 - lowerFeeGrowthOutside0X128 - upperFeeGrowthOutside0X128;` The intermediate result can temporarily go negative (due to fee growth wraparound behavior), causing a revert before the mathematically correct final value is reached. Uniswap V3's original implementation intentionally relies on overflow/underflow wrapping to handle this."
> Mitigation: "Use `unchecked` when calculating `feeGrowthInside0X128` and `feeGrowthInside1X128`."

- **[P-ELSTS-1]** — Atis Elsts liquidity formula notation
> "L = √(x · y)" (Eq. 2.1 from the whitepaper)
> "x_offset = L / √(P_b)"
> "y_offset = L · √(P_a)"

## Pre-Completion Obligation Audit

| Obligation | Status | Evidence |
|---|---|---|
| At least 3 distinct WebSearch calls with topic-specific queries | met | 9 distinct WebSearch calls listed in External Research Trail (queries 1–9), each on a different facet of V3 math |
| At least 3 distinct WebFetch calls against primary sources | met | 15 distinct WebFetch URLs listed; 5 are Solidity primary contracts (TickMath, LiquidityAmounts, SqrtPriceMath, Tick, UniswapV3Pool), 2 are bug-tracker primary sources (v3-core #578, code-423n4 H-04), 2 are Uniswap-official documentation (whitepaper PDF attempted but binary; blog math primer 2; dev book) |
| Sources span at least 2 source classes | met | 6 source classes spanned: reference implementations (Solidity), engineering reference texts (dev book), official documentation (Uniswap blog), technical write-ups (RareSkills, Auditless, Atis Elsts), bug tracker / audit findings (v3-core issue #578, Code4rena), contrasting source (Coinmonks) |
| At least 1 direct quoted passage per major source-backed claim | met | Every section of the body that makes a source-backed claim references at least one passage ID from the Quoted Passages table; Section 10 Research Signal table makes the source-class mapping explicit |
| At least 1 contrasting / limiting / disagreeing source consulted | met | Coinmonks "All Your Uniswap V3 Liquidity Farming Calculations Are Dead Wrong" [P-COIN-1, P-COIN-2] argues that even Uniswap's own subgraph mismeasures TVL; Code4rena H-04 [P-C4-1] documents a real revert path; v3-core issue #578 [P-ISSUE-578] documents a precision-loss boundary case. The Coinmonks article is the explicit "the standard approach is wrong" contrasting source. |
| Relevant `context/` files read before project-specific claims | met | `context/architecture.md` (full repo map), `context/notes.md` (notes index), `context/notes/error-handling.md`, `context/notes/rust-doc-style.md`, `context/notes/dex-name-contract.md`, `context/plans/vector-a-v3-lp-backtester.md` (the LP backtester plan that this paper supports). All listed under Relationship To Existing Context. |
| Relevant code inspected (list file paths) | met | `src-tauri/src/dex/uniswap_v3.rs` lines 1–152 (only V3 file in repo); `context/architecture.md` for the full file inventory; the systems and notes folders enumerated above. No other repo files were relevant since the V3 math primitives don't yet exist. |
| `scripts/init_research_artifact.py` run (stdout captured) | met | `Created file scaffold: /Users/atacanercetinkaya/Documents/Programming-Projects/Aurix/context/references/v3-mathematics-deep-dive.md` |
| `scripts/validate_research_artifact.py` run (stdout captured) | met | See Step 13 final report — script run output captured in completion report |

## What I Did Not Do

- **Did not fetch the V3 whitepaper PDF text.** WebFetch returned binary PDF content for `https://app.uniswap.org/whitepaper-v3.pdf` and `atiselsts.github.io/pdfs/uniswap-v3-liquidity-math.pdf`, which the WebFetch backend cannot parse to text. The whitepaper formulas in Section 2 and Section 3 are corroborated through (a) the Solidity source code, which is the executable specification, and (b) the Uniswap blog math primer 2, which restates the whitepaper's section 6.2 in HTML form. The HAL academic paper (`hal.science/hal-04214315/document`) was blocked by an Anubis access-control system; not consulted. The mitigation is the multiple Solidity-source quotations that encode the whitepaper's claims exactly. **Implementer should still skim the whitepaper PDF directly when implementing M2.2 — equations 6.27–6.31 are the canonical statement of the liquidity formulas, and reading them in context is faster than re-deriving from the Solidity source.**
- **Did not trace through the SqrtPriceMath.sol full body.** The fetch returned function signatures and one-line summaries, not the full inner arithmetic of `getNextSqrtPriceFromAmount0RoundingUp` etc. Section 4.6's worked example assumes the protocol's swap-step math is correct; the implementer must read SqrtPriceMath.sol directly when writing the inner-loop primitives and decide rounding direction case by case. The plan's M2.2 acceptance criterion ("validated against fixtures from V3 whitepaper section examples + at least 3 on-chain reference values per function") is the verification surface, and the per-function rounding flag is exactly the thing the on-chain reference value will catch.
- **Did not exhaustively enumerate every OSS V3 library bug.** Section 9 covers the highest-leverage classes (underflow, precision, ordering, decimal-scaling, boundary). Library-specific bugs (e.g. one-off bugs in the various Python ports) are not exhaustively catalogued because the validation harness (M2.4) is the right surface for catching the *Aurix* implementation's own bugs, not for cataloguing OSS history.
- **Did not validate the worked numerical example in Section 3.4 against a live mainnet block.** The example numbers are illustrative; the implementer must verify against an actual `slot0()` snapshot when writing the test fixture. This is called out explicitly in Section 3.4 ("for an actual implementation, write a fixture that reads the live `slot0` of WETH/USDC 5bps at a specific block").
- **Did not benchmark `BigUint` performance vs `primitive_types::U256` for the hot loop.** The "Open Uncertainties" section flags this as a possible follow-up if the M2.4 validation harness shows the simulation is too slow. Premature optimisation is the wrong call here — the correctness criterion is the binding constraint, not throughput.
- **Did not address V4.** V4's hook architecture and singleton-pool model is a separate research target. If Aurix ever migrates to V4, a sibling paper `context/references/v4-hooks.md` is the right vehicle.
