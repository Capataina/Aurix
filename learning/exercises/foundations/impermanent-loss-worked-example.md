# Exercise: Impermanent Loss Worked Example

## Goal

Track a Uniswap V2 LP position through a price swing. Compute exact impermanent loss vs the hold-only baseline. Build intuition for why IL is "impermanent" and when it crystallises.

## Estimated Time

45-60 minutes with paper and pencil.

## Setup

You enter a V2 WETH/USDC pool when the price is $2,500 per WETH. To keep math simple, assume YOU ARE THE ENTIRE POOL (so the math works exactly without proportional scaling).

Initial deposit: **2 WETH + 5,000 USDC**.

- Initial WETH value: 2 × $2,500 = $5,000
- Initial USDC value: $5,000
- **Total initial value: $10,000**

## Tasks

### Part 1 — Sanity check

- [ ] Compute `k` for the initial pool state
- [ ] Confirm the implied price is $2,500/WETH

### Part 2 — Price moves UP to $4,000/WETH

External arbitrage has moved the price. Your pool's reserves rebalance to reflect the new price.

- [ ] What's the new ratio constraint? (You need x × y = k AND y/x = 4,000)
- [ ] Solve for the new x (WETH reserves) and y (USDC reserves)
- [ ] What's the dollar value of your position now? (At the new price of $4,000/WETH)

### Part 3 — Hold-only baseline

If you'd just held the original 2 WETH + 5,000 USDC without LPing:

- [ ] What's the dollar value of "just holding" at the new price of $4,000/WETH?
- [ ] What's the impermanent loss? (LP value - hold-only value)
- [ ] What's the IL as a percentage of the LP position?

### Part 4 — Price moves further UP to $5,000/WETH

From the new pool state in Part 2, the price moves further. Repeat the calculations:

- [ ] What are the new reserves?
- [ ] What's the LP value at the new price?
- [ ] What's the hold-only value?
- [ ] What's the IL?

Did the IL grow? By roughly how much per dollar of price move?

### Part 5 — Price returns to $2,500

From the state in Part 4, an arbitrageur reverses the price back to $2,500.

- [ ] What are the reserves now?
- [ ] How does this compare to the original (2 WETH, 5,000 USDC)?
- [ ] What's the LP value?
- [ ] What's the hold-only value?
- [ ] What's the IL now?

This is the "impermanent" part. If you withdraw at this point, IL is zero (or near-zero).

### Part 6 — Compare IL between Parts 2, 4, and 5

Make a small table:

| Price | LP Value | Hold-only Value | IL ($) | IL (%) |
|---|---|---|---|---|
| $2,500 (start) | $10,000 | $10,000 | $0 | 0% |
| $4,000 (Part 2) | ? | ? | ? | ? |
| $5,000 (Part 4) | ? | ? | ? | ? |
| $2,500 (Part 5) | ? | ? | ? | ? |

What pattern do you see?

### Part 7 — Volatility vs trend

Suppose instead of a one-way price move, the price oscillated:
$2,500 → $3,000 → $2,000 → $3,000 → $2,500 (round trip).

For each of these stages, the pool's math has been rebalancing your position. At the end:

- [ ] How does the pool composition compare to the original?
- [ ] In a *zero fee* world, what's the IL after the round trip?
- [ ] If we add the fee — say the pool collected $200 in fees during all this trading — what's your net outcome vs hold-only?

## Hints

### Hint 1

For a V2 pool with `x × y = k`, given a target price `P`, the new reserves are:
- `x = √(k/P)`
- `y = √(k × P)`

Check: `y/x = √(P) × √(P) = P ✓` and `x × y = √(k/P) × √(k × P) = k ✓`.

### Hint 2

For Part 2 (price = $4,000):
- k = 2 × 5,000 = 10,000
- x = √(10,000 / 4,000) = √2.5 ≈ 1.581 WETH
- y = √(10,000 × 4,000) = √40,000,000 ≈ 6,325 USDC
- LP value at $4,000: 1.581 × 4,000 + 6,325 ≈ $6,324 + $6,325 ≈ **$12,649**
- Hold-only at $4,000: 2 × 4,000 + 5,000 = **$13,000**
- IL = $12,649 - $13,000 = **-$351 (about -2.7%)**

### Hint 3

The closed-form IL formula:

> **IL = (2√r) / (1 + r) - 1**

where r = current_price / entry_price.

For r = 4000/2500 = 1.6:
- IL = (2 × √1.6) / 2.6 - 1
- IL = 2.530 / 2.6 - 1
- IL = 0.973 - 1 = **-2.7%** ✓

This matches the worked Part 2 calculation.

For Part 4 (r = 5000/2500 = 2.0):
- IL = (2 × √2) / 3 - 1
- IL ≈ 0.943 - 1 = **-5.7%**

For Part 5 (r = 1.0, price returned to entry):
- IL = (2 × 1) / 2 - 1 = **0%** (impermanent loss reverted)

## Expected Behaviour / Self-Check

The big insights:

1. **IL is symmetric in log-price**: doubling and halving produce the same magnitude IL
2. **IL is non-linear**: small moves → small IL; large moves → much-bigger-than-proportional IL
3. **IL reverts when price reverts**: hence "impermanent"
4. **Volatility around a fixed mean** generates fees but minimal net IL — this is when LPing wins

For the round-trip in Part 7: if the price genuinely returns to entry and there are fees, you net WIN by being an LP (you collected fees, IL is zero). This is the LP's bet — fees > IL.

## What You Should Take Away

- IL is a geometric consequence of the AMM curve, not a "tax" or "loss" anyone collects
- IL is opportunity cost vs holding, not absolute loss in dollars
- LPs win when fees > IL; lose otherwise
- High-volume range-bound markets are good for LPs; trending markets are bad
- "Impermanent" really means impermanent — IL goes to zero if price returns to entry

## Related Files

- `concepts/core/liquidity-providers-and-impermanent-loss.md` — the theoretical treatment
- `concepts/core/amm-mechanics-v2-and-v3.md` — the underlying AMM math
- `context/plans/vector-a-v3-lp-backtester.md` — the project that simulates this for V3 LPs
