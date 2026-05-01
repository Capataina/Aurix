# The Mempool: Public vs Private

## Why This Matters Here

The mempool is the staging area for transactions before they're included in blocks. Understanding what's in the mempool, who can see it, and the difference between "public" and "private" mempools is essential to understanding MEV, why pro traders bypass the public path, and what Vector B (the mempool watcher) would actually be subscribing to.

## Prerequisites

- `concepts/domain-patterns/gas-and-execution-costs.md` (you should understand block construction and priority fees)
- `concepts/domain-patterns/mev-and-transaction-ordering.md` (you should understand why ordering matters)

## What the Mempool Is

When you submit an Ethereum transaction (via MetaMask, your own wallet, or any RPC node), it enters the **mempool** — a queue of pending transactions waiting to be included in a block. The transaction sits in the mempool for up to ~12 seconds (one block time), and then either:

- Gets included in the next block (most common case)
- Remains pending if the gas price is too low or the network is too congested
- Gets dropped if it sits too long (typically after a few hours)

The mempool isn't a single global queue. **Each Ethereum node has its own view of the mempool**, slightly different from every other node's. When you submit a transaction, it propagates through the peer-to-peer network — your node tells its peers, who tell their peers, and so on. This propagation takes hundreds of milliseconds. Within a few seconds, most of the network has the same view, but at any specific instant the views differ.

Block builders choose from their local view of the mempool when constructing the next block. The transactions they pick are generally those paying the highest priority fees, but the algorithm is up to the builder — they can include any subset in any order.

## Public Mempool

The "default" mempool is the public one — every Ethereum node propagates pending transactions to its peers via the standard P2P protocol. Anyone running a node (including arbitrage bots) sees the mempool as it propagates.

When you submit a tx via MetaMask using a generic RPC (Infura, Alchemy free tier), your tx enters the public mempool. The moment it does:

- Every other node knows about it (within a few seconds)
- Every MEV bot watching the mempool knows about it
- It can be sandwiched, front-run, or copied

The public mempool is the substrate for MEV. Sandwich attacks specifically depend on the bot seeing your tx before it's included — if the tx were invisible until inclusion, sandwich attacks would be impossible.

## Private Mempool / Private Orderflow

A "private mempool" is any submission channel that doesn't propagate transactions through the public P2P network. Several flavours exist:

### Flashbots Protect

The original and largest private orderflow channel. You submit your transaction directly to Flashbots Auction, which forwards it to a network of trusted builders who include it in blocks. The transaction is invisible to the public mempool.

For users: free for normal swaps; protects against sandwich attacks. Most reputable wallets (MetaMask, Rabby, Frame) offer Flashbots as a submission option.

For builders: receives high-quality flow that's competitive against public mempool flow.

### MEV-Share

A more recent Flashbots product that provides controlled visibility — your transaction is partially revealed to searchers (e.g. "WETH→USDC swap of size X") so they can construct backruns, but not enough information to construct sandwiches. The user shares some MEV but not the catastrophic kind.

### Builder-Specific RPCs

Most major builders (Flashbots, BloXroute, Eden, Beaverbuild) accept direct submissions to their RPC endpoint. Submitting this way is similar to Flashbots Protect — your tx goes to the builder, not to the public mempool.

### MEV-Boost

The protocol-level mechanism that connects validators to a market of builders. A validator running MEV-Boost listens to bids from multiple builders for the next block; the highest bidder's block is the one the validator proposes. This is what gave rise to the Builder/Searcher/Validator separation.

## Watching the Public Mempool

For a tool like Vector B that needs to see pending transactions, the standard interface is the JSON-RPC subscription:

```
eth_subscribe newPendingTransactions
```

This is a WebSocket subscription. The node pushes you transaction hashes the moment new pending transactions arrive in its mempool. You then call `eth_getTransactionByHash` for each to fetch the full transaction body (calldata, value, gas, etc.).

There are three flavours of this subscription:

| Subscription | What you get | Latency |
|---|---|---|
| `newPendingTransactions` | Hashes only | Lowest |
| `newPendingTransactionsWithBody` (Alchemy custom) | Full bodies | Slightly higher (extra data per push) |
| `pendingTransactions` (filter-based, older API) | Filtered subset | Variable |

The trade-off is between latency (hashes-first means an extra round-trip to fetch the body) and bandwidth (bodies-included means more data per push). For Vector B's purposes, either works.

## Latency Matters

The reason MEV bots care about latency:

- An opportunity becomes visible when a transaction enters the public mempool
- Bots have ~12 seconds (one block time) before that transaction lands in a block
- During those 12 seconds, the bot must: see the tx, decode its intent, simulate its effect, decide whether/how to react, construct a bundle, and submit the bundle to a builder

For competitive opportunities (sandwich attacks on large swaps), this whole pipeline needs to happen in milliseconds — multiple bots are racing to be first. The bot that's slowest doesn't get the opportunity; the fastest one wins.

This is why infrastructure matters:

- **Self-hosted nodes** (Geth, Erigon) on dedicated hardware: ~5-15ms from tx-broadcast to your node seeing it
- **Co-located with major peers**: even faster — you're geographically close to where transactions originate
- **Direct relationships with searchers/builders**: bypass the P2P propagation entirely

Retail bots using free public RPCs (Alchemy free tier, Infura free tier) typically see transactions hundreds of milliseconds late. By the time they react, pro bots have already won the race.

## What Aurix Doesn't Do (And Vector B Would)

Aurix currently doesn't watch the mempool at all. It uses `eth_call` (not subscriptions) to read pool state every 1 second. There's no mempool visibility, no MEV awareness, no understanding of pending transactions.

Vector B's plan is the entire mempool layer:

1. WebSocket subscription to `eth_subscribe newPendingTransactions`
2. Decode each pending tx's calldata to identify swap intent
3. Locally simulate the swap against current pool state to predict price impact
4. Classify the tx (normal swap, sandwich attempt, JIT liquidity, frontrun, liquidation)
5. Compute extractable value for sandwichable swaps
6. Surface all of this in a live feed with sub-10ms classification latency

Aurix would still never execute — the read-only principle is non-negotiable — but it would see the MEV ecosystem in motion.

## Common Misunderstandings

❌ **"The mempool is sorted by gas price."** It's not literally sorted — it's a set of pending transactions. Block builders SELECT in priority order, but the mempool itself isn't an ordered queue. Different builders can pick different transactions in different orders.

❌ **"Once a transaction is in the mempool, it's guaranteed to land."** No. If gas spikes after you submit, your transaction might wait for hours; if gas spikes hard enough, it might get dropped. You can also "cancel" by submitting a 0-gas transaction with the same nonce (which replaces the original).

❌ **"Private orderflow is always better than public."** Private orderflow protects against sandwiches but introduces other trade-offs:
- Slightly higher latency to first inclusion
- Trust in the relay/builder
- Some private channels show your tx to searchers (MEV-Share) for back-running, which leaks some value
- For SMALL transactions, public mempool is fine (sandwich economics don't work below ~$10K trade size)

❌ **"You can see all of mempool by querying any node."** You can see your node's view, which is *most* of mempool but not all. To see ~100% of mempool, you'd need to peer with many geographically-diverse nodes and union their views.

❌ **"Transactions in the mempool are encrypted."** No. Standard transactions in the public mempool are fully visible — calldata, value, signer, gas. Encryption is an active research area ("encrypted mempools," "FHE-based ordering") but not deployed today.

## Related Files

- `concepts/domain-patterns/mev-and-transaction-ordering.md` — what people DO with mempool visibility
- `concepts/advanced/mempool-mev-detection-mechanics.md` — the technical depth for Vector B
- `concepts/domain-patterns/gas-and-execution-costs.md` — priority fee market and inclusion economics
- `materials/mev-resources.md` — Flashbots docs, MEV ecosystem resources
- `context/plans/vector-b-mev-detector.md` — the implementation plan
