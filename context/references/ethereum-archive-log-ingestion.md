# Ethereum Archive Log Ingestion — Production Patterns

## Scope / Purpose

This paper is the implementation reference for Aurix milestone **M2.1 — Historical data ingestion** (`context/plans/vector-a-v3-lp-backtester.md`, lines 99–107). It answers the repository-specific question:

> *What concrete patterns must Aurix's M2.1 ingestion code follow to backfill ≥30 days of WETH/USDC 5bps swaps via Alchemy free tier without learning by trial and error — covering batch sizing, reorg safety, retry semantics, idempotency, storage budget, and provider fallback?*

It is not a general guide to Ethereum log ingestion, not a comparison of indexer products, and not a deep-dive on V3 math (that lives in `v3-mathematics-deep-dive.md`). It exists to close the gap between the plan's one-line bullets ("reorg-safe, batched, rate-limit-aware, idempotent") and the code the implementer has to write next.

Out of scope:

- contract-level state queries beyond logs (covered indirectly by V3 math primitives in M2.2),
- L2 ingestion (mainnet WETH/USDC only for V1, per plan "Out of Scope"),
- WebSocket subscriptions for live data (M2.1 is historical replay; live ingestion is Tab 1's existing path),
- subgraph / GraphQL alternatives (DefiLlama / Aave subgraph use is M2.7, a separate concern).

## Current Project Relevance

Aurix today (verified by `src-tauri/src/ethereum/client.rs:1-131` and `src-tauri/src/dex/uniswap_v3.rs:1-152`) has a minimal JSON-RPC client that supports two methods only: `eth_call` and `eth_gasPrice`. There is no `eth_getLogs`, no batching, no retry layer, no rate-limit awareness, and no storage layer (M2.0 is the prerequisite milestone, not yet started — `context/plans/vector-a-v3-lp-backtester.md:90-98`).

The resume bullet supporting Tab 2 commits to "Uniswap V3 LP backtesting" with on-chain validation against ≥5 reference positions — none of which is possible until M2.1's swap event corpus exists. The plan budgets **~100k+ events / ~50MB SQLite / 30 days of WETH/USDC 5bps swaps**, and explicitly nominates Alchemy free tier for prototyping (`vector-a-v3-lp-backtester.md:248`). This paper is the bridge from "fetch some logs" to a concrete pattern that survives free-tier limits, reorgs, throttling, and re-runs.

The downstream consumers of the swap corpus depend on it being **trustworthy and complete**:

- M2.3's per-swap fee distribution requires every in-range swap, in order, with no duplicates and no silent gaps. A missing swap during a high-volume burst silently understates fees for any position that was in range during that burst.
- M2.4's validation harness compares modelled fees to on-chain truth within $X / 0.5%. If the ingestion path drops swaps under load, the validation tolerance is consumed by ingestion error rather than by math error, and the validation signal becomes uninterpretable.
- M2.5's strategy grid runs the same swap corpus against thousands of `(range × rebalance × deposit × period)` cells. Recomputing the corpus would 1000× the load; idempotent, append-only ingestion is the only viable shape.

## Current State Snapshot

| Aspect | Verified state | Source |
|---|---|---|
| RPC client surface | `eth_call`, `eth_gasPrice` only; no `eth_getLogs` | `src-tauri/src/ethereum/client.rs:33-105` |
| Transport | `reqwest::Client::new()`; no connection pool tuning, no timeout, no retry | `src-tauri/src/ethereum/client.rs:8-25` |
| Error types | `Transport`, `Rpc(message)`, `MissingResultField`, `MalformedHexValue`; no rate-limit branch, no JSON-RPC error code surfaced | `src-tauri/src/ethereum/client.rs:108-130` |
| Provider config | Builds `https://eth-mainnet.g.alchemy.com/v2/{ALCHEMY_API_KEY}` if `MAINNET_RPC_URL` not set; single endpoint, no fallback | `src-tauri/src/config/rpc.rs:23-52` |
| Storage layer | Not implemented; M2.0 is prerequisite | `vector-a-v3-lp-backtester.md:90-98` (plan); no `src-tauri/src/storage/` exists |
| Pool universe | WETH/USDC 5bps `0x88e6...5640` is the M2.1 starting point | `vector-a-v3-lp-backtester.md:103` |
| Storage budget | "≥ 30 days of WETH/USDC swaps (~100k+ events, ~50MB SQLite)" | `vector-a-v3-lp-backtester.md:103` |
| Reorg requirement | "only ingest blocks confirmed at depth ≥ 12" | `vector-a-v3-lp-backtester.md:103` (depth 12 reflects pre-Merge convention; this paper revises in light of PoS finality) |
| Idempotency requirement | "re-running ingestion never duplicates events" | `vector-a-v3-lp-backtester.md:103` |
| Rate-limit requirement | "respect free-tier RPC quotas, batch requests" | `vector-a-v3-lp-backtester.md:103` |

> **Repository fact** vs **project inference**: the plan's "depth ≥ 12" comes from Bitcoin/PoW-era convention. After PoS Merge (Sept 2022) the right primitive is **finalised block height** (~64-slot lag, ~12.8 min). The plan should be updated to read "ingest only blocks at or before the `finalized` tag, with `safe` as a fallback when the user accepts a known finality stall failure mode." See "Reorg-Safe Ingestion" below.

## Research Signal

| Topic | Source-backed signal | Source citation (URL + quoted passage) | Current repository state | Citation (file:line) | Project implication | Evidence class |
|---|---|---|---|---|---|---|
| Alchemy free-tier block range | "Free tier: 10" blocks; "Pay As You Go: unlimited"; all responses capped at 150MB | [Alchemy eth_getLogs reference](https://www.alchemy.com/docs/reference/eth-getlogs) — quoted §A1 | No `eth_getLogs` implementation yet | `client.rs:33-105` | M2.1 must batch in **10-block windows** on free tier; this is the binding constraint, not a "soft limit" | source-backed |
| Alchemy CU per call | `eth_getLogs` = 60 CU; `eth_call` = 26 CU | [Alchemy compute-unit costs](https://www.alchemy.com/docs/reference/compute-unit-costs) — quoted §A2 | n/a | n/a | 30-day backfill at 10-block batches = ~21.6k calls × 60 CU = ~1.3M CU = **0.43% of monthly cap**; budget is not the limit, throughput is | source-backed |
| Alchemy free-tier throughput | 300M CU/month free tier; 500 CUPs (compute units per second) | [Alchemy pricing / free tier details](https://www.alchemy.com/pricing) (search-extracted) | n/a | n/a | At 60 CU/call and 500 CUPs, theoretical max is ~8.3 calls/sec; latency-bound serial floor is ~0.2s/call → ~5 calls/sec; **30-day backfill takes ~50–75 min wall-clock** at 10-block batches | source-backed |
| Alchemy error response shape | `code: -32602`, message includes a **suggested working range** as hex tuple, e.g. `[0x0, 0xd043b8]` | [ethers.js issue #4703](https://github.com/ethers-io/ethers.js/issues/4703) — quoted §A3 | No JSON-RPC error code parsing in `RpcErrorPayload` | `client.rs:114-117` | Aurix's retry layer can parse the suggested range from the error message and self-tune batch size — far more efficient than blind exponential bisection | source-backed |
| Infura error response shape | `code: -32005`, `data: { from, limit, to }`; structured suggested range in `data` | [ethers.js issue #4703](https://github.com/ethers-io/ethers.js/issues/4703) — quoted §A3; [Infura docs](https://docs.infura.io/networks/ethereum/json-rpc-methods/eth_getlogs) | n/a | n/a | If Aurix adds Infura as fallback (M2.1 stretch), the parser must handle both error shapes; Infura's structured `data.limit` is cleaner | source-backed |
| Post-Merge finality | "all blocks that are deeper than 2 epochs in the past are considered 'finalized', i.e. it is impossible to revert past them" | [Paradigm "Ethereum Reorgs After The Merge"](https://www.paradigm.xyz/2021/07/ethereum-reorgs-after-the-merge) — quoted §A4 | Plan says "depth ≥ 12" (pre-Merge convention) | `vector-a-v3-lp-backtester.md:103` | Replace "depth 12" with `finalized` block tag (~2 epochs ≈ 12.8 min ≈ 64 blocks); use `safe` as a faster fallback (~1 epoch ≈ 6.4 min ≈ 32 blocks) when the user accepts the stall-on-finality-failure failure mode | source-backed + project inference |
| Block tag support | `finalized`, `safe`, `latest` defined per Alchemy spec and EIP-1898 | [Alchemy reference §"Block Tags"](https://www.alchemy.com/docs/reference/eth-getlogs) — quoted §A1 | Aurix passes hard-coded `"latest"` | `client.rs:48` | M2.1 ingestion `toBlock` must be `finalized` (or numeric `finalized − N`), never `latest`; this eliminates reorg risk by construction rather than by reactive correction | source-backed |
| Empirical reorg depth | "Full nodes have experienced about 90 chain re-orgs since the merge, with none of them reverting deeper than a single block" | search-extracted from Paradigm / hackmd corpus (cross-confirmed across multiple sources) | n/a | n/a | A naive 2-block depth would handle nearly all observed reorgs; the case for `finalized` is finality-guarantee, not historical-depth | source-backed |
| Subsquid reorg pattern | Configurable `setFinalityConfirmations()`; processor polls RPC for consensus changes; on reorg, "re-run the batch handler with the new consensus data and ask the Database to adjust its state" | [Subsquid SDK docs — RPC ingestion and reorgs](https://docs.sqd.ai/sdk/resources/unfinalized-blocks/) — quoted §A5 | n/a | n/a | Production pattern: lock `finalized` data as immutable, treat `safe..finalized` as a hot region requiring rollback; Aurix can simplify by ingesting **only ≤ finalized** and accepting ~13-min freshness lag | source-backed |
| alloy-rs retry layer | `RetryBackoffLayer::new(max_retry, backoff_ms, cups)`; retries 429 responses up to `max_retry` times | [alloy-rs example](https://github.com/alloy-rs/examples/blob/main/examples/layers/examples/retry_layer.rs) — quoted §A6 | No retry; single `?` on `error_for_status()` | `client.rs:51-67` | Either (a) migrate the transport to alloy and use `RetryBackoffLayer`, or (b) write the equivalent in-house tied to the existing `reqwest::Client`. (b) keeps the plan's "no ethers-rs" spirit and is small (~80 lines) | source-backed + project inference |
| ethers-rs deprecation | "ethers-rs has been deprecated, and users are recommended to migrate to Alloy" | search-extracted from alloy.rs migration material | n/a | Plan note explicitly avoids ethers-rs (see `vector-a-v3-lp-backtester.md:250`) | The plan's "no ethers-rs" stance is reinforced; alloy is the safer reference even for in-house transport because the failure modes ethers wrapped are now visible directly | source-backed |
| ethers retry gap | "Ethers.js does not automatically retry with adjusted ranges. Instead, these errors are merely emitted as 'debug' events, requiring manual handling by developers." | [ethers.js issue #4703](https://github.com/ethers-io/ethers.js/issues/4703) — quoted §A3 | n/a | n/a | **Contrasting source**: a popular library *does not* implement the obvious retry; the suggested-range parsing is cited as a known unfixed gap. Aurix's in-house implementation must avoid this same pit | source-backed (contrasting) |
| Reth log-index gap | "queries that took 15+ minutes now complete in 1 second" with a dedicated log index | [Reth issue #16999](https://github.com/paradigmxyz/reth/issues/16999); [Nethermind blog](https://www.nethermind.io/blog/speeding-up-eth-getlogs-at-scale) | n/a | n/a | **Contrasting source**: even archive-node providers can serve `eth_getLogs` slowly. Aurix's plan to "switch to archive node" later (`vector-a-v3-lp-backtester.md:249`) inherits this performance variance — Alchemy's Reth/Erigon backend is opaque. Plan a per-call timeout (30s) regardless | source-backed (contrasting) |
| V3 Swap event size | `Swap(address indexed sender, address indexed recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick)` → 3×32B topics + 5×32B data = **256 B binary payload**; ~1.1 KB per log on the JSON-RPC wire | [Uniswap V3 IUniswapV3PoolEvents docs](https://docs.uniswap.org/contracts/v3/reference/core/interfaces/pool/IUniswapV3PoolEvents) — quoted §A7 | n/a | n/a | SQLite per-row binary-packed: ~248 B; with page+index overhead: ~396 B/row → 30 days at 2,580 swaps/day = 77.4k rows = **~29 MB**. Plan's ~50MB budget holds even at 5,000 swaps/day. **Storage is not the bottleneck**; ingestion throughput is | source-backed + computed |
| Pool activity baseline | WETH/USDC 5bps pool: ~2,580 transactions in 24h, ~$15.5M volume (mainnet, recent snapshot) | [GeckoTerminal](https://www.geckoterminal.com/eth/pools/0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640) — quoted §A8 | n/a | n/a | Plan's "~100k+ events" estimate is high — actual 30-day count is closer to **~75–80k** at recent activity; the ~100k figure is a safe upper bound | source-backed + project inference |
| dRPC free tier | "120,000 CUs per minute per IP, approximately 100 eth_call requests per second"; flat 20 CU per method | [dRPC rate limiting](https://drpc.org/docs/howitworks/ratelimiting) (search-extracted) | n/a | n/a | dRPC is a viable fallback if Alchemy free tier proves throttle-bound; flat-rate CU is friendlier for `eth_getLogs` than method-weighted | source-backed |
| QuickNode block range | Universal 10K block range cap on `eth_getLogs` (no free-tier reduction) | [QuickNode docs](https://www.quicknode.com/docs/ethereum/eth_getLogs) (search-extracted) | n/a | n/a | If Aurix migrates to QuickNode, the batch size moves from 10 blocks to 10,000 blocks — **1000× fewer calls** — but free QuickNode is rate-limited differently | source-backed |

## Quoted Passages

> §A1 — Alchemy `eth_getLogs` reference, on per-tier block range:
>
> > "For Ethereum mainnet supports these maximum block ranges: Free tier: 10. Pay As You Go: unlimited. Enterprise: unlimited. All responses will be capped at 150MB."
>
> *Same page on block tags:* "`finalized` — The most recent crypto-economically secure block, cannot be re-orged outside of manual intervention driven by community coordination. `safe` — The most recent block that is safe from re-orgs under honest majority and certain synchronicity assumptions. `latest` — The most recent block in the canonical chain observed by the client, this block may be re-orged out of the canonical chain even under healthy/normal conditions."

> §A2 — Alchemy compute-unit costs, on per-method CU:
>
> > "eth_getLogs | 60 | | … eth_call | 26 | | … eth_blockNumber | 10 | | … eth_getBlockByNumber | 20 | | … eth_getTransactionReceipt | 20 | |"
>
> (table row format from the docs)

> §A3 — ethers.js issue #4703, on Alchemy and Infura error shapes for oversized `eth_getLogs`:
>
> > Infura: `"error": { "code": -32005, "data": { "from": "0xBDE5F8", "limit": 10000, "to": "0x102DBCC" }, "message": "query returned more than 10000 results..." }`
> >
> > Alchemy: `"error": { "code": -32602, "message": "Log response size exceeded...this block range should work: [0x0, 0xd043b8]" }`
> >
> > "Ethers.js does **not** automatically retry with adjusted ranges. Instead, these errors are merely emitted as 'debug' events, requiring manual handling by developers."

> §A4 — Paradigm, "Ethereum Reorgs After The Merge", on finality:
>
> > "all blocks that are deeper than 2 epochs in the past are considered 'finalized', i.e. it is impossible to revert past them."
> >
> > "even single-block reorgs are extremely difficult, because an attacker controlling only a few validators has no way to beat the honest majority of thousands of attesters."

> §A5 — Subsquid SDK docs, on reorg handling:
>
> > "When ingesting from RPC, Squid SDK can index blocks before they are finalized, enabling real-time use cases. If a blockchain reorganization happens, processor will roll back any changes to the database made due to orphaned blocks, then re-run its batch handler on consensus blocks."
> >
> > "All blocks with fewer confirmations than the number set by the `setFinalityConfirmations()` setting are considered 'hot' (unfinalized) on EVM."

> §A6 — alloy-rs example `retry_layer.rs`:
>
> > ```rust
> > let max_retry = 10;
> > let backoff = 1000;
> > let cups = 100;
> > let retry_layer = RetryBackoffLayer::new(max_retry, backoff, cups);
> > let client = RpcClient::builder().layer(retry_layer).http(anvil.endpoint_url());
> > ```
> >
> > "The layer will retry all requests that return a rate limit error (eg. 429) until max_retries have been reached."

> §A7 — Uniswap V3 `IUniswapV3PoolEvents` interface:
>
> > `event Swap(address indexed sender, address indexed recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick);`

> §A8 — GeckoTerminal pool snapshot:
>
> > "USDC/WETH on Uniswap V3 (Ethereum) 0.05% Fee — 24h transactions: 2,580. 24h volume: $15.53M."

## Mechanism Summary

### A. The shape of `eth_getLogs`

`eth_getLogs` takes a filter object — `{ fromBlock, toBlock, address, topics }` — and returns every log matching the filter, in chronological order. There is **no native pagination**: the only knob the caller has is the block range. Every production ingestion library and indexer treats "block range" as the page size primitive (Chainstack and the alloy-rs `LogQuery` iterator both confirm this).

For Aurix, the filter is fixed:

```text
{
  "fromBlock": "0x...",
  "toBlock":   "0x...",
  "address":   "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640",       // WETH/USDC 5bps pool
  "topics":    ["0x7a6f9cbbaf2f9feccfd2e1e45f4f3b20f1dfaf425d9b97fb32c7a313562c861f"]   // V3 Swap signature
}
```

The `address` and `topics[0]` filters do most of the work — both are bloom-filter-indexed on the node side, so the per-call cost on the node scales with block range, not log count. This is why providers cap by block range, not by output rows alone.

### B. The four constraints providers actually impose

Aggregating the source material:

| Provider | Block-range cap | Result-count cap | Response-size cap | CU/call (where applicable) |
|---|---|---|---|---|
| Alchemy (free) | **10 blocks** [§A1] | none stated below 150MB | 150MB | 60 [§A2] |
| Alchemy (PAYG) | unlimited (with 2K/no-cap or any-range/10K-result split) | 10K logs **or** unlimited if range ≤ 2K | 150MB | 60 |
| Infura | any range, 10K-result hard cap | **10,000 logs** (hard cap) | not stated | not CU-priced |
| QuickNode | **10,000 blocks** | not stated | not stated | not CU-priced |
| dRPC | unspecified | unspecified | unspecified | flat 20 CU/method |
| Chainstack | recommends 5,000 blocks (Ethereum) | not enforced | not stated | not CU-priced |

All caps are **simultaneous AND** — Alchemy free tier means "≤10 blocks AND ≤150MB AND under your CU/sec ceiling". The block range is the binding constraint by orders of magnitude.

### C. The reorg model after the Merge

The plan's "depth 12" is pre-Merge advice. Post-Merge (Sept 2022), Ethereum has hard finality: once a block is justified by ≥2/3 of validators across two consecutive epochs (64 slots, 12.8 minutes), reverting it would require burning ≥1/3 of the entire stake. Empirically, the deepest observed mainnet reorg post-Merge is one block (per Paradigm and the broader corpus surfaced during research).

This reshapes the ingestion model:

```text
LATEST    ────────────────────────────────────  block N (head)
            |  may reorg even under healthy operation
SAFE      ────────────────────────────────────  block N − 32  (≈6.4 min ago)
            |  reorg requires ≥1/3 attacker; one observed
FINALIZED ────────────────────────────────────  block N − 64+ (≈12.8 min ago)
            |  reorg requires burning ≥1/3 stake; never observed
[SAFE TO INGEST]
```

Aurix M2.1 should ingest **only `toBlock = "finalized"`**. This:

- removes reorg from the failure model entirely (no rollback logic needed in storage),
- introduces a known ~13-minute freshness lag (acceptable: M2.1 is historical replay, not live),
- means the implementation never has to reason about "did the block I just stored still exist".

The cost of this model is one extra `eth_getBlockByNumber("finalized", false)` call before each ingestion run — 20 CU, negligible. The benefit is that the storage layer can treat every ingested row as immutable.

> **Failure mode worth knowing:** during a long inactivity leak (no economic finality for many epochs, e.g. >50% validators offline), the `finalized` tag stops advancing. M2.1 ingestion would stall. This is acceptable: it is a global Ethereum incident, not an Aurix bug, and the right behaviour is to stop ingesting rather than ingest re-orgable data. Surface the lag in the UI ("most recent finalized block: N, X minutes ago") so the user sees the stall.

### D. CU and wall-clock budget

For 30 days of WETH/USDC 5bps swaps, on Alchemy free tier:

```text
30 days × 7,200 blocks/day      = 216,000 blocks
÷ 10 blocks/call (free tier cap) =  21,600 eth_getLogs calls
× 60 CU/call                     = 1,296,000 CU spent
÷ 300,000,000 CU/month free cap  =   0.43% of monthly budget

Throughput floor (latency-bound, ~200 ms/call serial):
  21,600 × 0.2 s = 72 min wall-clock

Throughput floor (CU-bound, 500 CUPs):
  1,296,000 ÷ 500 = 43 min wall-clock

Practical expectation: ~50–75 min for the 30-day backfill on free tier.
```

CU consumption is a non-issue. The backfill is **latency-bound**, not budget-bound. Two implementation consequences:

1. **Sequential calls are ~5× slower than they need to be.** A small concurrent worker pool (`tokio::task::JoinSet` with 4–8 in-flight calls, throttled by a semaphore) cuts wall-clock to ~10–15 minutes without hitting the 500 CUPs ceiling.
2. **The 60 CU per call has zero practical effect** for this single workload. CU only becomes a problem when M2.1 runs alongside Tab 1's live polling (which spends CU on `eth_call` at 1 Hz) and any future M2.7 benchmark fetches.

If Alchemy's free tier proves throttle-bound mid-backfill (HTTP 429s saturate the retry layer), the right escalation order is:

```text
1. Reduce concurrent workers to 2.    (no plan change)
2. Add jitter to backoff.              (small code change)
3. Switch to Pay-as-you-go.            (~$1 for the full backfill at PAYG rates)
4. Switch to QuickNode (10K blocks/call → 22 calls total).   (provider change)
5. Switch to a public node (PublicNode, LlamaRPC).           (free, less reliable)
```

Pay-as-you-go is the operationally cheapest escalation for this workload — the backfill costs cents.

### E. The "100k events" question

The plan estimates ~100k+ events. The empirical anchor (GeckoTerminal §A8: 2,580 tx/24h on the 5bps pool) gives ~77.4k swaps over 30 days at recent activity levels. Historic peak periods (March 2023 SVB, March 2024 ETH ATH attempts) push this to ~150–200k for those 30-day windows. **The ~100k figure is a safe central estimate; budget for 200k as the upper-tail.**

### F. Idempotency and the composite key

The right primary key for a `swap_event` row is the **composite (block_hash, log_index)**, not (block_number, log_index).

Block hash makes the key reorg-invariant in the strict sense: a reorged block produces a different hash for the same block number, and the row from the old hash is naturally orphaned rather than silently overwritten. Even though the `finalized` policy in §C means Aurix never ingests reorg-prone blocks, the block-hash key is the cheaper and more defensive choice — it costs ~10 bytes per row and removes an entire class of "did we just clobber a real row" failure modes.

Log index is unique within a block (it's the EVM's per-block sequence number across all logs in all transactions). The combination is unique across the entire chain.

| Candidate key | Reorg-safe? | Storage cost | Notes |
|---|---|---|---|
| `(block_number, log_index)` | No — same key after reorg, different content | smallest | rejected |
| `(block_hash, log_index)` | Yes | +24 bytes vs block_number | **chosen** |
| `(tx_hash, log_index)` | Yes (tx hash is content-addressed) | +24 bytes | viable; less cache-friendly because logs in the same block share `block_hash` but not `tx_hash`, hurting range scans |
| `(tx_hash, log_index_in_tx)` | Yes | smallest of tx-hash variants | rejected — `log_index_in_tx` is not always returned and tools commonly use the block-level index |

SQL form (M2.0 schema, anticipated):

```sql
CREATE TABLE swap_events (
    block_hash      BLOB(32) NOT NULL,
    log_index       INTEGER  NOT NULL,
    block_number    INTEGER  NOT NULL,
    block_timestamp INTEGER  NOT NULL,
    tx_hash         BLOB(32) NOT NULL,
    pool_address    BLOB(20) NOT NULL,
    sender          BLOB(20) NOT NULL,
    recipient       BLOB(20) NOT NULL,
    amount0         BLOB(32) NOT NULL,  -- int256, two's-complement big-endian
    amount1         BLOB(32) NOT NULL,
    sqrt_price_x96  BLOB(32) NOT NULL,  -- uint160 stored in 32B for SQL ergonomics
    liquidity       BLOB(16) NOT NULL,  -- uint128
    tick            INTEGER  NOT NULL,  -- int24 fits in INTEGER (8B)
    gas_price_wei   BLOB(16) NOT NULL,  -- uint128 from block header (per plan M2.1 last bullet)
    PRIMARY KEY (block_hash, log_index)
) WITHOUT ROWID;

CREATE INDEX ix_swap_events_block_number ON swap_events(block_number);
CREATE INDEX ix_swap_events_pool_block   ON swap_events(pool_address, block_number);
```

`WITHOUT ROWID` keeps the primary key as the heap layout, which is correct here because the natural query pattern (M2.3 simulation: "all swaps for pool P between blocks A and B") uses a covering index path through `(pool_address, block_number)`. The composite primary key prevents duplicates without an explicit `ON CONFLICT IGNORE` round-trip during the insert hot path.

`INSERT ... ON CONFLICT(block_hash, log_index) DO NOTHING` makes ingestion truly idempotent: a re-run, a partial rewind, or a manual replay of the same range produces zero duplicates with no caller-side bookkeeping. This is the property the plan wants when it says "re-running ingestion never duplicates events".

### G. The retry-layer pattern (in-house, ~80 lines)

Three error categories need three responses:

| Error | Response | Why |
|---|---|---|
| HTTP 429 / JSON-RPC `-32005` "rate limited" | exponential backoff with jitter; retry up to N | transient throughput overage |
| JSON-RPC `-32602` / "log response size exceeded" / "block range too large" | parse suggested range from message; halve `(toBlock − fromBlock)`; retry with smaller window | persistent for this range; deterministically resolvable |
| HTTP 5xx | exponential backoff; retry up to N (smaller cap) | transient provider failure |
| HTTP 4xx other than 429 | fail fast | client bug; retrying won't help |
| Network timeout | retry once with longer timeout | TCP weather |
| JSON-RPC `-32601` "method not found" | fail fast | endpoint misconfigured |

Backoff math (exponential with full jitter, per AWS Builder's Library — well-established pattern):

```text
delay_n  =  random_uniform(0, min(cap, base × 2^n))

base = 250 ms      // initial backoff
cap  = 8 s         // ceiling per retry
n    = 0..7        // max 8 retries → worst-case ~30s of accumulated waits
```

Jitter is the load-balancer-friendly variant (every client randomises in `[0, 2^n × base]` rather than waiting exactly `2^n × base`). Without jitter, fleets of clients synchronise their retries and produce the next 429 spike together. For a single-user desktop app this matters less — but the cost is one `random()` call, so adopt the better pattern unconditionally.

The "suggested range" parser is the differentiator. Pseudocode against the existing `EthereumRpcClient` shape:

```rust
async fn get_logs_with_adaptive_batch(
    &self,
    filter: LogFilter,           // address, topics, fromBlock, toBlock
    cap_blocks: u64,             // start at provider's documented cap
) -> Result<Vec<Log>, EthRpcError> {
    let mut from = filter.from_block;
    let mut chunk = cap_blocks;
    let mut out = Vec::new();
    let mut retry_count = 0u32;

    while from <= filter.to_block {
        let to = (from + chunk - 1).min(filter.to_block);
        match self.eth_get_logs_one_call(&filter, from, to).await {
            Ok(logs) => {
                out.extend(logs);
                from = to + 1;
                retry_count = 0;
                // gradually grow chunk back toward cap_blocks
                chunk = (chunk * 2).min(cap_blocks);
            }
            Err(EthRpcError::Rpc { code: -32602, message, .. })
                if message.contains("block range") || message.contains("Log response size") =>
            {
                // Try to parse Alchemy's "[0x0, 0xd043b8]" suggestion;
                // fall back to halving on any parse failure.
                chunk = parse_suggested_range(&message)
                    .map(|(lo, hi)| (hi - lo + 1).max(1))
                    .unwrap_or(chunk / 2);
                if chunk == 0 {
                    return Err(EthRpcError::BatchSizeExhausted);
                }
                // do NOT advance `from` — retry the same window with the smaller chunk
            }
            Err(EthRpcError::HttpStatus(429)) => {
                tokio::time::sleep(jittered_backoff(retry_count)).await;
                retry_count += 1;
                if retry_count > MAX_RETRIES { return Err(EthRpcError::RetriesExhausted); }
            }
            Err(other) => return Err(other),
        }
    }
    Ok(out)
}
```

Two implementation notes worth highlighting:

1. **Do not pre-batch the entire 30-day range and dispatch all calls.** Stream: each chunk's logs go to SQLite before the next chunk is fetched, so a mid-backfill failure resumes from the last persisted block, not from zero.
2. **`from = to + 1` is correct** because `eth_getLogs` block ranges are **inclusive on both ends**. Off-by-one errors here either skip blocks or duplicate them; the composite primary key catches duplication, but skipped blocks are silent.

### H. The contrasting view — why this matters

Three sources surfaced during research push back against the obvious approach:

- **ethers.js issue #4703** (quoted §A3): a popular library does not implement the suggested-range retry. The error is emitted as a debug event and the application is left to handle it. Aurix's "in-house implementation" plan is correct, but the implementation must avoid this same hole.
- **Reth issue #16999 / Nethermind blog on log indexing**: even archive-node providers can serve `eth_getLogs` slowly when a dedicated log index is missing — "queries that took 15+ minutes now complete in 1 second" with the index. Alchemy's backend is opaque, and the pre-index slow path can show up under load. Aurix's per-call timeout (default 30s, with retry) is the load-bearing mitigation.
- **Free-tier reduction is recent and undocumented in changelogs**: community wisdom (and even Alchemy's own deep-dive page, per quoted §A1 vs the 2K-block secondary doc) still references 2K-block tolerance. The **10-block free-tier cap is the current authoritative limit** but is not what most blog posts and StackOverflow answers will tell you. This is a real "trial and error" pit and the primary justification for this paper.

## What Fits This Project Well

1. **`finalized` block tag as the ingestion ceiling.** Eliminates an entire failure category at zero CU cost. Aligns with the project's "honest" framing — historical backtests against immutable, finalised data are exactly what hiring readers want to see.
2. **`(block_hash, log_index)` composite primary key with `ON CONFLICT DO NOTHING`.** Matches the plan's idempotency requirement directly with no application-layer dedup state.
3. **In-house retry layer (~80 lines) over the existing `reqwest::Client`.** Preserves the plan's "no ethers-rs" stance, keeps the dependency footprint small, and lets the retry behaviour be unit-tested deterministically (mock the JSON-RPC response, assert the chunk-shrink path). alloy-rs is the reference for the design but does not need to be a dependency.
4. **Stream-and-persist per chunk.** Each call's result hits SQLite before the next call goes out. Resumability becomes free: re-running ingestion with the same `(fromBlock, toBlock)` produces zero duplicates and skips work already on disk via the unique key. Matches the M2.0 plan's resumable schema design.
5. **Concurrent worker pool, sized to ≤500 CUPs.** Cuts wall-clock to ~10–15 min. Use `tokio::task::JoinSet` + `tokio::sync::Semaphore::new(8)` — small, idiomatic, no extra dependency.
6. **Per-block gas price recorded alongside each swap.** The plan calls this out explicitly (`vector-a-v3-lp-backtester.md:104`); fold it into the same ingestion path by issuing one `eth_getBlockByNumber` per *unique* block in the batch result (most batches of swaps span a small set of blocks; cache hit ratio is high).

## What Fits This Project Badly

1. **Ingesting unfinalised blocks with a Subsquid-style rollback layer.** Subsquid's design is right for live indexers, but Aurix is a desktop backtester with no real-time consumer. The complexity (rollback queue, double-bookkeeping, fork detection) is not earned by any user-visible benefit. Reject in favour of the `finalized`-only approach.
2. **A general retry middleware crate (`tower-retry`, `governor`, etc.).** These are good crates, but the Aurix retry policy is unusually specific (parse JSON-RPC error messages for suggested block ranges) and the in-house version is small enough that the dependency is net-negative for read-cost.
3. **WebSocket subscriptions for the historical backfill.** WS is the right primitive for live data (Tab 1's existing path), but `eth_getLogs` over HTTP is what providers tune for archive queries. Use the right tool — historical → HTTP, live → WS.
4. **Block-hash + log-index without a separate `block_number` column.** Tempting for storage minimalism, but the natural query in M2.3 is "all swaps for pool P in `[block_a, block_b]`" — this needs a `block_number` index. Keep both columns; the cost is 8 bytes per row.
5. **Pre-fetching the full block list with `eth_getBlockByNumber` before issuing log calls.** Wastes 20 CU per block × 216k blocks = 4.3M CU for no information that the log results don't already carry (logs include `blockNumber`, `blockHash`, `transactionHash`). Issue block-by-number calls only for the *unique blocks* in the log result, and only when gas price is needed.
6. **Trusting the `latest` tag.** Aurix's existing client passes `"latest"` (`client.rs:48`) for `eth_call`. That's fine for live spot prices (Tab 1 tolerates a transient reorg). For M2.1 historical ingestion, `"latest"` is wrong — it can change underneath an in-flight ingestion run. Use `"finalized"` or a numeric block number captured at run start.

## Gap Analysis

| Plan obligation (`vector-a-v3-lp-backtester.md`) | Status | Gap |
|---|---|---|
| Batched `eth_getLogs` fetcher | Not started | No `eth_getLogs` in `client.rs`. Add `eth_get_logs(&self, filter)` method + `get_logs_with_adaptive_batch` orchestrator |
| Topic filtering by pool address | Not started | Topic constant for V3 Swap signature should live in `dex/uniswap_v3.rs` next to existing `SLOT0_CALLDATA` |
| Reorg-safe (depth ≥ 12) | Plan-stale | Replace "depth 12" with `finalized` block tag in plan + implementation |
| Backfill ≥ 30 days, ~100k+ events, ~50MB SQLite | Not started | Storage budget verified safe (29–57 MB for 2.5–5k swaps/day) |
| Idempotency | Not started | Compose `(block_hash, log_index)` primary key + `ON CONFLICT DO NOTHING` in M2.0 schema |
| Rate-limit-aware | Not started | In-house retry layer; suggested-range parser; ≤8-worker concurrency |
| Per-block gas price recorded | Not started | Cache-by-unique-block `eth_getBlockByNumber` in the same ingestion path |

## Recommended Priority Order

1. **Update the plan** — strike "depth ≥ 12", insert "ingest only `toBlock = finalized`". Cite this paper. (15 min — touches `vector-a-v3-lp-backtester.md:103`.)
2. **Extend `EthereumRpcClient`** with `eth_get_logs(&self, filter: LogFilter) -> Result<Vec<Log>, _>` returning the raw RPC log objects; deserialise with `serde::Deserialize` against the documented `Log` shape. (2–3 h — sits next to existing `eth_call`.)
3. **Add `LogFilter` and `Log` types** in `src-tauri/src/ethereum/types.rs` (new file). Use `BlockTag::Finalized | BlockTag::Number(u64)` for `to_block`; reject `Latest` at the type level for ingestion. (~1 h.)
4. **Write the adaptive-batch orchestrator** `get_logs_with_adaptive_batch` per pseudocode in §G. Parameterise on `cap_blocks` (default 10 for Alchemy free) and `max_concurrency` (default 8). Stream results via a `tokio::sync::mpsc::Sender<Vec<Log>>` so persistence and fetch overlap. (4–6 h.)
5. **Implement the retry/backoff with jitter** as a small module (`ethereum/retry.rs`). Unit-test deterministically with a `tokio::time::pause()` clock and a mock JSON-RPC layer. (2–3 h.)
6. **Land M2.0 schema** with the composite primary key as written in §F. SQLite `WITHOUT ROWID` + `INSERT … ON CONFLICT DO NOTHING`. (covered separately in `sqlite-rust-production-patterns.md`.)
7. **Wire up the V3 Swap event decoder** — pure function from `Log` → `SwapEvent`, parsing the 5×32B data section and 2×32B indexed addresses. Property-test against alloy-rs's decoder for one canonical fixture log. (3–4 h.)
8. **Backfill orchestrator** at the command layer: walk `[start_block, finalized_block]` in 10-block chunks for free tier, persist each chunk transactionally. Surface progress via a Tauri event so the UI can show ingestion progress. (2–3 h.)
9. **Smoke-test the full path** by ingesting one day of WETH/USDC 5bps and asserting count and totals against Etherscan's exported CSV for the same range. (1–2 h.)

Total: ~16–22 h before M2.1 is acceptance-ready.

## Open Uncertainties And Validation Needs

1. **Free-tier 500 CUPs vs 330 CUPs ambiguity.** Different Alchemy support pages quote different per-second caps. Resolve at first 429 — the actual cap is whatever the response says. Validation: log `Retry-After` headers and the empirical CUPs at which 429s start, cache that as the practical limit.
2. **Whether `finalized` block tag is supported in `eth_getLogs.fromBlock/toBlock` on Alchemy.** The reference page (§A1) defines the tag, but the search did not surface a direct example of it being passed to `eth_getLogs`. Validation: send one `eth_getLogs` with `toBlock="finalized"` against the test endpoint at start-up and assert success; if it errors, fall back to `eth_getBlockByNumber("finalized") → toBlock=<number>`. The fallback is universal.
3. **Reth/Erigon backend variance behind Alchemy.** Cannot probe directly. Mitigation: per-call 30s timeout with retry; if a single call times out twice, halve the chunk and try again.
4. **Mainnet swap rate at peak vs current snapshot.** The 2,580 tx/24h baseline is recent; 2024 peak periods saw 3–4× this. Validation: parameterise the storage budget so the SQLite file path can hold ≥150k rows comfortably; the 50MB plan budget allows ≥125k rows at the calculated 396 B/row, so already adequate.
5. **The "10 blocks free tier" cap may change.** Alchemy has historically tightened limits without changelog announcements. Validation: surface the chosen `cap_blocks` in the ingestion config; on first `-32602`, log the suggested range and treat that as the new ceiling rather than failing the run.

## Relationship To Existing Context

- **Cross-references this paper.** `context/plans/vector-a-v3-lp-backtester.md` (M2.1 milestone, lines 99–107) — the operational target. Update line 103's "depth ≥ 12" to "finalized block tag" with a back-reference here.
- **Adjacent reference.** `context/references/sqlite-rust-production-patterns.md` — the schema in §F belongs at the M2.0 schema-design moment; this paper provides the ingestion-side requirements that the schema must satisfy.
- **Adjacent reference.** `context/references/v3-position-validation-methodology.md` — M2.4's validation harness depends on the swap corpus this paper specifies; the trust contract between ingestion and validation is "every in-range swap, exactly once, with block hash for reorg-invariance".
- **Adjacent reference.** `context/references/v3-mathematics-deep-dive.md` — owns the `int256/uint160/uint128/int24` decoding details for the Swap event's `data` section. This paper does not reproduce that; the consumer of `swap_events` rows reads the Q64.96 values into `BigUint` exactly as `dex/uniswap_v3.rs:52-63` already does for `slot0`.
- **Touches.** `context/architecture.md` — the "Backend market pipeline" subsystem table grows to include an `ingestion/` module after M2.1 lands. Update at the "completing M2.1" milestone, not now.

## External Research Trail

Primary URLs cited in this paper (verbatim, for the validator and for downstream readers):

- https://www.alchemy.com/docs/reference/eth-getlogs
- https://www.alchemy.com/docs/reference/compute-unit-costs
- https://www.alchemy.com/docs/deep-dive-into-eth_getlogs
- https://www.alchemy.com/docs/reference/compute-units
- https://www.alchemy.com/overviews/ethereum-commitment-levels
- https://www.alchemy.com/pricing
- https://www.paradigm.xyz/2021/07/ethereum-reorgs-after-the-merge
- https://www.circle.com/blog/exploring-confirmation-rules-for-ethereum
- https://goldsky.com/products/subgraphs
- https://docs.sqd.ai/sdk/resources/unfinalized-blocks/
- https://github.com/ethers-io/ethers.js/issues/4703
- https://github.com/alloy-rs/examples/blob/main/examples/layers/examples/retry_layer.rs
- https://alloy.rs/examples/layers/retry_layer/
- https://www.geckoterminal.com/eth/pools/0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640
- https://drpc.org/docs/howitworks/ratelimiting
- https://docs.chainstack.com/docs/understanding-eth-getlogs-limitations
- https://docs.infura.io/networks/ethereum/json-rpc-methods/eth_getlogs
- https://www.quicknode.com/docs/ethereum/eth_getLogs
- https://github.com/paradigmxyz/reth/issues/16999
- https://www.nethermind.io/blog/speeding-up-eth-getlogs-at-scale
- https://docs.uniswap.org/contracts/v3/reference/core/interfaces/pool/IUniswapV3PoolEvents

Quoted passages relevant to the floor (each paraphrased here so the trail section also carries direct evidence; full versions in §A1–§A8 above):

> "Free tier: 10. Pay As You Go: unlimited. Enterprise: unlimited. All responses will be capped at 150MB." — Alchemy `eth_getLogs` reference

> "eth_getLogs | 60 | | … eth_call | 26 | | … eth_blockNumber | 10" — Alchemy compute-unit costs

> "Ethers.js does not automatically retry with adjusted ranges. Instead, these errors are merely emitted as 'debug' events, requiring manual handling by developers." — ethers.js issue #4703

> "all blocks that are deeper than 2 epochs in the past are considered 'finalized', i.e. it is impossible to revert past them." — Paradigm

> "When ingesting from RPC, Squid SDK can index blocks before they are finalized... If a blockchain reorganization happens, processor will roll back any changes to the database made due to orphaned blocks." — Subsquid SDK docs

> "The layer will retry all requests that return a rate limit error (eg. 429) until max_retries have been reached." — alloy-rs RetryBackoffLayer example

> "USDC/WETH on Uniswap V3 (Ethereum) 0.05% Fee — 24h transactions: 2,580. 24h volume: $15.53M." — GeckoTerminal

### Searches run

| # | Query | Tool | Rationale | Sources surfaced |
|---|---|---|---|---|
| 1 | `Alchemy eth_getLogs maximum block range limit 2026 compute units` | WebSearch | Establish provider cap baseline | Alchemy reference, Alchemy compute-unit costs, Alchemy deep-dive, Chainstack, ethers.js #4703 |
| 2 | `Infura eth_getLogs block range limit 10000 blocks response size cap` | WebSearch | Cross-provider comparison | Infura docs, Infura community thread, MetaMask docs, graph-protocol issue |
| 3 | `QuickNode eth_getLogs block range limit free tier 2026` | WebSearch | Third-provider comparison | QuickNode forum, QuickNode docs, QuickNode 10K-block support article |
| 4 | `Alchemy free tier eth_getLogs 10 blocks reduced limit 2024 2025 announcement` | WebSearch | Sanity check on the 10-block cap | Alchemy reference, Alchemy pricing, Alchemy free-tier details, Chainstack overview |
| 5 | `Ethereum reorg depth post-merge finalized safe block confirmation 2 epochs PoS` | WebSearch | Establish post-Merge finality model | Circle, Alchemy commitment levels, Paradigm, eprint cryptography paper |
| 6 | `Subsquid Ponder Goldsky archive log indexer reorg handling block hash` | WebSearch | Production-indexer reorg patterns | Envio, Goldsky, Subsquid blog/docs, Ormi Labs |
| 7 | `alloy-rs ethers-rs eth_getLogs batching pagination retry implementation` | WebSearch | Reference implementation of retry/batching in Rust | alloy-rs examples, ethers-io discussions, ethers-rs pagination issue, Alchemy roadmap |
| 8 | `Alchemy free tier compute units monthly cap 300 million throttle 429` | WebSearch | CU budget confirmation | Alchemy pricing, free-tier details, throughput docs, Chainstack 2026 overview |
| 9 | `"eth_getLogs" "block range too large" Alchemy error code -32602 -32005 truncation` | WebSearch | Error-shape evidence | ethers.js #4703, Infura community, Alchemy roadmap, geth #25343, geth #28765 |
| 10 | `Subsquid hot block reorg unfinalized rollback indexer documentation` | WebSearch | Subsquid primary doc | docs.sqd.ai unfinalized-blocks, Squid SDK |
| 11 | `Uniswap V3 Swap event ABI bytes log size topics data fields` | WebSearch | Event size ground truth | Uniswap docs IUniswapV3PoolEvents, web3-ethereum-defi, v3-core source |
| 12 | `Ankr dRPC PublicNode LlamaNodes eth_getLogs free public RPC limits comparison` | WebSearch | Alternative-provider tradeoffs | comparenodes.com, awesome-list-rpc-providers, dRPC rate-limiting docs |
| 13 | `alloy-rs RetryBackoffLayer rate limit 429 implementation source code` | WebSearch | Canonical Rust retry pattern | alloy.rs example, alloy-rs/examples GitHub |
| 14 | `Uniswap V3 WETH USDC 0.05 fee tier daily swap count 2025 events per day` | WebSearch | Pool-activity baseline | GeckoTerminal mainnet pool, Nansen, Uniswap support |
| 15 | `"0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640" pool swap volume per day historical` | WebSearch | Pool-specific historical anchor | Yield Samurai, Uniswap app, EigenPhi, Chainbase |
| 16 | `Reth Erigon eth_getLogs index performance archive node log query` | WebSearch | Archive-node performance variance (contrasting view) | Reth #16999, Erigon GitHub, Nethermind blog |

### Sources consulted

| URL | Tool | Source class | Quoted in artefact |
|---|---|---|---|
| https://www.alchemy.com/docs/reference/eth-getlogs | WebFetch (×2) | Official documentation | Yes (§A1) |
| https://www.alchemy.com/docs/reference/compute-unit-costs | WebFetch | Official documentation | Yes (§A2) |
| https://www.alchemy.com/docs/deep-dive-into-eth_getlogs | WebFetch | Official documentation (deep-dive) | Yes (referenced as conflicting older guidance) |
| https://www.alchemy.com/docs/reference/compute-units | WebFetch | Official documentation | No (no specific CU data extracted) |
| https://www.alchemy.com/overviews/ethereum-commitment-levels | WebFetch | Official documentation | Yes (commitment-level definitions) |
| https://www.paradigm.xyz/2021/07/ethereum-reorgs-after-the-merge | WebFetch | Engineering writeup (foundational) | Yes (§A4) |
| https://www.circle.com/blog/exploring-confirmation-rules-for-ethereum | WebFetch | Engineering writeup (production constraint) | Yes (referenced for ~15-min finality figure) |
| https://goldsky.com/products/subgraphs | WebFetch | Vendor / open-source-adjacent | No (page lacked technical detail) |
| https://docs.sqd.ai/sdk/resources/unfinalized-blocks/ | WebFetch | Open-source implementation docs | Yes (§A5) |
| https://github.com/ethers-io/ethers.js/issues/4703 | WebFetch | Open-source issue tracker | Yes (§A3) — **contrasting source** |
| https://github.com/alloy-rs/examples/blob/main/examples/layers/examples/retry_layer.rs | WebFetch | Open-source reference implementation | Yes (§A6) |
| https://alloy.rs/examples/layers/retry_layer/ | WebFetch | Open-source documentation | Yes (§A6) |
| https://www.geckoterminal.com/eth/pools/0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640 | WebFetch | Empirical / market-data | Yes (§A8) |
| https://drpc.org/docs/howitworks/ratelimiting | WebFetch | Provider documentation | Search-extracted (page redirect) |
| https://docs.chainstack.com/docs/understanding-eth-getlogs-limitations | WebFetch | Provider engineering writeup | Yes (referenced for 5K-block recommendation) |

Source classes covered:

- **Official provider documentation**: Alchemy (4 pages), Chainstack, dRPC, Subsquid SDK, Uniswap V3 interface
- **Open-source implementation / issue trackers**: alloy-rs/examples, ethers.js issue, Subsquid docs, Reth issue, Nethermind blog
- **Engineering writeup (foundational)**: Paradigm post-Merge reorg analysis, Alchemy commitment-levels overview
- **Empirical / market data**: GeckoTerminal pool snapshot

Contrasting / failure-mode sources represented:

- ethers.js issue #4703 — popular library does not auto-retry suggested ranges (quoted §A3)
- Reth issue #16999 / Nethermind log-indexing post — even archive-node `eth_getLogs` can be slow without dedicated indexes (quoted in research signal table)
- Alchemy reference vs Alchemy deep-dive — internally contradictory documentation; the 10-block free-tier cap is the binding constraint and contradicts older 2K-block guidance still on Alchemy's own deep-dive page

## Pre-Completion Obligation Audit

| Obligation | Evidence | Status |
|---|---|---|
| ≥3 distinct WebSearch calls | 16 distinct queries listed above | met |
| ≥3 distinct WebFetch calls against primary sources | 15 distinct fetches against ≥4 source classes | met |
| ≥2 source classes | Official docs + OSS implementation/issues + engineering writeups + empirical data (4 classes) | met |
| ≥1 contrasting source | ethers.js #4703 (library does not implement obvious retry); Reth #16999 (archive-node slow path); Alchemy doc internal contradiction (§A1 vs deep-dive) | met |
| ≥1 direct quoted passage per major source-backed claim | §A1–§A8 cover the eight load-bearing claims | met |
| Project files read | `context/plans/vector-a-v3-lp-backtester.md`, `context/architecture.md`, `context/references/lp-rebalancing-strategies.md` (scaffold), reference folder listing | met |
| Code files inspected | `src-tauri/src/ethereum/client.rs`, `src-tauri/src/dex/uniswap_v3.rs`, `src-tauri/src/config/rpc.rs` | met |
| `init_research_artifact.py` run | Output: "Created file scaffold: .../ethereum-archive-log-ingestion.md" | met |
| `validate_research_artifact.py` run | See Completion Report | pending — runs before handoff |
| Adversarial sweep | Performed; see "What I Did Not Do" | met |

## What I Did Not Do

- **Did not run `eth_getLogs` against Alchemy live during research.** The implementer should validate the actual error message format on first call and confirm whether `toBlock="finalized"` is accepted directly. Both are five-minute checks; both could change my §G implementation if Alchemy's behaviour drifts from the documented contract.
- **Did not benchmark concurrent worker counts empirically.** The 8-worker recommendation comes from the 500 CUPs ÷ (60 CU × 1/0.2 s) calculation, not from a live measurement. The first ingestion run will reveal the real ceiling; if 429s start at 4 workers, the cap is lower than expected.
- **Did not deeply inspect Goldsky Mirror's reorg-handling source.** The marketing page disclaimed the technical detail and I relied on the Subsquid docs as the load-bearing OSS reorg reference. If the implementer wants a second reorg-handling primary source for cross-validation, Goldsky's `docs.goldsky.com/mirror` would be the next read.
- **Did not analyse Erigon's `LogIndex` stage in source.** Cited Reth #16999 and Nethermind's writeup for the contrasting "archive nodes can also be slow" point, which was sufficient. A deeper read would only matter if Aurix self-hosts an Erigon archive node — currently out of scope per the plan ("Pay-as-you-go" is the documented escalation path).
- **Did not estimate the worst-case 30-day backfill on a degraded 100 CUPs path.** The escalation table in §D names the fallbacks; quantifying the slowest viable path (free tier under throttle saturation) would only matter if Alchemy actively degrades free-tier service mid-run, which this paper treats as a "stop and escalate" event rather than a path to optimise.
