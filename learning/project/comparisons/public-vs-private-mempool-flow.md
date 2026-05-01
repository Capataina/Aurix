# Public vs Private Mempool Flow

## Why This Comparison Matters

Vector B (mempool MEV detector) watches the public mempool. Understanding the difference between public and private mempool flow is essential for (a) calibrating expectations about what Vector B can see, (b) understanding why most arbitrage Aurix observes isn't actionable, and (c) sizing Vector B's hiring-signal value relative to a "real" production MEV bot.

## What Stayed The Same

Both public and private mempool flow:
- Eventually result in transactions included in Ethereum blocks
- Pay gas, follow EIP-1559 fee mechanics
- Are subject to the same EVM execution rules
- Have the same finality once confirmed

The differences are entirely in the visibility, latency, and economic dynamics during the pre-inclusion window.

## What Changed: Visibility

| Property | Public mempool | Private (Flashbots, etc.) |
|---|---|---|
| Who sees the tx pre-inclusion | Every Ethereum node, every MEV bot | Only the relay's searchers and selected builders |
| Sandwich vulnerability | Yes | No (no public visibility means no sandwich) |
| Information leakage | Full calldata visible | None until inclusion |
| Time from broadcast to first observer | ~5-50ms (peer propagation) | Same (relay receives first) |
| Time from broadcast to all observers | ~500-2000ms (P2P propagation) | Never publicly visible |

## What Changed: Latency

| Stage | Public mempool | Private mempool |
|---|---|---|
| Submission to first relay observation | ~10-100ms | ~10-50ms (direct submission) |
| Bundle simulation | N/A | ~5-15ms (relay simulates) |
| Inclusion in block | Next block (~12s) | Next block, atomically with bundle ordering |
| Cancellation possible | Yes (replace by fee) | Sometimes (depends on relay) |

Latency to first inclusion is similar between public and private — the difference is in WHO can see and react to your transaction during the pre-inclusion window.

## What Changed: Economic Dynamics

### Public mempool

- Submitter signals intent to anyone watching
- MEV bots can extract value via sandwich/frontrun/JIT
- Slippage tolerance is the user's main defence
- Failed sandwich attempts still cost the bot gas (the unsuccessful tx pays gas)
- Block builders include high-tip transactions first, but visibility is universal

### Private mempool

- No public visibility — no sandwich risk
- Transactions go directly to a relay, which forwards to builders
- Builders compete to construct the most-profitable block including your tx
- Some private channels (MEV-Share) reveal partial information for backruns but not sandwiches
- Submitter gets sandwich protection in exchange for some MEV being shared with searchers

## How This Affects Aurix's Observations

Aurix doesn't watch the mempool at all currently. When Vector B ships, it'll watch the public mempool. This means Vector B will see:

- Retail user swaps (most retail uses public mempool)
- MEV-bot front-runs and back-runs (the visible portion of MEV)
- Public arbitrage attempts (most retail arbitrage)
- Liquidations (almost all liquidations are public)

It will NOT see:
- Pro arbitrage submitted via Flashbots
- Sandwich bundles (these are submitted privately by definition)
- Private orderflow from sophisticated traders
- Builder-direct submissions

Estimating the visible fraction is itself a research question. Approximate consensus: 30-60% of high-value transactions on Ethereum mainnet flow through private channels. The exact fraction varies by transaction type — almost all retail goes public, almost all sophisticated trading goes private.

## What This Means For Vector B's Signal Value

Vector B's resume bullet should be honest about this:

✅ **"I built a public mempool MEV classifier with sub-10ms p99 latency, validated against historical sandwich attacks on visible flow."**

❌ **"My MEV detector sees all Ethereum MEV in real time."** — false; misses ~50% of MEV that flows privately.

The right framing for hiring signal: Vector B demonstrates **mempool literacy**, the ability to **build sub-10ms latency systems**, and **classification of swap intent from raw calldata**. The completeness-of-coverage limitation is honest about scope.

A "real" production MEV bot would supplement public mempool with:
- Direct relationships with builders (so private bundles become visible at the builder level)
- Multiple geographically-distributed nodes for faster propagation visibility
- Order-flow from specific protocols (DEX backends sometimes share intent before transaction broadcast)
- Statistical inference about private flow (estimating it from gas market dynamics)

These are out of scope for Vector B (and probably out of scope for any portfolio project).

## Why Pros Use Private Channels

Several reasons:

1. **Sandwich protection** — the dominant reason. A pro trader submitting a $1M swap doesn't want a $30K sandwich tax.

2. **Strategic privacy** — for active strategies, revealing your trades early gives competitors information. Private flow keeps strategies opaque.

3. **MEV redistribution** — some private channels (MEV-Share) actively share extracted MEV back with the original submitter, creating a positive-sum relationship.

4. **Bundle atomicity** — Flashbots bundles guarantee atomic execution of multi-tx strategies. Submitting via public mempool can't guarantee ordering or atomicity.

5. **Builder relationships** — sophisticated traders sometimes have direct relationships with specific builders, getting better inclusion guarantees.

## Why Retail Uses Public Mempool

Mostly because the default setup uses public mempool:

- MetaMask defaults to a generic RPC that broadcasts to public mempool
- Setting up Flashbots Protect requires a custom RPC URL configuration
- For small transactions ($100-$1K), sandwich risk is minimal anyway (sandwich economics need $10K+ to be profitable for the bot)

So most retail flow is public not because it's optimal but because it's the path of least resistance.

## How To Tell Public vs Private Flow Apart (Post-Hoc)

Looking at confirmed transactions on Etherscan, you can sometimes infer:

- Tx with priority fee much higher than baseline → likely a public-mempool MEV bot
- Tx that's part of a Flashbots bundle → look for known Flashbots builder addresses
- Tx with a specific gas price exactly matching the block's base fee → likely a private bundle (Flashbots bundles often submit at base fee + 0 tip, with the value paid via a separate `coinbase.transfer` to the builder)

Vector B's classifier could potentially identify these patterns. Combined with mempool watching, you'd have visibility into "what's likely public" vs "what showed up unexpectedly = probably private."

## The Future: Encrypted Mempools

An active research area: cryptographic schemes that hide tx content even from validators until after inclusion. Several proposals:

- **Threshold encryption**: txs encrypted to a committee; decrypted after inclusion
- **FHE-based ordering**: full homomorphic encryption allowing ordering without decryption
- **Commit-reveal schemes**: txs committed in one block, revealed in the next

If any of these deploy, MEV (and Vector B) would change fundamentally. Currently they're research-stage. Worth knowing as the trajectory; not actionable for Aurix today.

## Related Files

- `concepts/domain-patterns/mev-and-transaction-ordering.md` — what MEV is
- `concepts/domain-patterns/the-mempool-public-vs-private.md` — the conceptual coverage
- `concepts/advanced/mempool-mev-detection-mechanics.md` — Vector B's technical depth
- `materials/mev-resources.md` — Flashbots, MEV-Share docs
- `context/plans/vector-b-mev-detector.md` — the implementation plan
