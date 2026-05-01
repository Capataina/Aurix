# Exercise: Sandwich Attack Economics

## Goal

Compute the Sandwich Extractable Value (SEV) for a hypothetical victim swap. Build intuition for what makes a swap sandwichable, what doesn't, and where the bot's profit comes from.

## Estimated Time

45-60 minutes.

## Setup

A victim is about to submit this swap on a Uniswap V3 WETH/USDC pool:

- **Pool state**: roughly equivalent to a V2 pool with reserves `(100 WETH, 300,000 USDC)`. Treat it as V2-style for this exercise (the V3 math is more complex but the principle is identical).
- **Victim's swap**: 50 WETH for USDC
- **Victim's slippage tolerance**: 3% (so `amount_out_min` = 0.97 × expected_out_at_marquee)

A bot has spotted this swap in the public mempool and wants to sandwich it.

## Tasks

### Part 1 — What would the victim get without a sandwich?

- [ ] What's the implied marquee price at the current pool state?
- [ ] If the victim's swap executes alone (no sandwich), what would they receive in USDC?
- [ ] What's the victim's effective execution price?
- [ ] What is `amount_out_min` set to (with 3% tolerance)?

### Part 2 — The bot's front-run

The bot front-runs by buying WETH from the pool (pushing the price UP, making it more expensive for the victim). Suppose the bot buys 10 WETH worth of USDC.

- [ ] What USDC amount must the bot put in to receive 10 WETH? (Apply V2 math)
- [ ] What are the new pool reserves after the bot's front-run?
- [ ] What's the new implied marquee price?

### Part 3 — The victim's swap, post-front-run

Now the victim's 50 WETH swap executes against the bot's modified pool state.

- [ ] What does the victim receive?
- [ ] Did this exceed `amount_out_min`? (If not, the victim's swap reverts and the sandwich fails)
- [ ] What are the new pool reserves after the victim's swap?

### Part 4 — The bot's back-run

The bot now sells the 10 WETH it bought, capturing the inflated USDC price the victim's swap created.

- [ ] What USDC does the bot receive for selling 10 WETH? (Apply V2 math against current state)
- [ ] What are the bot's net USDC flows? (Front-run cost - back-run revenue)
- [ ] What's the bot's gross profit (before gas)?

### Part 5 — Adding gas costs

Assume gas at 30 gwei, ETH at $3,000.

- [ ] Each transaction costs ~150,000 gas. What's the dollar cost of one transaction?
- [ ] The bot submits 2 transactions (front + back). Total gas cost?
- [ ] What's the bot's NET profit (gross - gas)?

### Part 6 — Why this works at this size

- [ ] What was the victim's "missed USDC" (compared to no-sandwich Part 1)?
- [ ] Compare to the bot's gross profit. Where's the difference (LP fees, etc.)?
- [ ] What size of victim swap would make the SEV positive even with gas at 100 gwei?

### Part 7 — Why this DOESN'T work at smaller sizes

Suppose the victim swap was only 1 WETH (not 50).

- [ ] Repeat Parts 1-3 with 1 WETH victim swap
- [ ] What's the bot's gross profit? Net?
- [ ] At what victim size does the sandwich break even after gas?

## Hints

### Hint 1

For each step, you're applying the V2 swap math:
- Adding `Δx` to the pool: `Δy = y - k/(x + Δx)`
- Adding `Δy` to the pool: `Δx = x - k/(y + Δy)` (symmetric)

Track `(x, y, k)` after each transaction and compute the implied price as `y/x`.

Remember the V2 fee: 0.30% of input stays in the pool. For ROUGH calculations you can ignore the fee initially; for precise SEV computation you need to apply it.

### Hint 2

For Part 1 (no sandwich): the victim swap is straightforward. 
- Initial: x=100 WETH, y=300,000 USDC, k=30M, P=3,000
- Victim adds 50 WETH (ignoring fee for now): x=150, y_new = 30M/150 = 200,000
- Victim received: 300,000 - 200,000 = 100,000 USDC
- Effective price: 100,000 / 50 = 2,000 (massive slippage on a 50% pool depth swap)
- Marquee was 3,000; victim got 2,000, that's 33% slippage

The victim's slippage tolerance is 3% which means `amount_out_min` = 0.97 × (50 × 3,000) = 145,500 USDC. The actual receive of 100,000 < 145,500, so the swap REVERTS. Try with a much smaller pool or much smaller victim swap.

### Hint 3

The Part 1 hint reveals the issue — your initial setup of 50 WETH on a 100 WETH pool is too large for the slippage tolerance. Either:
- Use a deeper pool (e.g. 1000 WETH, 3,000,000 USDC)
- Use a smaller victim swap (e.g. 5 WETH on 100 WETH pool)

Let me redo with deeper pool: x=1000 WETH, y=3,000,000 USDC, k=3B, P=3,000.
Victim 50 WETH: x_new = 1050, y_new = 3B/1050 ≈ 2,857,143. Received = 142,857 USDC. Effective price = 2,857. Slippage = 4.76%.

That's still over the 3% tolerance. The victim's swap would revert even without a sandwich. So real sandwich victims either have higher tolerance OR use much smaller swaps relative to pool depth.

Real-world: sandwich victims tend to be users with WIDE slippage tolerances (10-30%) that they may not have realised are wide. Set the victim's tolerance to 10% for the rest of the exercise.

## Expected Behaviour / Self-Check

The math should reveal:

- A sandwich requires victim slippage tolerance LOOSE enough that the bot's front-run + the victim's swap stays within tolerance
- For pools at typical depth, the victim swap needs to be meaningful (>5% of pool depth) for sandwich to be profitable
- Gas costs are a hard floor: at 30 gwei the bot needs gross profit > $9 to break even
- At larger victim sizes (which need larger slippage tolerance), bot profits scale roughly linearly

You'll find that **most sandwiches require victim swaps in the $10K+ range** for the economics to work. This is why retail users with default settings (1% tolerance, small swaps) are mostly safe; large institutional swaps with wide tolerances are the targets.

## What You Should Take Away

- Sandwich economics are about size × tolerance × pool depth
- Tight slippage tolerance is the user's primary defence
- Below ~$10K victim swap size, sandwich economics don't usually work
- Above that size, private orderflow (Flashbots) is the right defence
- The bot's profit comes from value the victim "voluntarily" gave up by setting wide tolerance

## Related Files

- `concepts/domain-patterns/mev-and-transaction-ordering.md` — sandwich attacks in context
- `concepts/domain-patterns/the-mempool-public-vs-private.md` — public visibility is what enables this
- `concepts/advanced/mempool-mev-detection-mechanics.md` — Vector B's classifier
- `context/plans/vector-b-mev-detector.md` — the implementation plan
