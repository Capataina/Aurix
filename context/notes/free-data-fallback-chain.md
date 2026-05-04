# Free-Data Fallback Chain

## Current Understanding

Aurix uses **only free, no-API-key data sources** in user-facing flows, per the project's hiring-positioning constraint (the user has no wallet, so paid options that require wallet-signed key purchase are unreachable). The backend implements a tiered fallback chain so the LP backtester works without any user configuration, and degrades to an explicit empty state — never to fabricated synthetic numbers — when every live source fails.

This convention is **architecturally load-bearing**. Many code paths depend on it:
- `commands::lp::run_lp_ingestion` orchestrates the 3-tier ingest fallback.
- `commands::lp::lp_get_chain_head` follows the same pattern (user-Alchemy → public RPC).
- `commands::lp::lp_token_usd_prices` uses DefiLlama (free, no key).
- `benchmarks/tradfi.rs` swapped Stooq → FRED in commit 391eadd specifically because Stooq added a key requirement.

## The Chain

| Tier | Source | Auth | Rate-limit posture | When it fires |
|---|---|---|---|---|
| 1 | Uniswap V3 hosted subgraph (or fork's hosted equivalent) | Optional `THE_GRAPH_API_KEY`; hosted URL is no-key | Generous on hosted; per-second limits on gateway | Always tried first (the cheapest path) |
| 2 | User's Alchemy archive RPC | `ALCHEMY_API_KEY` / `MAINNET_RPC_URL` from `.env` (mainnet only) | Free tier: ~1k req/sec, range caps on `eth_getLogs` | Tier 1 failed AND a key is configured AND chain is mainnet |
| 3 | Free public RPC for the chain (chain-specific defaults) | None | Throttled but not auth-gated; sub-1-rps for personal volume | All key-bearing paths failed |
| 4 | Empty state with explicit error message | n/a | n/a | All live sources failed |

**No synthetic-data fallback in user-facing flows.** Synthetic ingest exists as a separate IPC (`commands::lp::run_lp_synthetic_ingest`) for unit tests + local-dev workflows. The auto-run pipeline does not call it. Empty state is the explicit terminal of the fallback chain — a fabricated number in a portfolio piece is a worse signal than an honest empty dashboard with a "needs setup" message.

## Provider Defaults

Per `src-tauri/src/config/chains.rs` (added in the 2026-05-03 sprint):

| Chain | Hosted subgraph | Public RPC |
|---|---|---|
| Ethereum | Uniswap V3 mainnet | LlamaRPC (`https://eth.llamarpc.com`) |
| Arbitrum | Uniswap V3 Arbitrum | Arbitrum's official RPC |
| Optimism | Uniswap V3 Optimism | Optimism's official RPC |
| Base | Uniswap V3 Base | Base's official RPC |
| Polygon | Uniswap V3 Polygon | Polygon's official RPC |

Sushi V3 + Pancake V3 forks reuse Uniswap V3's GraphQL schema; subgraph URLs differ per protocol. `chains::ChainConfig::subgraph_url_for(protocol)` resolves the right URL.

## Pricing & Benchmarks

- **DefiLlama coins API** (`https://coins.llama.fi/prices/current/<chain>:<addr>`) — free, no key, generous rate limits, sub-second response. Used for non-USD-quote-pool token prices and for the Aave V3 / Compound V3 / Lido APYs.
- **FRED `.txt` endpoints** (e.g. `SP500.txt`, `DGS3MO.txt`, `GOLDAMGBD228NLBM.txt`) — free, no key. Used for the TradFi benchmark series.
- **beaconcha.in ETH.STORE** — `KEY_REQUIRED` (no free tier for the staking-yield endpoint). Surfaces as `IngestError::KeyRequired("beaconchain")`. Frontend can prompt user to configure a key; without it, ETH.STORE is missing from the headline verdict but everything else works.

## Verification Question

- Does every IPC in `commands/lp.rs` end with an empty-state path on total failure rather than synthetic fallback? (Answered yes as of 2026-05-04 — the 4-tier extension's `run_lp_ingestion` and `lp_get_chain_head` both propagate `CommandError` to the frontend instead of falling through to mock.)

## Guiding Principles

- New IPCs that need live data follow the same tier ordering: cheapest free source first, key-bearing paths in the middle, public-tier last, empty state on terminal failure.
- Synthetic data may exist for testing but **never** in the auto-run user-facing flow.
- API-key requirements are surfaced via the `KeyRequired(name)` error variant so the frontend can prompt structurally rather than parsing error messages.
- When a free source is deprecated (Stooq → FRED), the migration is structural (swap the URL family + the parser) rather than degrading to a paid alternative.

## Cross-references

- Related notes: `notes/lp-backtester-data-sources.md` (per-IPC data-source mapping, the most detailed view).
- Systems: [ingest](../systems/ingest.md), [benchmarks](../systems/benchmarks.md).
- Audit findings: [potential-issues.md §2](../plans/code-health-audit/potential-issues.md) (Alchemy 400 carry-forward).
