# LP Backtester Data Sources

The LP backtester's auto-run pipeline reads live swap data from a tiered fallback chain. Each tier degrades gracefully to the next on failure; the chain ends in an explicit empty state, **not** synthetic data.

Caner's stance on cost: no paid API options. Free sources only. The Graph's decentralized gateway requires a wallet to obtain an API key; Caner has no wallet, so the gateway is effectively unreachable today and the legacy hosted URL is the no-key path.

## Fallback chain

| Tier | Source | Auth required | Speed | When it fires |
|------|--------|---------------|-------|---------------|
| 1 | Uniswap V3 subgraph (The Graph) | Optional `THE_GRAPH_API_KEY` for the gateway; legacy hosted URL otherwise | Fast (single GraphQL query, no chunking) | Always tried first |
| 2 | User's Alchemy archive RPC | `ALCHEMY_API_KEY` / `MAINNET_RPC_URL` from `.env` (Ethereum mainnet only) | Medium (`eth_getLogs` chunked; auto-fallback to 10-block chunks on free-tier 400s) | Tier 1 failed and a key is configured |
| 3 | Free public RPC (chain-specific default) | None | Slow on free-tier range caps; the adaptive chunk-size logic in `get_pool_logs` shrinks to 10 blocks on first 400/429 | All key-bearing paths failed |
| 4 | Empty state with explicit error message | n/a | n/a | All live sources failed |

**No synthetic-data fallback in user-facing flows.** Synthetic ingest (`run_lp_synthetic_ingest`) remains as a Tauri command for unit tests and local-dev workflows, but the auto-run pipeline does not call it. Empty state is preferable to fabricated numbers in a hiring-portfolio piece.

## Per-IPC mapping

- `run_lp_ingestion`: subgraph → user-Alchemy (mainnet only) → public RPC → error.
- `lp_pool_metadata`: subgraph only (single GraphQL query for token0/token1 + decimals + symbols + fee tier).
- `lp_query_first_swap_price`: storage-only read after ingest lands.
- `lp_get_chain_head`: user-Alchemy (mainnet only) → chain's public RPC → error.
- `lp_token_usd_prices`: DefiLlama `https://coins.llama.fi/prices/current/<chain>:<addr>,...` — no key, generous limits, sub-second responses.
- `lp_fetch_benchmark_series`: FRED `.txt` endpoints (`SP500.txt`, `DGS3MO.txt`, `GOLDAMGBD228NLBM.txt`) — no key — for S&P 500 / 3-mo T-bill / gold; DefiLlama for Aave V3 USDC + Lido stETH APY.

## Cross-chain support

`ChainId` enum in `src-tauri/src/config/chains.rs` covers Ethereum / Arbitrum / Optimism / Base / Polygon. Each chain has its own subgraph URL, public RPC, block-time, and native gas-token symbol. The chain selector in LP settings drives which row of the table the IPCs read.

## V3-protocol routing

`Protocol` enum (also in `chains.rs`) covers UniswapV3 / SushiswapV3 / PancakeswapV3. Subgraph URL is `chain.subgraph_url_for(protocol)`. The forks share Uniswap V3's GraphQL schema so the existing queries work unchanged; only URLs differ. Pool presets in `src/features/lp-backtest/pools.ts` cover the Uniswap V3 footprint plus a few Sushi V3 + Pancake V3 entries (the latter need verification — picked from public hosted-service convention).

## API keys (all optional)

- `THE_GRAPH_API_KEY` — gateway preferred when set; falls through to legacy hosted URL when absent. Requires wallet to obtain.
- `ALCHEMY_API_KEY` / `MAINNET_RPC_URL` — Tier 2 path; mainnet only. **Note:** the value currently in `.env` (`QNZ1oqj_e9R6izhNcz_9X`, ~21 chars) is shorter than typical Alchemy keys (32+ chars) and returns 400 on `eth_getLogs` — investigate whether it was truncated when the env was set up. Public RPC fallback works regardless.
- `COINGECKO_API_KEY` — not currently used; DefiLlama covers prices without a key.

## Why this shape

- **Subgraph first** — even when keyed, eth_getLogs over a 1000-block range can hit per-call caps on free tiers; the subgraph indexes everything in advance and serves a paginated GraphQL query in one round-trip.
- **Public RPC last (not synthetic)** — public RPCs (LlamaRPC for ETH, Arbitrum's official RPC, Optimism's etc.) are throttled but not auth-gated; they always work for personal-volume traffic. Synthetic data is *not* a fallback because rendering fake numbers in a portfolio piece is a worse signal than an empty dashboard.
- **Per-token USD pricing** — non-USD-quote pools (WBTC/ETH, LDO/ETH) cannot be valued via the in-pool ratio alone; DefiLlama's coins endpoint provides the external feed. Engine refactor (`position_usd_value_explicit` in `src-tauri/src/backtest/price.rs`) uses these prices when both are supplied; falls back to the in-pool ratio with token1=USD assumption when not.

## Telemetry visibility

Every IPC call routes through `loggedInvoke` in `src/lib/telemetry.ts`, so failures in any of the tiers above land in `~/Library/Logs/com.ataca.aurix/last-session.json` as `ipc.error` events with the full backend error message. Diagnose without screenshots:

```bash
jq '.events[] | select(.type == "ipc.error")' ~/Library/Logs/com.ataca.aurix/last-session.json
```
