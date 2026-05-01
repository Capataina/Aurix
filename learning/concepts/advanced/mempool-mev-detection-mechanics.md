# Mempool MEV Detection Mechanics

## Why This Matters Here

Vector B (mempool MEV detector) is the most technically demanding of the three vectors in terms of distributed-systems literacy. This file is the prerequisite for actually building Vector B — the WebSocket subscription mechanics, the calldata decoding, the local pool simulation, and the classification heuristics. Read this before you read `context/plans/vector-b-mev-detector.md`.

## Prerequisites

- `concepts/domain-patterns/mev-and-transaction-ordering.md` (you should know what MEV is and why it matters)
- `concepts/domain-patterns/the-mempool-public-vs-private.md` (you should know what the mempool is)

## Status

Foundational domain knowledge. Not yet implemented in Aurix — Vector B's plan is to implement this.

## The Detection Pipeline

A mempool MEV detector has four stages, each with its own technical challenges:

```
┌──────────────────┐    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│ 1. Subscription  │ -> │ 2. Decoding  │ -> │ 3. Simulation│ -> │ 4. Classify  │
│  (WebSocket)     │    │  (calldata)  │    │  (pool state)│    │  (heuristics)│
└──────────────────┘    └──────────────┘    └──────────────┘    └──────────────┘
   ~5-100ms latency        ~1-5ms              ~1-10ms              ~0.1-1ms
```

Each stage adds latency. The total budget for "tx detected → classification rendered" is bounded by the next block — once the tx lands, classifying it is post-hoc rather than predictive. So the whole pipeline targets sub-100ms (achievable on free public RPC) or sub-10ms (requires self-hosted node).

## Stage 1: Subscription

The Ethereum JSON-RPC method:

```
eth_subscribe newPendingTransactions
```

This is a WebSocket subscription. Establishing it:

```
WebSocket connection to wss://eth.alchemy.com/...
Send: {"jsonrpc": "2.0", "id": 1, "method": "eth_subscribe", "params": ["newPendingTransactions"]}
Receive: {"jsonrpc": "2.0", "id": 1, "result": "0x9cef478923ff08bf67fde6c64013158d"}  // subscription ID
```

After that, the node pushes notifications as new pending transactions arrive:

```
Receive: {"jsonrpc": "2.0", "method": "eth_subscription", "params": {"subscription": "0x9cef...", "result": "0xabc...123"}}  // tx hash
```

This is hash-only. To get the full tx body, you call `eth_getTransactionByHash(0xabc...123)`. Some providers (Alchemy in particular) offer `newPendingTransactionsWithBody` which pushes full bodies, saving the round-trip.

### Latency Considerations

- Self-hosted Geth/Erigon node: ~5-15ms tx-broadcast → your node sees it
- Alchemy free tier: ~50-200ms (depends on region)
- Premium RPC providers: ~10-50ms
- Direct peering with major builders: ~0-10ms (you're effectively in the same network as the source)

For Vector B's MVP, free Alchemy tier is fine. For "competitive" detection, self-host.

### Reconnection Logic

WebSocket connections drop. The detector needs:

- Automatic reconnection with exponential backoff (start at 1s, max 30s)
- Re-subscription on reconnect (subscriptions don't survive disconnect)
- Idempotency: process each tx hash exactly once (deduplicate within a sliding window)

## Stage 2: Calldata Decoding

A pending transaction has these fields (relevant ones):

- `to` — target contract address
- `input` (calldata) — the bytes the EVM will execute
- `value` — ETH sent with the tx
- `gas` — gas limit
- `maxPriorityFeePerGas`, `maxFeePerGas` — EIP-1559 fee parameters

The first 4 bytes of `input` are the **function selector** — the first 4 bytes of `keccak256(function_signature)`. This identifies which function is being called.

Common selectors for swap routers:

| Selector | Function | Router |
|---|---|---|
| `0x38ed1739` | `swapExactTokensForTokens(uint256,uint256,address[],address,uint256)` | Uniswap V2 Router 02 |
| `0x18cbafe5` | `swapExactETHForTokens(uint256,address[],address,uint256)` | Uniswap V2 Router 02 |
| `0x414bf389` | `exactInputSingle((address,address,uint24,address,uint256,uint256,uint256,uint160))` | Uniswap V3 SwapRouter |
| `0xc04b8d59` | `exactInput((bytes,address,uint256,uint256,uint256))` | Uniswap V3 SwapRouter |
| `0x3593564c` | `execute(bytes,bytes[],uint256)` | Uniswap Universal Router |

Each router has its own ABI for the function parameters. The Uniswap V2 swap functions take (amount_in, amount_out_min, path, recipient, deadline). The V3 single-pool swaps take a struct with token0/token1/fee/etc. The Universal Router uses a command-stream encoding where the first parameter is a sequence of opcodes describing the multi-step operation.

Decoding requires:

1. Switching on the selector
2. Decoding the parameters per the function's ABI
3. Extracting the relevant fields (input/output tokens, amounts, slippage tolerance, recipient)

For Vector B, you'd build a decoder for at least: V2 Router (8 variants), V3 SwapRouter and SwapRouter02 (4-8 variants each), SushiSwap router (V2-fork, same ABI as V2), Universal Router, and 1inch (V5 and V6 have different ABIs).

The output is a structured `SwapIntent`:

```
SwapIntent {
    router: V2 | V3 | UniversalRouter | OneInch,
    input_token: Address,
    output_token: Address,
    amount_in: U256 (or "exact_out" if amount_out is fixed),
    amount_out_min: U256 (the slippage tolerance lower bound),
    recipient: Address,
    deadline: u64,
    pool_address: Option<Address>,  // Some V3 swaps target specific pools
}
```

### Unrecognised Transactions

Not every pending tx is a swap. Some are token transfers, contract deployments, NFT mints, governance votes, etc. The decoder should:

- Quickly identify "not a swap" via selector matching
- Tag those as `OTHER` and discard
- Not waste budget on full decoding for non-swaps

A good heuristic: maintain a list of ~50-100 known router addresses; only attempt swap decoding on txs whose `to` matches one of these. This filters out >95% of non-swap traffic immediately.

## Stage 3: Pool State Simulation

For each decoded swap intent, you want to predict its execution price. To do this, you need:

1. The current pool state (reserves for V2, sqrtPriceX96 + tick state for V3)
2. The swap math for that protocol
3. Apply the swap to the cached state, derive the new state and the trader's outcome

### V2 Simulation

Given pool with reserves (x, y) and a swap of amount `Δx` of token0 in:

```
Δy = (y × Δx × 0.997) / (x + Δx × 0.997)  # 0.997 is the 1 - 0.003 fee factor
new_state = (x + Δx × 0.997, y - Δy)
trader_received = Δy
```

Straightforward. Cache the pool state, refresh every block via `getReserves()`, apply the formula on each pending swap.

### V3 Simulation

Significantly more complex. A V3 swap walks through ticks as it executes:

```
remaining_in = amount_in
total_out = 0
current_sqrt_price = pool.sqrtPriceX96
current_tick = pool.tick

while remaining_in > 0:
    # Find next tick boundary in swap direction
    next_tick = next_initialised_tick(current_tick, swap_direction)
    next_sqrt_price = tick_to_sqrt_price(next_tick)
    
    # How much can we swap before crossing next_tick?
    available_in_segment = compute_swap_to_tick(current_sqrt_price, next_sqrt_price, pool.liquidity)
    
    if remaining_in <= available_in_segment:
        # Whole swap fits within current tick range
        out = compute_amount_out(current_sqrt_price, ?, pool.liquidity, remaining_in)
        total_out += out
        remaining_in = 0
    else:
        # Swap exhausts current segment; cross into next tick
        out_at_segment = compute_amount_out_to_tick(current_sqrt_price, next_sqrt_price, pool.liquidity)
        total_out += out_at_segment
        remaining_in -= available_in_segment
        # Cross the tick: add/subtract the tick's net liquidity
        pool.liquidity += pool.tick_data[next_tick].liquidity_net  
        current_sqrt_price = next_sqrt_price
        current_tick = next_tick
```

Implementing this correctly is a large fraction of Vector B's work. Reusing parts of `Vector A`'s tick math is natural — both vectors need this code.

For Vector B's purposes, you can simplify by ignoring tick crossings for small swaps (most pending swaps are small enough that they don't cross ticks). The simplification produces ~95% accurate predictions and is much faster. The simulation only needs to be approximate to drive classification — an exact simulation isn't required.

## Stage 4: Classification

Once you've decoded the swap intent and predicted its execution price, you classify the tx into one of several categories:

### Sandwichable (target candidate)

A swap is sandwichable if:
- Its `amount_out_min` allows for meaningful slippage (e.g. `actual_expected_out * 0.95 > amount_out_min`)
- Its size is large enough that sandwich profit > sandwich gas (typically: >$10K trade size on a typical pool)
- The pool has enough depth that the sandwich's own front-run won't move the price too far

### Sandwich Execution (the bot's bundle)

Detect tx groups that look like already-deployed sandwiches:
- Same sender's two txs (front-run + back-run) bracketing a victim
- Both bot txs in the same block, with the victim sandwiched between them
- The bot's two txs are mirror swaps (token0→token1 then token1→token0)

This requires looking at confirmed blocks (post-hoc), not just mempool. Many MEV detectors maintain a "block scanner" alongside the mempool watcher to identify which sandwiches actually executed.

### Frontrunning

A pending tx with a much-higher-than-baseline priority fee, often submitted shortly after another tx that creates an opportunity (oracle update, large user swap, liquidatable position).

Heuristic: priority fee > 95th percentile of recent priority fees.

### JIT Liquidity

A mint LP position immediately followed (in the same or adjacent blocks) by a burn of the same position. Look for:
- Mint tx with liquidity in a tight range (~10-50 ticks wide)
- Burn tx of the same NFT position 1-2 blocks later
- A large swap between the mint and burn that touched the LP's range

JIT detection requires both mempool and confirmed-block visibility.

### Liquidation

A call to a known liquidation function on Aave, Compound, Maker, or similar lending protocols. These have well-defined ABIs and are easy to identify from the function selector alone.

### Normal Swap

Everything else — the residual class. Most swaps are normal swaps. The classifier defaults to this when no other category fits.

## Sandwich Extractable Value (SEV)

For sandwichable swaps, compute the maximum profit a sandwich would extract.

The math:

1. **Optimal front-run size**: maximises (back_run_value - front_run_cost - 2*gas)
2. **Front-run cost**: amount of token swapped in × current price
3. **Back-run value**: amount of token swapped out × inflated price
4. **Net SEV**: back_run_value - front_run_cost - gas_cost_both_txs

Iterating to find optimal front-run size is a 1D optimisation — bounded above by the victim's slippage tolerance. The Flashbots `eth_callBundle` simulation can verify the SEV by actually running the bundle locally.

## Latency Targets

For Vector B's hiring-signal value:

| Tier | Pipeline latency p99 | Achievable with |
|---|---|---|
| Educational | <500ms | Free Alchemy + naive implementation |
| Competent | <100ms | Free Alchemy + optimised pipeline (cached state, parallelism) |
| Strong | <30ms | Premium RPC + optimised pipeline |
| Pro | <10ms | Self-hosted Geth/Erigon + colocation + heavily optimised |

Each tier signals different levels of seriousness. <100ms is the threshold where the resume bullet "sub-100ms p99 mempool MEV classification" becomes credible. <10ms is the threshold where "competitive with production MEV bots" becomes credible.

## Common Misunderstandings

❌ **"Decoding calldata is just ABI parsing."** It is in principle, but the surface area is large (5-10 routers, 30+ function variants, Universal Router's command-stream encoding adds complexity). Robust decoders take weeks to build.

❌ **"Pool state can be read fresh per swap."** Reading state per swap adds RPC round-trips (50-200ms each). For sub-100ms pipeline, you must cache pool state and refresh per block, not per swap.

❌ **"Classification is easy with rules."** Rules get you to 70-80% accuracy. Higher accuracy requires ML — exactly the V2 of Vector B (ML-based classifier sharing infrastructure with Vector C).

❌ **"You can detect MEV by watching confirmed blocks."** You can detect MEV that already executed by watching blocks. You can't detect MEV in flight that way. Mempool watching is what gives you predictive signal.

❌ **"All MEV happens in the public mempool."** A growing fraction of MEV happens in private orderflow (Flashbots Protect, MEV-Share, builder-direct submissions). Public mempool watching only sees the visible portion. Estimating the private fraction is itself an active research area.

## Related Files

- `concepts/domain-patterns/mev-and-transaction-ordering.md` — what MEV strategies exist
- `concepts/domain-patterns/the-mempool-public-vs-private.md` — the mempool itself
- `materials/mev-resources.md` — Flashbots, libMEV, Eigenphi
- `materials/ethereum-internals-resources.md` — for the EVM details
- `context/plans/vector-b-mev-detector.md` — the implementation plan
