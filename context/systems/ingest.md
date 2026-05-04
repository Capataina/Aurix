# Ingest

## Scope / Purpose

- Translates Ethereum archive data (live RPC via Alchemy, hosted Uniswap V3 subgraph, or synthetic mock) into rows in the [storage](storage.md) layer for downstream replay by [backtest](backtest.md).
- Supports three sources and a **tiered fallback chain** that lets Aurix run without paid API keys: subgraph (free, hosted) → user's Alchemy key (when configured) → free public RPC for the chain → empty state (never fabricated).

## Boundaries / Ownership

- Owns: archive log fetching, ABI decoding of V3 pool events (Swap, Mint, Burn, Collect), idempotent persistence into [storage](storage.md), per-pool ingestion checkpoints in `storage::state`, the synthetic-data generator (`MockArchiveSource`), the tiered-fallback orchestration in `Ingester::backfill`.
- Does **not** own: the data semantics (those live in [math](math.md) for sqrt-price/tick interpretation, and in [backtest](backtest.md) for replay). Ingest is deliberately a write-side façade.
- Composite hotspot 0.95 (highest in repo) — central to every Vector A flow.

## Current Implemented Reality

```text
ingest/
├── mod.rs              # Ingester, IngestionReport, ArchiveSource trait + tests
├── error.rs            # IngestError thiserror enum + KeyRequired variant
├── source.rs           # EthLog, ArchiveSource trait
├── decoder.rs          # ABI decoders for Swap/Mint/Burn/Collect (515 lines, 10+ tests)
├── alchemy.rs          # AlchemyArchiveSource — chunked eth_getLogs + finalized-block helpers
├── subgraph.rs         # UniswapV3SubgraphSource — hosted GraphQL backfill (per-chain × per-protocol URLs)
├── pipeline.rs         # Lower-level pipeline helpers
└── mock.rs             # MockArchiveSource — synthetic deterministic data
```

**`ArchiveSource` trait** (in `source.rs`) is the contract every source implements:

```rust
trait ArchiveSource {
    async fn fetch_logs(&self, pool: &str, from: u64, to: u64) -> Result<Vec<EthLog>, IngestError>;
    async fn latest_finalized_block(&self) -> Result<u64, IngestError>;
    async fn block_gas_price_gwei(&self, block: u64) -> Result<Option<f64>, IngestError>;
    async fn pool_metadata(&self, pool: &str) -> Result<PoolMetadata, IngestError>;
}
```

`Ingester::backfill(pool, from, to)` iterates the source, decodes each `EthLog` into `SwapEventRow` / `PoolEventRow` (via `decoder::decode_swap` etc.), and writes idempotently to [storage](storage.md). The pipeline is fully restartable — re-running over the same range is a no-op via `INSERT OR IGNORE`.

**Address-case normalisation.** Pool addresses are lowercased on insert (per [storage](storage.md)) — but the `EthLog.address` from Alchemy already arrives lowercase by chain convention. The user-supplied EIP-55-checksummed input from the LP page settings goes through `commands/lp.rs` and `delete_synthetic_swaps_in_range` at insert time. Lowering on the storage side is the canonical fix.

**Synthetic mode.** `MockArchiveSource` is constructed by `commands::lp::synthetic_mock` to generate a deterministic sinusoidal swap walk over the requested block range, anchored at tick `-195_580` (≈3000 USDC/WETH at the WETH(18)/USDC(6) layout). Each synthetic swap carries `transaction_hash = SYNTHETIC_TX_HASH` so live ingestion never overlaps; `delete_synthetic_swaps_in_range` purges only synthetic rows when re-running. The active liquidity in synthetic data is `1e17` (calibrated post-mortem in commit 391eadd: previously `1e12` over-attributed fees by ~5000×).

**Decoder details.** `decoder.rs` parses 32-byte hex words from `EthLog.data` per ABI conventions. Helpers:
- `parse_uint256` / `parse_int256` (two's-complement for negatives)
- `parse_uint160_word` (sqrtPriceX96)
- `parse_uint128_word` (liquidity, fee amounts)
- `parse_int24_word` (tick — sign-extension from 24 to 32 bits)

The four decoders (`decode_swap`, `decode_mint`, `decode_burn`, `decode_collect`) each verify the topic[0] keccak hash matches the expected signature constant (e.g. `SWAP_TOPIC0 = c42079f9...cca67`) and then unpack the data words per the event's Solidity ABI shape.

## Key Interfaces / Data Flow

```
commands::lp::run_lp_ingestion(pool, from, to, chain_id, protocol)
  └─ tiered fallback orchestrated in commands::lp:
     1. UniswapV3SubgraphSource::for_protocol(chain, proto) → Ingester::backfill
        ↓ on err → log + try next
     2. AlchemyArchiveSource::from_environment() (mainnet only) → Ingester::backfill
        ↓ on err → log + try next
     3. AlchemyArchiveSource::with_rpc_url(chain.public_rpc_url()) → Ingester::backfill
        ↓ on err → propagate to caller as CommandError
```

**`IngestionReport`** is the per-call summary returned through the IPC: pool address, block range, swap count, pool-event count, source identifier (subgraph / alchemy / public-rpc / mock), checkpoint advance.

**Three call sites for `Ingester::backfill`**:
- Live: `commands::lp::run_lp_ingestion` (3-tier fallback above)
- Synthetic: `commands::lp::run_lp_synthetic_ingest` (mock source only)
- Tests: `backtest::tests::build_synthetic_swaps` (direct mock construction)

## Implemented Outputs / Artifacts

- Rows in `swap_events` and `pool_events` tables ([storage](storage.md)).
- Per-pool ingestion checkpoint rows in `ingest_state`.
- Per-block gas-price rows in `block_gas` (when the source provides them).
- 10+ decoder tests + 6 pipeline tests + 3 `#[ignore]`d live-Alchemy integration tests in `ingest/tests`.

## Known Issues / Active Risks

- **Untested 4-tier extension at session-end.** Commit 391eadd's body explicitly flags Alchemy 400 ("key URL looks short — possibly truncated in .env") and Sushi/Pancake subgraph URL + pool-preset verification. The audit's [Today's Likely Focus #1](../plans/code-health-audit/index.md) and [potential issue #2](../plans/code-health-audit/potential-issues.md) both center on this.
- **`parse_int24_word` per-event allocation.** The decoder allocates a 32-byte `Vec<u8>` per int24 word and reads only 3 bytes. Recorded as a medium-severity perf finding in [audit findings](../plans/code-health-audit/ingest.md). Downstream: 100k swaps = 3.2 MB of allocator churn just on tick decoding.
- **Subgraph schema dependency.** `UniswapV3SubgraphSource` queries the hosted Uniswap V3 subgraph; if Uniswap deprecates the legacy hosted endpoint (they have signalled this in 2026), the legacy code path stops working. The `THE_GRAPH_API_KEY` gateway path is the migration target. Documented in `notes/lp-backtester-data-sources.md`.

## Partial / In Progress

- The 4-tier extension (subgraph + cross-chain + V3 forks + non-USD pools) shipped untested 2026-05-04 01:28; verifying each tier on the next session is a carry-forward.

## Planned / Missing / Likely Changes

- Verification of Tier 2 (Arbitrum / Optimism / Base / Polygon subgraph URLs + public RPCs).
- Verification of Tier 3 (Sushi V3 / Pancake V3 subgraph URL + protocol selector).
- Verification of Tier 4 (DefiLlama-fed token USD prices for non-USD pools).
- Resolution of the Alchemy 400 (truncated key in `.env`).

## Durable Notes / Discarded Approaches

- **Tiered fallback over hard-coded source.** Earlier design used Alchemy-only, requiring users to bring an API key. Moving to the subgraph-first → user-Alchemy → public-RPC tier closes the no-API-key UX gap that blocked dashboard usability for users without a wallet (the user has no wallet by project-preference; see `notes/lp-backtester-data-sources.md`). Documented in commit 391eadd.
- **Synthetic-vs-live separation via tx-hash sentinel.** Earlier design used a separate table for synthetic swaps; the sentinel-tx-hash approach simplifies the schema (one `swap_events` table) at the cost of needing the `delete_synthetic_swaps_in_range` purge step before re-ingesting tweaked synthetic data. Documented in commit 53f99eb's case-normalisation fix.
- **Hard-coded `1e17` synthetic active-liquidity.** Earlier value `1e12` over-attributed fee share by ~5000× because realistic position liquidity sits around `5.6e15` for a $10k position at ±3% range — the position would compute as 5600× the active liquidity, an unphysical fee share. Recalibrated to `1e17` in commit 391eadd. Recorded as [potential issue #1](../plans/code-health-audit/potential-issues.md) — the calibration vs realistic mainnet active-liquidity is unverified.

## Obsolete / No Longer Relevant

- None — every part of the ingest layer is in active use.

## Cross-references

- Producer for: `swap_events`, `pool_events`, `ingest_state`, `block_gas` rows in [storage](storage.md).
- Consumer of: [storage](storage.md) write API.
- Caller of: `decoder::*`, internal source impls.
- Used by: `commands::lp::run_lp_ingestion`, `commands::lp::run_lp_synthetic_ingest`.
- Related research: `references/ethereum-archive-log-ingestion.md`.
- Related notes: `notes/lp-backtester-data-sources.md`, `notes/free-data-fallback-chain.md`.
