# V2 vs V3 AMM Math

## Why This Comparison Matters

Aurix decodes prices from both V2-style pools (Uniswap V2, SushiSwap) and V3-style pools (Uniswap V3 at two fee tiers). The decoding paths look similar at the surface but diverge significantly in math. Understanding the comparison helps you (a) read the dex/ adapters, (b) understand why Vector A picks V3 specifically, and (c) reason about which pool to act on for any given hypothetical use case.

## What Stayed The Same

Both V2 and V3 are AMMs with the constant-product family of math. The CORE invariant `x * y = k` (where x, y are token reserves, k is constant) holds in both. The price implied by the pool comes from the reserve ratio.

Both are accessed via `eth_call` reads against the pool contract. Both have ERC-20 token0 and token1 (lexicographic ordering). Both charge a fee per swap distributed to LPs.

## What Changed

| Property | V2 | V3 |
|---|---|---|
| **Capital coverage** | Spread uniformly $0 to ∞ | LP-chosen `[lower_tick, upper_tick]` |
| **Capital efficiency** | Low | Up to ~4000× higher in tight ranges |
| **State variables** | (reserve0, reserve1) — two integers | sqrtPriceX96 + ticks + per-tick liquidity |
| **Fee tiers** | One per pair (0.30%) | Three per pair (0.05%, 0.30%, 1.00%) |
| **Reading state** | `getReserves()` returns 3 values | `slot0()` returns 7 values, you take the first |
| **Price encoding** | Implicit in reserves; computed via ratio | Explicit `sqrtPriceX96` (Q64.96 fixed-point) |
| **LP positions** | Fungible (ERC-20 LP tokens) | Non-fungible (each is an NFT) |
| **Position math** | Trivial: own X% of pool, get X% of fees and IL | Complex: depends on range and current price |
| **Implementation complexity** | Simple | Significantly more complex |

## V2: Worked Decoding

`dex/uniswap_v2.rs` flow:

```
1. Call factory: getPair(USDC, WETH) → pair address
   selector: 0xe6a43905
   args: USDC address (32-byte padded) + WETH address (32-byte padded)

2. Call pair: token0() → which token is at index 0?
   selector: 0x0dfe1681
   No args.

3. Call pair: getReserves() → (reserve0, reserve1, blockTimestampLast)
   selector: 0x0902f1ac
   No args.

4. Compute price:
   if token0 is USDC: price_usd = (reserve0 / reserve1) × 10^12
   else:              price_usd = (reserve1 / reserve0) × 10^12
```

Three RPC calls. Each is straightforward. The 10^12 scaling adjusts for the decimal asymmetry (USDC has 6 decimals, WETH has 18, so the raw ratio needs scaling by 10^12 to express in equivalent decimal units).

## V3: Worked Decoding

`dex/uniswap_v3.rs` flow:

```
1. Call pool: slot0()
   selector: 0x3850c7bd
   No args.
   Returns: 7 values (sqrtPriceX96, tick, observationIndex, observationCardinality,
                       observationCardinalityNext, feeProtocol, unlocked)
   Aurix takes only the first 32-byte word: sqrtPriceX96.

2. Decode sqrtPriceX96 as BigUint::from_bytes_be.

3. Compute price:
   price_usd = (2^192 × 10^12) / sqrtPriceX96^2
```

One RPC call. The math is more complex (the `(2^192 × 10^12) / sqrtPriceX96^2` expression is the V3-specific way of recovering the WETH/USDC price from the encoded `sqrtPriceX96`).

## Why V3 Encodes Price as sqrtPrice

Two reasons (covered in detail in `concepts/advanced/uniswap-v3-tick-mathematics.md`):

1. **Computational efficiency**: many V3 swap math operations involve square roots; pre-storing `√P` skips per-swap square roots.

2. **Liquidity unit consistency**: V3's "liquidity" `L` has the property `L = √(x × y)` — the geometric mean of reserves. Storing prices in `√P` units makes the liquidity math clean.

The `× 2^96` scaling provides 96 bits of fractional precision in an integer representation (Q64.96 fixed-point). Without it, you'd lose precision quickly because integer division truncates.

## Why V3 Is Harder For LP Backtesting

V2 LP simulation is straightforward:

- Position size = % of pool
- Fees per swap = % × swap_fee
- IL formula is closed-form (single equation in r = current_price / entry_price)

V3 LP simulation has none of these properties:

- Position is active only inside `[lower_tick, upper_tick]` — outside, it earns nothing
- Per-swap fee depends on the LP's share of in-range liquidity at the moment of the swap, which changes as ticks are crossed
- IL is path-dependent and significantly worse than V2 in tight ranges

Vector A's plan is exactly this: implement V3 LP simulation correctly with per-swap fee distribution and exact tick-aware IL. Validation against on-chain reference positions is the only way to know your implementation is correct.

A V2 LP backtester would be a much smaller project — probably 1 week of work vs Vector A's 4-6 weeks. But V2 LPing is largely uninteresting compared to V3 (lower capital efficiency, worse fee/IL trade-offs in normal markets).

## Why Both Pools Exist Even Though V3 Is "Better"

V3 launched in May 2021. V2 launched in May 2020. The natural question: why does V2 still have meaningful liquidity?

Several reasons:
- **Long-tail tokens** still primarily live on V2 because V3 deployment requires more tooling
- **Set-and-forget LPs** prefer V2's simpler model — no range management
- **MEV bots** sometimes target V2 specifically because the simpler math means simpler arbitrage routes
- **SushiSwap (a V2-fork)** never matched V3's adoption but retained legacy liquidity

For WETH/USDC specifically, V3 5bps has the deepest liquidity by a large margin. V2 and Sushi exist but are smaller. The price differences Aurix observes between V3 and V2/Sushi reflect this depth difference — V3 stays close to "true" price, V2/Sushi drift further before arbitrage closes the gap.

## What To Learn From The V2/V3 Coexistence

A few teaching points:

1. **"Better" depends on use case.** V3 is more capital-efficient and offers more fee control. But V2 is simpler, more accessible, and adequate for set-and-forget LPing.

2. **Migration takes years.** Even with V3 dominating in volume, V2 has billions in TVL three years after V3 launched. Liquidity is sticky.

3. **Forks proliferate.** SushiSwap forked V2 with a few changes (governance token, slightly different fee distribution). V3-style AMMs have many forks (PancakeSwap V3, Camelot, etc.). Each fork has slightly different math but the core invariants are similar.

4. **Aurix watches both deliberately.** The price comparison V3 vs V2 reveals depth differences and arbitrage equilibrium dynamics that you wouldn't see by watching V3 alone.

## How This Comparison Maps to Aurix's Code

| Code path | Lines | Complexity |
|---|---|---|
| `dex/uniswap_v2.rs` | ~180 | Moderate (3 RPC calls + decimal-aware ratio) |
| `dex/uniswap_v3.rs` | ~130 | Lower line count, but BigUint math is more substantial than V2's reserves division |

Despite V3 being "more complex" mathematically, the decoding code is comparable in size because V3 needs only one RPC call (V2 needs three). The complexity is concentrated in different places: V3 in the decode math, V2 in the multi-RPC orchestration.

## Related Files

- `concepts/core/amm-mechanics-v2-and-v3.md` — the conceptual foundation
- `concepts/advanced/uniswap-v3-tick-mathematics.md` — the V3 deep math
- `context/systems/arbitrage-market-data.md` — the implementation truth
- `materials/amm-foundational-resources.md` — V2 and V3 whitepapers
- `context/plans/vector-a-v3-lp-backtester.md` — the V3-specific backtester plan
