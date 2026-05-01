# Exercise: AMM Constant Product By Hand

## Goal

Work through Uniswap V2's `x × y = k` math with concrete numbers. Build intuition for how price emerges from reserves and how slippage emerges from the curve.

## Estimated Time

30-45 minutes with paper and pencil (or a scratch text file).

## Setup

Imagine a Uniswap V2 WETH/USDC pool with these starting reserves:

- 50 WETH (token1)
- 175,000 USDC (token0)

Compute `k` and the implied price (USDC per WETH) at this state.

## Tasks

Work each part on paper. Don't peek at hints until you've genuinely tried.

### Part 1 — Initial state

- [ ] Compute `k` from the reserves
- [ ] Compute the implied marquee price
- [ ] What is the implied marquee price for a swap of 0 WETH (the limit of an infinitely small trade)?

### Part 2 — A small swap

A trader swaps 1 WETH for USDC. Ignoring the fee for now (assume 0% fee):

- [ ] What are the new reserves after the swap?
- [ ] How much USDC did the trader receive?
- [ ] What was the trader's average execution price (USDC per WETH for this trade)?
- [ ] What is the new implied marquee price?
- [ ] What's the slippage as a percentage of marquee?

### Part 3 — A larger swap

Reset to the original reserves (50 WETH, 175,000 USDC, k = 8,750,000). A trader swaps 5 WETH:

- [ ] What are the new reserves?
- [ ] How much USDC did the trader receive?
- [ ] What was the average execution price?
- [ ] What's the slippage as a percentage of marquee?

Compare to Part 2's slippage. How much worse is a 5× larger trade?

### Part 4 — With the 0.30% fee

Reset to the original reserves. Apply the V2 fee — only 99.7% of the input enters the formula:

- [ ] Trader swaps 1 WETH (so 0.997 WETH effectively enters)
- [ ] Compute new reserves accounting for the fee
- [ ] How much USDC did the trader receive?
- [ ] What's the trader's average execution price?
- [ ] How much USDC less did they get vs the no-fee case from Part 2?

That difference is the fee, captured by LPs.

### Part 5 — The reverse direction

Reset to original reserves. A trader swaps 10,000 USDC for WETH (notice: now they're adding to the USDC side, removing from WETH side).

- [ ] Compute new reserves (without fees, then with fees)
- [ ] How much WETH did the trader receive?
- [ ] What was the average execution price (now in USDC per WETH but inverted because they're buying WETH)?

## Hints

### Hint 1

Recall the formula: after any swap, `(new_x) × (new_y) = k` (modulo fees, which slightly increase k over time). To find the new reserves, you know what you ADDED to one side; the formula determines what's REMOVED from the other.

### Hint 2

For a swap of Δx into the pool: `(x + Δx) × (y - Δy) = k`, so `y - Δy = k / (x + Δx)`, so `Δy = y - k/(x + Δx)`.

For Part 4 with the fee, the effective Δx going into the formula is `Δx × 0.997`, but the trader still added the full `Δx` to the pool's x reserve (the 0.3% stays in the pool as fees).

### Hint 3

For Part 3 with 5 WETH:
- `x_new = 50 + 5 = 55`
- `y_new = k / x_new = 8,750,000 / 55 = 159,090.91`
- Δy = 175,000 - 159,090.91 = 15,909.09 USDC
- Average price per WETH = 15,909.09 / 5 = 3,181.82
- Marquee was 175,000/50 = 3,500
- Slippage = (3,500 - 3,181.82) / 3,500 = 9.09%

For Part 2 (1 WETH swap), slippage should be about 1.96%.

A 5× larger trade got ~5× worse slippage — almost exactly proportional in this regime. Slippage grows convexly: bigger trades face disproportionately worse slippage.

## Expected Behaviour / Self-Check

Your answers should land near these (small rounding differences are fine):

- Part 1: k = 8,750,000; marquee = $3,500/WETH
- Part 2: ~3,431.37 USDC received, average price ~$3,431, slippage ~1.96%
- Part 3: ~15,909 USDC received, average price ~$3,182, slippage ~9.09%
- Part 4: ~3,420.99 USDC received (you get ~10 less than Part 2 due to fee)
- Part 5: ~2.78 WETH received

If your numbers are dramatically different, recheck the formula `Δy = y - k/(x + Δx)`.

## What You Should Take Away

- The constant-product invariant is the entire pricing model
- Slippage is geometric, not a separate fee — it falls directly out of the math
- Slippage is convex: bigger trades pay disproportionately more
- The fee is a scalar adjustment that captures value for LPs without changing the price-discovery mechanism

## Related Files

- `concepts/core/amm-mechanics-v2-and-v3.md` — the full theoretical treatment
- `concepts/core/traders-and-slippage.md` — slippage in detail
- `exercises/core/simulate-v2-swap.rs` — implement this in code after you've done it on paper
