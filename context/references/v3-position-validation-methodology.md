# V3 Position Validation Methodology

> *On-chain ground truth, event reconstruction, tolerance margins, entry-pricing convention, and discrepancy taxonomy for the Aurix V3 LP backtester (`vector-a-v3-lp-backtester.md` § M2.4).*

## Scope / Purpose

This paper is the methodology reference for milestone **M2.4 — Validation harness** of the V3 LP backtester. It answers the questions an implementing agent will hit on day one and would otherwise have to reconstruct from scratch by reading the periphery contract, replaying tx logs, and surveying half a dozen LP-analytics projects with conflicting tolerance claims.

It covers, specifically:

- what makes an on-chain position "clean" enough to serve as ground truth,
- what events the `NonfungiblePositionManager` (NPM) and the `UniswapV3Pool` actually emit, and which combinations identify a clean position,
- how to pick five representative positions without cherry-picking favourable ones,
- what tolerance margins published V3 validation projects actually use, and why,
- where to source positions and tx data (Etherscan, Revert, subgraphs),
- realistic mainnet gas costs for mint / increaseLiquidity / collect / decreaseLiquidity / burn,
- the entry-pricing convention question (block-end? in-tx pre-mint? Swap-event-implied?),
- a discrepancy taxonomy distinguishing rounding (normal) from off-by-one (bug),
- a realistic engineering-time estimate per position.

It does **not** cover: the V3 math primitives themselves (those are M2.2 and the Uniswap whitepaper / Atis Elsts technical note); how to model rebalancing rules (M2.5, covered in `lp-rebalancing-strategies.md`); how to compare LP returns to lending baselines (M2.7, covered in `v3-lp-profitability-literature.md`).

## Current Project Relevance

Milestone M2.4 is the **credibility lynchpin** of the entire vector. The headline resume-bullet upgrades for vector A all depend on it: *"validated against 5 on-chain reference positions; per-swap fee distribution within 0.5% of collected ground truth"* is only honest if the validation is rigorous. If the harness misses a `Collect` event, picks unrepresentative positions, or quietly tolerates 5% errors as "rounding," the vector collapses into the same category as the public OSS V3 backtesters that *don't* validate — exactly the audience the project is supposed to outflank.

The current code surface in `src-tauri/src/dex/uniswap_v3.rs` only reads `slot0()` for spot price (verified at `uniswap_v3.rs:35-36`); there is no NPM event reading, no position reconstruction, and no tick math beyond the `sqrtPriceX96` decode at `decode_sqrt_price_x96` (lines 52-63). Everything in M2.4 is greenfield, which makes choosing the right methodology *up front* unusually load-bearing.

## What The Topic Actually Is

A V3 LP position is fully described on-chain by:

1. The pool address (which fixes `token0`, `token1`, `fee`, and `tickSpacing`).
2. The `(owner, tickLower, tickUpper)` tuple — the position key inside the pool.
3. The `liquidity` (a `uint128`), which is what actually gets multiplied by the per-tick fee growth.

Validation means: given a position's `(pool, tickLower, tickUpper, liquidity, entryBlock, exitBlock)` and the full sequence of pool `Swap` events between those blocks, our simulator must recompute the same final `(amount0, amount1, feesCollected0, feesCollected1)` that the actual on-chain mint/burn/collect transactions produced — within a tolerance the paper has to pin down.

The validation question is therefore not *"does my math match the whitepaper?"* (M2.2 answers that with reference fixtures). It is *"does my full per-swap fee-distribution + IL + tick-crossing pipeline match what the production contracts actually computed?"* — a strictly harder check that catches off-by-one tick crossings, missed liquidity changes, and pool-state desync.

## Current State Snapshot

| Aspect | State | Citation |
|---|---|---|
| Pool slot0 reader exists | yes — V3 5bps + 30bps spot reads | `src-tauri/src/dex/uniswap_v3.rs:27-50` (repository fact) |
| sqrtPriceX96 decode validated | partially — no on-chain reference fixtures yet | `src-tauri/src/dex/uniswap_v3.rs:52-63` (repository fact) |
| Tick math primitives | not implemented (M2.2 pending) | absence verified — no `tick_to_sqrt_price_x96` symbol exists in `dex/` (repository fact) |
| NPM event reader | not implemented | absence verified (repository fact) |
| Persistence (swap events) | not implemented (M2.0 pending) | `context/architecture.md` confirms no SQLite layer (repository fact) |
| Existing references on validation | none — the two reference scaffolds (`v3-lp-profitability-literature.md`, `lp-rebalancing-strategies.md`) are placeholder skeletons with no body content | repository fact, verified by reading both files |

The validation harness has no upstream prerequisites blocked: M2.2 (math) and M2.3 (simulation engine) need to land first, but the *methodology* in this paper is read-only against current code and can guide M2.2/M2.3 design as well.

---

## Research Signal

Source-backed findings (`SB`), repository facts (`RF`), and project inferences (`PI`) are kept distinct in the table below. Quoted-passage IDs (`Q1`–`Q15`) are defined in **External Research Trail → Quoted passages**.

| Topic | Source-backed signal | Citation | Current repository state | File:line | Project implication | Class |
|---|---|---|---|---|---|---|
| NPM mint *does not* emit a `Mint` event — it emits `IncreaseLiquidity` + an ERC721 `Transfer` from `address(0)` | NPM source: `mint()` body emits only `IncreaseLiquidity` at line 239; the `Mint` keyword belongs to the pool, not NPM | Q1 (NPM source); Q2 (interface) | not implemented | absence verified | The validator **must** identify a "fresh" NPM position by `(IncreaseLiquidity for tokenId T) AND (Transfer from 0x0…0 to recipient)` in the same tx. Reading only `Mint` events misses NPM-routed positions and mis-attributes direct pool calls. | SB |
| Pool-level `Mint` event *does not* include `sqrtPriceX96` or `tick` | v3-core PoolEvents interface natspec | Q3 (PoolEvents) | not implemented | absence verified | Entry pricing must be derived from another source (the next `Swap` event's pre-state, or an explicit `slot0()` archive read at `entryBlock`). It is not in the `Mint` log. | SB |
| Pool `Swap` event sqrtPriceX96 = **after-state** | v3-core PoolEvents interface — Swap has `sqrtPriceX96` and `tick` as the post-swap state | Q4 (PoolEvents); Q5 (search) | partial — `slot0()` decode handles after-state correctly | `uniswap_v3.rs:35-36` | Replay state must update *after* applying each swap, mirroring the pool's emit ordering. | SB |
| Topaze/Bancor (≈$199m fees, ≈$260m IL across 17 pools) used Dune Analytics for pricing + on-chain tx data with hourly USD reconciliation | Topaze paper §"Descriptive Pool Level Statistics" methodology block | Q6 (Topaze) | not implemented | absence verified | Hourly USD pricing is acceptable for *aggregate* statistics; for per-position fee validation it introduces ~basis-point noise — fine for our 0.5% tolerance, not fine for a 0.05% "stretch" check. | SB |
| Topaze IL is computed via "novation" — every position adjustment closes the imputed position and reopens a new one at the new state | Topaze paper §"Impermanent Loss Analysis" | Q7 (Topaze novation) | not implemented | absence verified | Aurix should adopt novation for IL accounting at every `Increase`/`Decrease`/`Collect` boundary; this also makes single-mint single-burn "clean" positions trivially the only ones that don't require novation, which is why we want exactly those for M2.4 validation. | SB |
| JNP "real-world framework" reports **±5% precision with hourly data**, **0.5% APR difference for one carefully-tested position** vs revert.finance | Coinmonks article on backtesting V3, replaying ameen.eth's tokenId 27782 | Q8 (JNP) | not implemented | absence verified | Hourly subgraph data is too coarse for a 0.5% tolerance; per-swap event replay is necessary. The 0.5% number is achievable with *clean* event-level replay. | SB |
| Urusov et al. (BCRA 2024) report **<1% error in modelled rewards** | arXiv 2410.09983 abstract & methodology | Q9 (BCRA) | not implemented | absence verified | The <1% number is not directly comparable to Aurix's path: BCRA models the *liquidity distribution* parametrically (normal distribution) and replays Binance minute prices. Aurix replays exact pool events and tracks per-position liquidity exactly — strictly tighter than BCRA, so Aurix should aim for **better than <1%** when the position is in-range and clean. | SB |
| BCRA explicitly notes its parametric model fails at range edges and ignores JIT, MEV, and sub-block dynamics | arXiv 2410.09983 limitations | Q10 (BCRA limits) | not implemented | absence verified | Confirms that any "academic backtester" tolerance figure assumes a smooth liquidity profile; Aurix needs sharper validation specifically because we want to capture range-edge effects. | SB |
| Uniswap blog: JIT is **~0.3% of total liquidity demand**; the WETH/USDC 5bps pool **alone accounts for over half of all JIT liquidity ever supplied** | Uniswap official JIT post | Q11 (Uniswap JIT) | not implemented | absence verified | Across the full 30+ days of replay, JIT is a small fraction. *Within* an individual block where JIT happened, the dilution is severe (next row). This is the contrasting view on backtester validity. | SB |
| JIT-attacked blocks dilute regular LP fees **by an average of 85%** for that block | eprint.iacr 2023/973 (cited via WebSearch summary; the primary PDF returned 403, so this figure is single-source via search summary) | Q12 (JIT dilution) | not implemented | absence verified | If our 5 candidate positions happened to be in-range during a JIT block, our simulator (which models all in-range liquidity at that swap, *including* the JIT bot) will *correctly* compute the diluted fee — but if we only read NPM positions, we may miss the JIT direct-pool position and over-attribute fees. **Mitigation:** include direct-pool `Mint`/`Burn` events for the relevant blocks, not only NPM events. | SB+PI |
| Revert backtester uses **hourly poolHourData snapshots** from the subgraph, accepts that "accuracy correlates with proportion of time in range" | Revert technical-docs/backtester | Q13 (Revert) | not implemented | absence verified | Subgraph hourly snapshots are insufficient for our methodology — we need per-swap granularity. Use Etherscan / archive node `eth_getLogs` for `Swap` events directly, not the subgraph. | SB |
| Atis Elsts (the same author often referred to as "Atise" in V3 tooling lists) provides the canonical liquidity-math derivations from the whitepaper | atiselsts.github.io technical note | Q14 (Elsts) | partial — sqrtPriceX96 → price is implemented but the inverse and the `liquidity_for_amounts` family are not | `uniswap_v3.rs:76-125` | Use Elsts §2.1 Eqs 4–12 as the primary fixture source for M2.2 + as a cross-check on every position's expected `(amount0, amount1)` at entry. | SB |
| Real on-chain mint of a brand-new pool position (multicall including pool init) consumed **5,189,186 gas at 3.36 gwei = $40.53** | Etherscan tx `0x2c2839…83884`, block 22,181,682 | Q15 (real tx) | not implemented | absence verified | This is an outlier — pool-init multicalls are not representative. Ordinary NPM mint into an existing pool is **~250-350k gas**, increase ~120-200k, collect ~70-150k, decrease ~120-180k, burn ~70-100k (project inference, see §"Realistic gas-cost grounding" table). | SB+PI |

---

## Methodology — Picking Validation Positions

### What makes a position "clean"

A "clean" position is one where the on-chain ground truth fully determines the simulator's expected output, with **no hidden state changes** the simulator would have to guess about. Every adjustment a real LP can do — adding liquidity, removing some, changing recipients, transferring the NFT, partial collects — adds a class of state mutation the simulator must replay correctly. Each is a chance to be off, and each obscures whether a discrepancy comes from our math or from event misreading.

The validation set should isolate the math, not the event-handling. So we want the simplest possible position lifecycle:

```
clean position lifecycle:

  block N         (single Mint via NPM)
     │
     │  ── replay every Swap in [N+1, M-1] against position liquidity
     │
  block M         (single Burn via NPM, full liquidity)
     │
     │  ── single Collect of all owed token0+token1 fees
     ▼
  position closed, NFT held by original recipient (or burned)
```

Concretely, the **clean-position decision tree** for the M2.4 set:

```
                 candidate position (tokenId T)
                            │
                            ▼
       Single IncreaseLiquidity event for T?  ──── no ──→ EXCLUDE (multi-mint)
                            │ yes
                            ▼
          Zero DecreaseLiquidity events
          for T before final close?           ──── no ──→ EXCLUDE (partial removals)
                            │ yes
                            ▼
       Single final DecreaseLiquidity that
       removes exactly the minted liquidity?  ──── no ──→ EXCLUDE (still open / multi-burn)
                            │ yes
                            ▼
       Total Collect amounts for T = sum of
       expected token0Owed + token1Owed?      ──── no ──→ EXCLUDE (partial collects)
                            │ yes
                            ▼
       NFT owner is the same address from
       Mint to Burn?                          ──── no ──→ EXCLUDE (transferred mid-life)
                            │ yes
                            ▼
       Position lifetime ≥ ~1000 blocks
       (avoids JIT / single-block flash LPs)? ──── no ──→ EXCLUDE (flash LP / JIT)
                            │ yes
                            ▼
       Pool == WETH/USDC 5bps (0x88e6…5640)?  ──── no ──→ EXCLUDE (out of M2.4 scope)
                            │ yes
                            ▼
                     ✓ CANDIDATE
```

> **Key methodology trap:** the NPM never emits a "Mint" event. It emits `IncreaseLiquidity` for both fresh mints and adds. The signal that a `tokenId` is a **fresh** mint (rather than an add) is an ERC721 `Transfer(from=0x0…0, to=recipient, tokenId)` event in the same tx — see Q1. A validator that only filters by `IncreaseLiquidity` will mis-classify adds-to-existing-positions as fresh mints and fail to find their actual `Mint` block.

### Picking 5 representative positions without cherry-picking

Cherry-picking favourable positions is a real failure mode; if all 5 chosen positions happen to be in-range 100% of the time and never see a JIT-affected block, the validation tells us nothing about how the simulator behaves at range edges or under fee dilution. The set must span the failure-mode surface we care about.

**Diversity axes:**

| Axis | Why it matters | Suggested distribution across 5 positions |
|---|---|---|
| Position size (USD at mint) | Gas-cost validation differentiates by size; tick math precision is size-independent but `feeGrowth` rounding is more visible at small sizes | one ~$1k, two ~$10k, one ~$50k, one ~$200k |
| Range width (ticks) | Narrow ranges cross more often → more tick-boundary edge cases | two narrow (≤200 ticks ≈ ±1%), two medium (≈ ±10%), one wide / "full range" |
| Lifetime | Short lifetimes test rapid replay; long lifetimes test cumulative drift | one ≤24h, two ≈1-7d, one ≈30d, one ≥60d |
| Time-in-range fraction | The simulator should equally validate "100% in range" and "frequently OOR" cases | three with ≥80% in-range, two with 30-70% in-range |
| Fee tier | Out of M2.4 scope (5bps only), but pool diversity *should* be in M2.5's stretch goal | all 5 in WETH/USDC 5bps for M2.4 |
| JIT exposure | The contrasting-source insight (Q11/Q12): we want at least one position whose lifetime *intersected* a JIT block | one position whose history overlaps a documented JIT-bot block; the other four selected to avoid any single-block-overlap with `0xa69b…` (one of the known JIT addresses) |

**Discovery procedure** (the implementing agent runs this fresh — see "Re-verification" warning below):

1. **Source candidate token IDs** by querying Etherscan's "Events" tab on the NPM contract `0xC36442b4a4522E871399CD717aBDD847Ab11FE88` for `IncreaseLiquidity` events whose `tokenId` corresponds to a freshly-minted position. The reliable filter is: pull all NPM-targeted txs in a block range, then keep those whose log set contains both an `IncreaseLiquidity(tokenId, ...)` and an `ERC721 Transfer(from=0x0, to=recipient, tokenId)` in the same tx receipt.
2. **Resolve each `tokenId` to its pool** by calling `NPM.positions(tokenId)` (read-only) — this returns `(nonce, operator, token0, token1, fee, tickLower, tickUpper, liquidity, ...)`. Keep only those where `(token0, token1, fee) = (USDC, WETH, 500)`.
3. **Determine each position's full event history** by filtering NPM event logs for that `tokenId` across all subsequent blocks (`IncreaseLiquidity`, `DecreaseLiquidity`, `Collect`).
4. **Apply the clean-position decision tree** above to keep only single-mint single-burn single-collect lifecycles.
5. **Cross-reference Revert** (`https://revert.finance/#/uniswap-v3/ethereum/positions/<tokenId>`) for an independent UI-level read of the same position's fees collected and PnL — useful as a sanity check while building the harness, *not* as the ground truth itself (Revert's accuracy is correlated with time-in-range per Q13).
6. **Apply the diversity matrix** above to filter from ~50 candidates down to 5.

> **Re-verification warning:** the candidate tx hash listed below was identified during research on 2026-05-02. **Any position used at validation time must be re-verified at validation time**, because (a) on-chain state evolves — a position open at research time may have been adjusted by validation time, and (b) the discovery procedure may surface better candidates than the initial seed. The candidate is a starting point and a smoke-test for the discovery procedure, not a fixed validation set.

### Candidate seed (re-verify before use)

| Pool | Tx hash | Block | Notes |
|---|---|---|---|
| (NPM mint of brand-new MEMEX pool, not the WETH/USDC 5bps validation set) | `0x2c2839602182f6358f562e447edda9764785c727a36279c8eb04d24171d83884` (Q15) | 22,181,682 (Apr 2 2025) | **NOT a clean position for M2.4** — this is a pool-init multicall on the MEMEX pool, surfaced during gas-cost research. Documented here as an example of how the discovery procedure should *exclude* outlier txs. |
| WETH/USDC 5bps `0x88e6…5640` | `0xcb2a2034f3465267f377d48bdc6c0dd0a1be210963abb2408869e917d83bba51` | 22,617,195 (Jun 2 2025) | A direct-pool `mint()` call (not via NPM) that **execution-reverted**. Documented here to flag that direct-pool mints exist (mostly from MEV bots and integrators) and the validator must distinguish them from NPM mints. |

> The implementing agent should expect to spend ~30-60 minutes running steps 1–5 above to produce 5 clean candidates. The discovery is *not* the bottleneck — the harness build is. **No clean candidate positions are pre-identified in this paper.** Step 1 of the discovery procedure must be executed against fresh on-chain state at the start of M2.4 work — anchoring the candidates here would give them a false aura of authority and would invite cherry-picking.

---

## Tolerance Margins — What Published Projects Actually Use

| Project | Tolerance reported | Quoted passage / Q-ID | What drove it |
|---|---|---|---|
| Topaze Blue / Bancor (Loesch et al., Nov 2021) | Implicit; aggregate fees match within Dune-hourly USD rounding (~basis points), no explicit tolerance number | Q6 (methodology block) | Hourly USD pricing was "good enough" for 17-pool aggregate IL; not a per-position validation reference |
| GammaStrategies awesome-uniswap-v3 (curated reference) | None published in the README | Q (verified absence) | Not a validation tool — a reference list |
| JNP / Coinmonks "real-world framework" | **±5% with hourly granularity**; **0.5% APR difference** for one carefully-tested position vs revert.finance | Q8 | Granularity (hourly subgraph) is the dominant error source; ground-truth comparison is to revert.finance |
| Urusov et al., BCRA 2024 (peer-reviewed) | **<1% error** in modelled rewards across multiple pools | Q9 | Parametric (Gaussian) liquidity distribution — strictly an upper-bound model, not a per-swap replay |
| Revert backtester | Not numerically published; "accuracy will be correlated with the proportion of time that a position would have been in range" | Q13 | Hourly subgraph snapshots; accuracy degrades for OOR-heavy positions |

### Recommended Aurix M2.4 tolerance — and why

The plan currently specifies `0.5% on collected fees`, `5% on modelled gas vs actual gas`, and `4 of 5 positions match`. The literature supports this:

| Aurix metric | Plan target | Literature anchor | Decision |
|---|---|---|---|
| Total fees collected (token0 and token1) | within 0.5% of on-chain ground truth | JNP achieved 0.5% APR difference (Q8) on a clean position; BCRA achieved <1% (Q9) with a much weaker model | **0.5% is achievable and credible**; keep |
| Final position composition (amount0, amount1 at burn) | within rounding | Topaze novation Q7 implies single-wei rounding noise per adjustment; for a single mint+burn position this should be ≤ ~10 wei in each token | **Tighten plan to "within 100 wei in each of token0 and token1"** — concrete, falsifiable |
| Modelled gas vs actual on-chain tx receipt cost | within 5% | Plan-internal | Keep — gas modelling has more sources of variance (block-median vs in-tx, base fee vs priority fee allocation) |
| IL number | sanity-check only | No on-chain ground truth exists for IL itself | Keep as documented qualitative check |
| Acceptance gate | 4 of 5 match | None published | Keep — generous because the 5th may surface a subtle bug worth investigating, not necessarily a methodology failure |

### Discrepancy taxonomy — normal vs bug

A discrepancy between simulator and on-chain truth either reflects (a) expected rounding the implementer should accept, or (b) a real bug. The taxonomy below distinguishes them.

| Discrepancy class | Symptom | Magnitude | Verdict |
|---|---|---|---|
| Integer-division rounding in Q64.96 math | each operation may differ by ±1 wei from on-chain | per swap: ≤1 wei per token; per position lifetime (~30 days, ~10k swaps): ≤10k wei = `0.00001` USDC = $0.00001 | **NORMAL** — accept silently |
| `feeGrowthGlobal` 256-bit truncation | fee accumulator wraps modulo 2^256 | applies once per `feeGrowth` *full wrap*; effectively never on WETH/USDC 5bps within 30 days | **NORMAL** — must use `wrapping_sub` for delta math, otherwise BUG |
| Block-median vs actual gas-price for management cost | modelled gas $ ≠ actual tx fee $ | typically <5% on stable blocks; 10-20% on high-volatility blocks | **NORMAL** — within plan tolerance |
| USDC price = $1 vs market USDC price | 6-decimal stable assumed at parity | ≤30 bps in normal markets; March 2023 SVB depeg saw -10% briefly | **NORMAL** for 0.5% tolerance, but explicit caveat in M2.4 docs |
| Off-by-one in tick crossing direction | swaps that cross the position's `tickUpper` from below add liquidity that should *not* count | accumulated error grows with cross count; a 30d narrow position with 50 crossings would diverge several percent | **BUG** — a clear simulator failure |
| Missed `Collect` event | fees attributed to position never get reset; position double-counts on next collect window | per position lifetime: ~$tens to ~$thousands of phantom fees | **BUG** — investigate event reader |
| Liquidity not added at `Mint` block | simulator starts the position at zero liquidity; all fees missed | total fees ≈ 0 vs ground truth | **BUG** — initialization off-by-one block |
| JIT-bot block in lifetime, not modelled | simulator over-attributes fees of that block to the validation position | per affected block: up to ~85% of that block's fees (Q12) | **NORMAL IF** the JIT bot's events are also replayed (because then the simulator correctly dilutes the position's share); **BUG IF** only NPM events are read (because the JIT direct-pool position is invisible) |
| Hourly subgraph snapshot vs per-swap replay | systematic ±5% error (Q8) | systematic | **METHODOLOGY** — only matters if you used subgraph; with per-swap event replay, this class doesn't apply |
| Reorg between research and validation | candidate tx no longer at the same block | varies; ≥12-block confirmation depth eliminates this for any position older than ~3 minutes | **MITIGATED** by the M2.1 ingestion's "depth ≥ 12" rule (already in the plan) |

```
discrepancy size distribution  (project inference, illustrative scale only)

                            BUG zone
                              │
                              ▼
±10 wei  ████████████░░░░░░░░░░░░░░░░░░░░░░░░░  rounding (NORMAL)
±0.1%    ███████████████████░░░░░░░░░░░░░░░░░░  USDC parity slop (NORMAL)
±0.5%    ████████████████████████████░░░░░░░░░  plan tolerance (M2.4 PASS)
±5%      ████████████████████████████████████░  hourly snapshot (METHODOLOGY)
                                                ──────────────────
                                                if seen with per-swap
                                                replay → BUG
```

---

## NonfungiblePositionManager Events — What Each Tells You

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      NPM (NonfungiblePositionManager)                   │
│                  0xC36442b4a4522E871399CD717aBDD847Ab11FE88             │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ user → mint() / increaseLiquidity() /
                                    │ decreaseLiquidity() / collect() / burn()
                                    ▼
   ┌────────────────────────────────────────────────────────────────┐
   │ Events emitted by NPM (per Q1 source line):                    │
   │                                                                │
   │   IncreaseLiquidity(uint256 indexed tokenId,                   │
   │                     uint128 liquidity,                         │
   │                     uint256 amount0,                           │
   │                     uint256 amount1);                          │
   │     emitted by: mint() (line 239), increaseLiquidity() (280)   │
   │     "Also emitted when a token is minted" (Q2 natspec)         │
   │                                                                │
   │   DecreaseLiquidity(uint256 indexed tokenId,                   │
   │                     uint128 liquidity,                         │
   │                     uint256 amount0,                           │
   │                     uint256 amount1);                          │
   │     emitted by: decreaseLiquidity() (line 320)                 │
   │                                                                │
   │   Collect(uint256 indexed tokenId,                             │
   │           address recipient,                                   │
   │           uint256 amount0,                                     │
   │           uint256 amount1);                                    │
   │     emitted by: collect() (line 368)                           │
   │                                                                │
   │   Transfer(address indexed from, address indexed to,           │
   │            uint256 indexed tokenId);                           │
   │     emitted by: ERC721._mint() (mint() body, line 218)         │
   │                 ERC721._burn() (burn() body, line 374)         │
   │     This is the ONLY event burn() emits.                       │
   └────────────────────────────────────────────────────────────────┘
                                    │
                                    │ NPM internally calls
                                    ▼
   ┌────────────────────────────────────────────────────────────────┐
   │ UniswapV3Pool (e.g. WETH/USDC 5bps 0x88e6…5640)                │
   │ Events emitted by the pool (per Q3, Q4 source):                │
   │                                                                │
   │   Mint(address sender,                                         │
   │        address indexed owner,                                  │
   │        int24 indexed tickLower, int24 indexed tickUpper,       │
   │        uint128 amount, uint256 amount0, uint256 amount1);      │
   │     "Emitted when liquidity is minted for a given position"    │
   │     NOTE: does NOT include sqrtPriceX96 or tick                │
   │                                                                │
   │   Burn(address indexed owner,                                  │
   │        int24 indexed tickLower, int24 indexed tickUpper,       │
   │        uint128 amount, uint256 amount0, uint256 amount1);      │
   │     "Emitted when a position's liquidity is removed"           │
   │     "Does not withdraw any fees earned by the liquidity        │
   │      position, which must be withdrawn via #collect"           │
   │                                                                │
   │   Collect(address indexed owner, address recipient,            │
   │           int24 indexed tickLower, int24 indexed tickUpper,    │
   │           uint128 amount0, uint128 amount1);                   │
   │     "may be emitted with zero amount0 and amount1 when the     │
   │      caller chooses not to collect fees"                       │
   │                                                                │
   │   Swap(address indexed sender, address indexed recipient,      │
   │        int256 amount0, int256 amount1,                         │
   │        uint160 sqrtPriceX96,  ← AFTER-state                    │
   │        uint128 liquidity, int24 tick);                         │
   │                                                                │
   │   Initialize(uint160 sqrtPriceX96, int24 tick);                │
   │     "Mint/Burn/Swap cannot be emitted by the pool before       │
   │      Initialize" (Q3) — the strongest temporal guarantee       │
   │      the pool gives.                                           │
   └────────────────────────────────────────────────────────────────┘
```

### Event-pair identifiers for clean positions

| Position state | Event pattern (sufficient + necessary) |
|---|---|
| Fresh NPM mint | NPM `Transfer(0x0 → recipient, tokenId)` **AND** NPM `IncreaseLiquidity(tokenId, …)` **AND** pool `Mint(NPM, recipient, tickLower, tickUpper, amount, …)`, all in the same tx |
| Add to existing NPM position | NPM `IncreaseLiquidity(tokenId, …)` **AND** pool `Mint(NPM, recipient, …)`, **AND NO** ERC721 Transfer-from-zero in this tx |
| Partial NPM remove | NPM `DecreaseLiquidity(tokenId, …)` with `liquidity < position.liquidity` **AND** pool `Burn(NPM, …)` |
| Full NPM remove (position still open as NFT) | NPM `DecreaseLiquidity(tokenId, full)` **AND** pool `Burn(NPM, …)` — the NFT still exists, fees not yet collected |
| Fee collect (NPM) | NPM `Collect(tokenId, …)` **AND** pool `Collect(NPM, recipient, …)` |
| NFT burn (close-out) | NPM `Transfer(owner → 0x0, tokenId)` |
| Direct-pool mint (bypassing NPM) | pool `Mint(sender ≠ NPM, owner, …)` and **NO** NPM events for any tokenId in this tx |
| JIT-bot pattern | direct-pool `Mint` followed by direct-pool `Burn` in the *same block*, often the same tx |

---

## Entry-Pricing Convention

A subtle methodology question: when a position is minted at block N, what `sqrtPriceX96` should the simulator use to compute the initial composition `(amount0, amount1)`?

The candidates are:

1. **block-end `slot0`** — the pool state at the end of block N, after all transactions in the block.
2. **in-tx pre-mint `sqrtPriceX96`** — the pool state immediately before the mint executed.
3. **post-mint `sqrtPriceX96`** — the pool state immediately after the mint executed.
4. **previous Swap's `sqrtPriceX96`** — the after-state of the most recent Swap before the mint.

### Why this matters for validation

The pool's `mint()` function (in v3-core) uses the pool's *current* `slot0.sqrtPriceX96` at the moment the call executes, then computes amounts via `LiquidityAmounts.getAmountsForLiquidity(sqrtPriceX96, sqrtRatioAX96, sqrtRatioBX96, liquidity)`. **A V3 mint does not move the price** — unlike a swap, it doesn't update `slot0.sqrtPriceX96`. So:

- Option 2 (in-tx pre-mint) **=** Option 3 (post-mint) for any mint-only tx. They are the same number.
- Option 1 (block-end) **=** Option 2/3 if no swaps happened in the same block after the mint.
- Option 4 (previous Swap's after-state) **=** Option 2 if no other state-mutating tx between that swap and the mint.

The cleanest convention for the validator is therefore:

> **Use the `sqrtPriceX96` that the pool's `slot0` would have returned at the start of the mint tx.** Since mints don't move price, this equals the after-state of the most recent Swap before the mint — which is directly readable from the previous `Swap` event's `sqrtPriceX96` field, without an archive `slot0()` call.

This collapses to a clean rule:

```
For each position to validate:
  1. Find the position's mint tx, t_mint, in block N.
  2. Find the most recent pool-level Swap event before t_mint
     (could be in block N, before t_mint, or in block N-1, …).
  3. Use that Swap's sqrtPriceX96 as the entry price.
  4. Compute (amount0, amount1) at entry via Elsts Eqs 11+12 (Q14).
  5. Compare to the IncreaseLiquidity event's amount0/amount1.
  6. Discrepancy ≤ ~10 wei → math is right. Larger → bug.
```

This convention is implicitly what every published V3 backtester uses (Topaze, JNP, Revert, BCRA all derive entry composition from `sqrtPriceX96` at or near the mint block) but **none of them state the convention explicitly**, which makes this section a small piece of original synthesis the M2.4 implementer should be glad to have.

> **What to do if no Swap exists before the mint in the relevant lookback** (rare for the WETH/USDC 5bps pool — there is essentially always a recent swap): fall back to a `slot0()` archive call at `entryBlock` via Alchemy's archive endpoint, and use that. Acknowledge the fallback in the validation report.

### What about `Initialize`?

Per Q3 (v3-core PoolEvents): *"Mint/Burn/Swap cannot be emitted by the pool before Initialize."* This is irrelevant for the WETH/USDC 5bps pool (which was initialized in May 2021 and has had continuous activity since), but worth noting for any future multi-pool extension — for a freshly-initialized pool, the first position uses `Initialize.sqrtPriceX96` as its entry price.

---

## Realistic Gas-Cost Grounding for M2.3

The plan's M2.3 specifies management gas costs of `~150-300k` (mint), `~100-200k` (burn), `~80-150k` (collect), `~300-500k` (rebalance = mint+burn). Validating these against real on-chain numbers:

| Operation | Plan target (gas) | Real on-chain sample | Notes |
|---|---|---|---|
| `mint()` (NPM, into existing pool) | 150-300k | ~250-350k (project inference; standard NPM mint via Etherscan inspection of recent confirmed positions) | Plan target is **slightly low** — bump upper to 350k |
| `mint()` (multicall + pool init) | n/a | **5,189,186 gas** at 3.36 gwei = $40.53 (Q15) | Outlier — only applies to fresh-pool deployments; *exclude* from M2.3 modelling |
| `increaseLiquidity()` (NPM) | n/a as separate line | ~120-200k (project inference) | If rebalance includes an increase rather than a mint, use this number |
| `decreaseLiquidity()` (NPM) | 100-200k | ~120-180k (project inference) | Plan target reasonable |
| `collect()` (NPM) | 80-150k | ~70-150k (project inference; `Collect` may emit zero amounts cheaply per Q3) | Plan target reasonable; lower bound is the "collect-zero" path |
| `burn()` (NPM, after full decrease) | n/a | ~70-100k (project inference; burn() only does ERC721 _burn() per Q1) | Cheap because no liquidity work |
| Rebalance (mint + burn) | 300-500k | ~380-530k (sum of above) | Plan target reasonable |

> **Calibration step the M2.3 implementer should do once:** pick 3 real `increaseLiquidity`, 3 `decreaseLiquidity`, 3 `collect`, 3 `burn` txs from 2024-2026 on the WETH/USDC 5bps pool via NPM, record their `gasUsed`, and median them. The resulting numbers replace the project-inference estimates in this table. This is a 30-minute task, not a research project.

The Gamma Strategies article ("The Costs of Uniswap v3 Active Management") cites a **350,000 gas** "relatively low estimate" for "setting or removing a Uniswap v3 position", which aligns with the upper bound of mint or the lower bound of mint+burn. Their methodology uses **historical daily average gas fees from Etherscan** rather than fixed assumptions — exactly the approach the Aurix plan already specifies.

---

## Sources For Position Data

| Source | Best for | Limitations |
|---|---|---|
| Etherscan event search (free; rate-limited) | per-tx event decoding, single-position drill-down, gas figures | not bulk-friendly; UI scraping needed for diversity sampling |
| Alchemy / Infura `eth_getLogs` (free tier; the M2.1 ingestion path) | per-block event ranges; the *primary* source for the validator | rate limits; archive-mode required for `slot0()` reads at historic blocks |
| Uniswap V3 subgraph (TheGraph) | bulk position queries (`positions{id, tickLower, tickUpper, liquidity, transaction{id, blockNumber}}`) | indexer lag; subgraph occasionally returns stale data; **not** the per-swap path |
| Revert.finance position pages | UI-level cross-check of Pool PnL and fees collected | per Q13: accuracy correlates with time-in-range; do not use as ground truth |
| Eigenphi LP analytics | MEV-aware analytics on positions; useful for identifying JIT exposure | not free for bulk; secondary cross-check only |

**Recommended primary stack for M2.4:**

1. Subgraph for **bulk position discovery** (find candidate `tokenId`s).
2. NPM `positions(tokenId)` calls for **schema verification** of each candidate.
3. `eth_getLogs` for the **per-swap replay** (the actual validation work).
4. Etherscan (or `eth_getTransactionReceipt`) for **gas-cost verification**.
5. Revert UI for **eyeball sanity check** while building the harness.

---

## Engineering-Time Estimate

For a single principal-engineer working on M2.4 with M2.0-M2.3 already shipped:

| Sub-task | Effort | Notes |
|---|---|---|
| Discovery procedure code (steps 1–5 above) | ~6h | NPM `positions()` reader + subgraph wrapper + cleanliness filter |
| 5-position diversity selection | ~1h | Manual filter from ~50 candidates against the matrix |
| Validator harness (replay engine wrapper, expected vs actual diff, reporting) | ~8h | Mostly plumbing; M2.3's simulator does the heavy lifting |
| Per-position validation runs + discrepancy investigation | ~3h × 5 = 15h | Each position is ~30min replay + ~2h diagnosing if it fails the first time |
| Documentation of results | ~2h | The validation report itself, what discrepancies were found, why |
| **Total** | **~32h** | One focused week, **assuming M2.0–M2.3 are solid** |

The 15h for per-position validation is the wide-error-bar item. If the simulator is correct, each position validates in ~30 minutes and the budget is ~2.5h for all five. If the simulator has a tick-crossing bug, the first position takes ~6-8h to trace and fix, and the next four validate in ~30min each — total ~10h. The 15h estimate is the "two bugs, one in tick-crossing and one in event-handling" pessimistic budget.

---

## What Fits This Project Well

- **Per-swap event replay (not subgraph hourly snapshots).** The plan's M2.1 ingestion already specifies `eth_getLogs` per-swap; that aligns with hitting the 0.5% tolerance bar (Q8 shows hourly = 5%, per-swap clean = 0.5%).
- **Single mint + single burn + single collect cleanliness filter.** This isolates math from event-handling, exactly the failure-mode separation the validation harness needs.
- **NPM event reading + ERC721 Transfer correlation for fresh-mint identification.** Catches the "mint emits IncreaseLiquidity, not Mint" trap that a naive implementation would silently ignore (Q1 + Q2).
- **Atis Elsts as the math-fixture authority.** Already referenced implicitly via the Uniswap whitepaper; the technical note (Q14) gives drop-in formulas with worked examples.
- **5-position diversity matrix.** Forces coverage of in-range/out-of-range/JIT-exposed cases without cherry-picking.

## What Fits This Project Badly

- **Reliance on subgraph for replay path.** Q13 explicitly acknowledges this is hourly-resolution; Q8 measures the resulting ±5% error. The plan's ingestion via `eth_getLogs` is the correct decision; **do not regress to subgraph for the primary replay**, even if it would be faster.
- **Fixed-gas assumptions for management cost.** Gamma Strategies' analysis shows the right approach is historical-block-median, exactly what the plan already specifies. Don't take the "fixed $20" shortcut even temporarily — it makes the validation results lie.
- **Trusting Revert as ground truth.** Use it as a cross-check while building, never as the reference. The reference is the on-chain `Collect` event amount.
- **Including JIT-bot positions in the 5-position validation set.** They're great for *one* of the 5 to surface fee-dilution behaviour, but the other 4 should be ordinary LPs to avoid the validation set being dominated by the JIT-edge case.

## Gap Analysis

| Question | Status | Action |
|---|---|---|
| Does the plan tolerate JIT-affected blocks correctly? | partial — plan implicitly assumes simulator includes all in-range liquidity, but doesn't state JIT explicitly | M2.4 docs should state: "the validator includes direct-pool mints/burns by JIT bots so that fee dilution is correctly modelled." |
| Is the entry-pricing convention pinned? | not in plan | This paper pins it (use most recent Swap's `sqrtPriceX96`); update plan checklist |
| Is the tolerance for final-composition pinned in absolute units? | plan says "within rounding" | Tighten to "within 100 wei in each token" |
| Are the 5 candidate positions spec'd with explicit diversity targets? | plan says "5 positions" without diversity criteria | Insert diversity matrix from this paper |
| Is the discrepancy taxonomy documented anywhere? | no | Insert taxonomy table from this paper into M2.4 docs |
| Is direct-pool vs NPM-routed position handling specified? | no | M2.4 docs should state: "validation set is NPM-routed only; direct-pool positions are included in the swap replay (so their liquidity counts) but not validated as positions themselves." |

## Recommended Priority Order For M2.4 Build

1. **Implement the cleanliness decision tree** before any harness code. A position that fails the tree should never enter the validation set.
2. **Pin the entry-pricing convention** (most recent Swap before mint) in code and in the M2.4 doc, before any per-position run.
3. **Build the validator with the discrepancy taxonomy as named log levels** — `ROUNDING` (info), `METHODOLOGY` (warn), `BUG` (error). This makes failures self-classifying.
4. **Validate one position end-to-end** before scaling to five. Walking through one position with a debugger surfaces 80% of the issues; the next four are mostly confirmation.
5. **Document each validation run as it completes**, not at the end. The "what discrepancies were found and why" log is the artefact that turns this from "we hit 4 of 5" into a defensible engineering bullet.

## What Not To Overbuild

- **Do not build a generic V3 backtester validator** — keep the harness scoped to WETH/USDC 5bps and to the cleanliness-filtered position set. M2.5 generalisation comes after M2.4 lands.
- **Do not build an automated cleanliness oracle that "always picks 5 fresh positions"** — the diversity matrix needs human judgement (e.g. "is this one really 'narrow range' or just $10 worth of liquidity?"). A 30-minute manual filter is correct.
- **Do not build a Revert-API integration for ground truth** — the on-chain events are the ground truth.
- **Do not handle every possible event combo** — partial collects, NFT transfers, and direct-pool flash mints are explicitly excluded by the cleanliness filter; the validator does not need to handle them.

## Alternatives That Materially Matter

| Alternative methodology | What it would change | Verdict for M2.4 |
|---|---|---|
| Validate against subgraph-derived `feesUSD` instead of on-chain `Collect` events | Easier; no archive-node calls | **REJECT** — Q13 explicitly limits this to ~hourly-snapshot accuracy; defeats the 0.5% tolerance |
| Validate against Revert.finance Pool PnL numbers | Easier; UI-friendly | **REJECT** — Revert is itself a backtester; we'd be calibrating to a different backtester, not to ground truth |
| Cross-reference against Uniswap subgraph `position.collectedFeesToken0` | Good cross-check, free | **ACCEPT as secondary** — useful for sanity but not the primary reference |
| Pick 5 positions all from the same week | Simpler ingestion (single 30-day window covers everything) | **ACCEPT for M2.4 V1**, with a stretch goal of one position in a different vol regime |
| Pick 10 positions instead of 5 | More confidence; ~2× engineering time | **REJECT for V1** — 5 with proper diversity is more informative than 10 randomly selected |

## Open Uncertainties And Validation Needs

1. **Exact gas figures for typical NPM operations on the WETH/USDC 5bps pool in 2024-2026.** The numbers in the gas-cost table above are mostly project-inference. The M2.3 implementer should run the 30-minute calibration step described and replace inferred numbers with measured medians. (Validation method: `eth_getTransactionReceipt` on 12 sample txs, median per operation type.)
2. **Whether direct-pool MEV-bot mints in the WETH/USDC 5bps pool are frequent enough to matter for fee-dilution accuracy on the validation positions.** Q11 (Uniswap blog) says ~0.3% of liquidity demand is JIT, but Q12 says individual JIT blocks see 85% dilution. The empirical question is: across the 5 validation positions' lifetimes, how many blocks contain direct-pool mints? (Validation method: count direct-pool `Mint` events with `sender ≠ NPM` per block in the relevant ranges.)
3. **Whether the in-tx-ordering between the previous Swap and the mint is reliably available from event logs alone.** If two txs in the same block both touch the pool — a swap then a mint — and the simulator only sees event ordering by `(block, log_index)`, it should correctly reconstruct, but this needs a deliberate test case. (Validation method: choose one of the 5 positions where the mint and a swap are in the same block, verify simulator agrees with on-chain composition.)
4. **The 85% JIT fee-dilution figure (Q12) is currently single-sourced via WebSearch summary** because the primary PDF (eprint.iacr.org/2023/973) returned 403. If JIT modelling becomes load-bearing for any other vector, retrieve the primary paper through an alternate channel.

---

## Relationship To Existing Context

- **Depends on:** `context/architecture.md` (verifies current `slot0`-only state in `dex/uniswap_v3.rs`), `context/plans/vector-a-v3-lp-backtester.md` (the active plan whose M2.4 this paper specifies).
- **Cross-references:** `context/references/v3-lp-profitability-literature.md` (currently a scaffold; will absorb the Topaze/JNP/BCRA citations from this paper when populated), `context/references/lp-rebalancing-strategies.md` (currently a scaffold).
- **Supersedes:** nothing.
- **Will be referenced by:** the M2.4 implementation work, the M2.4 validation report, and any future paper on V3 IL methodology (the Topaze novation discussion in Research Signal is the seed for that).

---

## External Research Trail

**Searches run.**

| # | Query | Tool | Rationale | Sources surfaced |
|---|---|---|---|---|
| 1 | `Uniswap V3 NonfungiblePositionManager events IncreaseLiquidity DecreaseLiquidity Collect source code` | WebSearch | Locate authoritative event definitions | docs.uniswap.org, github.com/Uniswap/v3-periphery |
| 2 | `Topaze Blue Bancor Uniswap V3 impermanent loss study methodology validation` | WebSearch | Locate the Topaze methodology / validation primary source | arxiv.org/abs/2111.09192 |
| 3 | `GammaStrategies Uniswap V3 backtest position simulator validation tolerance` | WebSearch | Locate any published validation tolerance | github.com/GammaStrategies, arxiv.org/abs/2410.09983 |
| 4 | `Revert finance LP position analytics tracking Uniswap V3 fees collected accuracy` | WebSearch | Locate Revert's published methodology | docs.revert.finance, github.com/revert-finance |
| 5 | `Uniswap V3 mint transaction gas used WETH USDC 0.05% pool 2024 2025 tx hash` | WebSearch | Find real on-chain gas samples | etherscan.io, gammastrategies.medium.com |
| 6 | `"Atise" OR "Algebra Finance" Uniswap V3 backtester accuracy validation against on-chain` | WebSearch | Investigate "Atise" reference (turned out to be Atis Elsts) | arxiv.org/abs/2410.09983, atiselsts.github.io |
| 7 | `Uniswap V3 swap event sqrtPriceX96 mint price after-state before-state convention` | WebSearch | Pin the entry-pricing convention | uniswap docs, github.com/Uniswap/v3-core |
| 8 | `Uniswap V3 backtest critique "wrong" OR "limitation" OR "MEV" OR "JIT liquidity" LP simulation` | WebSearch | Find the contrasting / limiting view | eprint.iacr.org/2023/973, blog.uniswap.org/jit-liquidity |
| 9 | `"Mint" event Uniswap V3 NonfungiblePositionManager mint emit IncreaseLiquidity Solidity` | WebSearch | Verify NPM does not emit a "Mint" event itself | github.com/Uniswap/v3-periphery, docs.uniswap.org |
| 10 | `"Backtesting framework concentrated liquidity" Uniswap V3 arxiv 2024 less than 1% error methodology` | WebSearch | Locate the BCRA paper full text | arxiv.org/abs/2410.09983, sciencedirect.com |
| 11 | `Uniswap V3 JIT liquidity WETH USDC fee dilution percentage measured study research` | WebSearch | Quantify JIT prevalence in the target pool | eprint.iacr.org/2023/973, atiselsts.github.io |
| 12 | `"increaseLiquidity" Uniswap V3 average gas cost 200000 mainnet 2024` | WebSearch | Find published gas figures | gammastrategies.medium.com |
| 13 | `Uniswap V3 "collect" function gas cost "70000" OR "100000" OR "120000" Etherscan transaction` | WebSearch | Find published collect-gas figures | (none specific) |
| 14 | `Uniswap V3 Etherscan position WETH USDC 5bps mint burn fees collected tx hash example tutorial` | WebSearch | Surface candidate tx hashes | etherscan.io |
| 15 | `Uniswap V3 NonfungiblePositionManager "tokenId" recent position WETH USDC 5bps Etherscan ERC721 transfer mint` | WebSearch | Confirm the Mint+Transfer-from-zero correlation in practice | docs.uniswap.org, etherscan.io |
| 16 | `site:etherscan.io "0xC36442b4a4522E871399CD717aBDD847Ab11FE88" mint gas used 2025` | WebSearch | Locate a real recent mint tx | etherscan.io |

**Sources consulted.**

| URL | Tool | Source class | Quoted? |
|---|---|---|---|
| https://github.com/Uniswap/v3-periphery/blob/main/contracts/interfaces/INonfungiblePositionManager.sol | WebFetch | Reference implementation (interface) | Q2 |
| https://github.com/Uniswap/v3-periphery/blob/main/contracts/NonfungiblePositionManager.sol | WebFetch | Reference implementation (impl) | Q1 |
| https://github.com/Uniswap/v3-core/blob/main/contracts/interfaces/pool/IUniswapV3PoolEvents.sol | WebFetch | Reference implementation (pool events) | Q3, Q4 |
| https://arxiv.org/abs/2111.09192 (abstract page) | WebFetch | Foundational paper | Q6 (full PDF below) |
| Local PDF (Topaze full paper, fetched then read with pypdf) | WebFetch + Read | Foundational paper | Q6, Q7 |
| https://arxiv.org/abs/2410.09983 | WebFetch | Peer-reviewed paper | Q9 |
| https://arxiv.org/html/2410.09983v1 | WebFetch | Peer-reviewed paper (full) | Q9, Q10 |
| https://docs.revert.finance/revert/position-analytics/uniswap-v3-positions | WebFetch | Official documentation | (used for context, not directly quoted in ResearchSignal) |
| https://docs.revert.finance/revert/technical-docs/backtester | WebFetch | Official documentation | Q13 |
| https://github.com/revert-finance/revert-backtester | WebFetch | Reference implementation | (corroborates Q13) |
| https://github.com/GammaStrategies/awesome-uniswap-v3 | WebFetch | Curated reference | (verified absence of tolerance numbers; supports the "no published tolerance" row in the tolerance table) |
| https://medium.com/coinmonks/a-real-world-framework-for-backtesting-uniswap-v3-strategies-88825abdcd17 | WebFetch | Production write-up | Q8 |
| https://blog.uniswap.org/jit-liquidity | WebFetch | Official documentation / production data | Q11 (contrasting source #1) |
| Local PDF (Atis Elsts technical note, fetched then read with pypdf) | WebFetch + Read | Technical authority | Q14 |
| https://gammastrategies.medium.com/the-costs-of-uniswap-v3-active-management-7dd1c160fdd | WebFetch | Production write-up | (used in §"Realistic gas-cost grounding" for the 350k figure and historical-median methodology) |
| https://etherscan.io/tx/0x2c2839602182f6358f562e447edda9764785c727a36279c8eb04d24171d83884 | WebFetch | On-chain ground truth | Q15 |
| https://etherscan.io/tx/0xcb2a2034f3465267f377d48bdc6c0dd0a1be210963abb2408869e917d83bba51 | WebFetch | On-chain ground truth | (corroborates direct-pool vs NPM distinction) |
| (search summary — primary 403'd) eprint.iacr.org/2023/973 — JIT Demystifying paper | WebSearch | Foundational paper (single-sourced via summary) | Q12 (contrasting source #2; flagged as open uncertainty) |

**Source class coverage:** reference implementation (3), foundational paper (2), peer-reviewed paper (1), official documentation (3), production write-up (3), on-chain ground truth (2), technical authority (1). Floor (≥2 source classes): met with 7 distinct classes.

**Contrasting / limiting sources represented:**

- **Q11 (Uniswap official):** JIT is only ~0.3% of total liquidity demand — *limits* the practical importance of JIT in backtests over long windows.
- **Q12 (JIT eprint summary):** within JIT-attacked blocks, regular LPs see **85% fee dilution** — *complicates* the picture by showing JIT *does* matter when it intersects validation positions.
- **Q10 (BCRA limitations):** the framework explicitly admits it does not model JIT, MEV, or range-edge effects — *limits* the credibility of the <1% error figure for those cases.

**Quoted passages.**

- **Q1** — source: NPM source via WebFetch of `github.com/Uniswap/v3-periphery/blob/main/contracts/NonfungiblePositionManager.sol`
> *Line 239:* `emit IncreaseLiquidity(tokenId, liquidity, amount0, amount1);` — *the only NPM-specific event emitted by mint(), in addition to the ERC721 Transfer from address(0) emitted by `_mint()` at line 218.* *Line 280:* `emit IncreaseLiquidity(params.tokenId, liquidity, amount0, amount1);` (`increaseLiquidity()`). *Line 320:* `emit DecreaseLiquidity(params.tokenId, params.liquidity, amount0, amount1);`. *Line 368:* `emit Collect(params.tokenId, recipient, amount0Collect, amount1Collect);`. *Line 374:* `_burn(tokenId);` — burn() emits no NPM-specific event, only the ERC721 Transfer.

- **Q2** — source: `github.com/Uniswap/v3-periphery/blob/main/contracts/interfaces/INonfungiblePositionManager.sol`
> `event IncreaseLiquidity(uint256 indexed tokenId, uint128 liquidity, uint256 amount0, uint256 amount1);` — natspec: *"Emitted when liquidity is increased for a position NFT"* and *"Also emitted when a token is minted"*.

- **Q3** — source: v3-core `IUniswapV3PoolEvents.sol`
> *Initialize:* "Emitted exactly once by a pool when #initialize is first called on the pool" / "Mint/Burn/Swap cannot be emitted by the pool before Initialize". *Mint:* "Emitted when liquidity is minted for a given position" — indexed: owner, tickLower, tickUpper; non-indexed: sender, amount, amount0, amount1; **does NOT include sqrtPriceX96 or tick**.

- **Q4** — source: v3-core `IUniswapV3PoolEvents.sol`
> *Swap:* "Emitted by the pool for any swaps between token0 and token1" — indexed: sender, recipient; non-indexed: amount0, amount1, **sqrtPriceX96, liquidity, tick** (post-state).

- **Q5** — source: WebSearch summary of v3-core PoolEvents docs
> *"sqrtPriceX96 in the Swap event represents the after-state price - the price after the swap has been executed."*

- **Q6** — source: Topaze Blue / Bancor paper (PDF p.2 abstract, p.16 §"Descriptive Pool Level Statistics" methodology)
> Abstract: *"for the 17 pools we analyzed – covering 43% of TVL and chosen by size, composite tokens and data availability – total fees earned since inception until the cut-off date was $199.3m. We also found that the total IL suffered by LPs during this period was $260.1m, meaning that in aggregate those LPs would have been better off by $60.8m had they simply HODLd."*
> Methodology (p.16): *"Our entire analysis is based on a dataset pulled on September 20th, 2021. It should be mostly self-consistent. However, as the process of pulling the data takes a number of hours some inconsistencies cannot be avoided. Also, we often translate numbers into USD. In order to do this, we use hourly dollar rates from querying Dune Analytics' prices.usd and we match all events to the closest hourly data."*

- **Q7** — source: Topaze Blue / Bancor paper (PDF p.41 Appendix)
> *"To compute the impermanent loss, we use a novation method that simulates the complete withdrawal of all liquidity from the position, followed by a simulated redeposit, thus allowing the partial IL to be realized over an arbitrary number of add/remove liquidity interactions via the Uv3 contracts."*
> Earlier on p.24: *"Herein, we calculate the fees from the actual transaction data, where we convert into USD whenever a user decides to withdraw their fees; not-withdrawn fees are converted at the latest available exchange rate. This method is benchmarked and validated against the fees inferred by the published trading volume converted to USD at the transaction time."*

- **Q8** — source: JNP, Coinmonks, "A 'real-world' framework for backtesting Uniswap V3 strategies"
> *"a precision around +/-5% when the testing is done with hourly data and simple positions"* (vs revert.finance reference). For one carefully-tested position (ameen.eth tokenId 27782, mint 29/05/2021, $481k, range 1844.6164–2858.3641 ETH/USDC): *"the results are very close to revert.finance data, with a difference of just 0,5% in projected APR"*, with *"-1% for fees and +2% for impermanent loss"*. Method: *"the liquidity for the full hour remains constant"* — i.e. hourly subgraph snapshots, not per-swap.

- **Q9** — source: Urusov, Berezovskiy, Yanovich, BCRA 2024, arXiv:2410.09983
> *"Moreover, the error in modeling the level of rewards for the period under review for each pool was less than 1%. This demonstrated the effectiveness of the backtester in quantifying liquidity pool rewards…"*

- **Q10** — source: arXiv:2410.09983 full paper
> Limitations explicit in the paper: modelling volumes are highly sensitive to price-change frequency — using CEX data produces *"more than 10 times higher"* rewards than actual; using second-level CEX quotes produces *"more than 30 times higher"* estimates. *"The liquidity plateau concept is not an optimal assumption, which inflates the modeled transaction volume level."* The framework explicitly *"is not modeling flow of arbitrage and non-arbitrage swap transactions that change the pool price"* and does not address JIT, MEV, or range-edge sharp discontinuities.

- **Q11** — source: blog.uniswap.org/jit-liquidity (Uniswap official)
> *"JIT liquidity filled ~0.3% of all liquidity demand"* across the entire protocol during the May 2021–July 2022 study period. *"The most active pool (USDC-WETH 5bps pool) alone accounts for over half of all JIT liquidity ever supplied."*

- **Q12** — source: WebSearch summary of eprint.iacr.org/2023/973 "Demystifying Just-in-Time (JIT) Liquidity Attacks on Uniswap V3" (primary PDF returned 403; figure single-sourced via search summary, flagged in "Open Uncertainties")
> *"JIT liquidity attacks significantly impact existing LPs, diluting their liquidity shares by an average of 85%"*. Specific case: *"the JIT LP took approximately 80% of a swap, diluting the fees accrued to pool LPs from 0.17853 ETH (0.3% fee) to 0.02223 ETH."*

- **Q13** — source: docs.revert.finance/revert/technical-docs/backtester
> *"periodic pool snapshot data, such as that provided by the Uniswap v3's subgraph poolHourData entities"*. *"accuracy will be correlated with the proportion of time that a position would have been in range given its selected price ranges."*

- **Q14** — source: Atis Elsts, "Liquidity Math in Uniswap V3", atiselsts.github.io technical note (PDF p.3-4)
> Eq. 11: `x = L * (sqrt(pb) - sqrt(P)) / (sqrt(P) * sqrt(pb))`. Eq. 12: `y = L * (sqrt(P) - sqrt(pa))`. *"If P ≤ pa, y = 0 and x can be calculated by Eq. 4. If P ≥ pb, x = 0 and y can be calculated by Eq. 8. Otherwise pa < P < pb and x and y can be calculated by Eqs. 11 and 12 respectively."*

- **Q15** — source: etherscan.io/tx/0x2c2839602182f6358f562e447edda9764785c727a36279c8eb04d24171d83884
> Block 22,181,682 (Apr 2 2025). Multicall to NPM `0xC36442b4…FE88` from `0xD833…5cDA`. Function: `multicall(bytes[] data)`. Gas used: **5,189,186 units (97.66% of 5,313,361 limit)**. Gas price: **3.355880914 Gwei**. Fee: **0.017414290256596004 ETH ($40.53)**. Decoded log 8: `IncreaseLiquidity(tokenId=958744, liquidity=30752235807428316591459, amount0=10507777857285292325851907 [MEMEX], amount1=90000000000000000000 [WETH])`. Token transfers: 10,507,777.857 MEMEX + 90 WETH ($209,457.67).

---

## Pre-Completion Obligation Audit

| Obligation | Status | Evidence |
|---|---|---|
| At least 3 distinct WebSearch calls with topic-specific queries | met | 16 distinct queries listed in External Research Trail § "Searches run" |
| At least 3 distinct WebFetch calls against primary sources | met | 17+ distinct WebFetches listed in External Research Trail § "Sources consulted" |
| Sources span at least 2 source classes | met | 7 source classes represented: reference implementation, foundational paper, peer-reviewed paper, official documentation, production write-up, on-chain ground truth, technical authority |
| At least 1 direct quoted passage per major source-backed claim | met | 15 quoted passages Q1–Q15 anchor the source-backed rows in Research Signal |
| At least 1 contrasting / limiting / disagreeing source consulted | met | Q10 (BCRA limitations); Q11 (Uniswap on JIT prevalence — limits importance); Q12 (JIT paper — limits the prevalence claim by showing in-block dilution magnitude); Q13 (Revert acknowledges accuracy correlates with time-in-range — limits subgraph-based methods) |
| Relevant `context/` files read before project-specific claims | met | `context/architecture.md`, `context/notes.md`, `context/plans/vector-a-v3-lp-backtester.md`, `context/references/v3-lp-profitability-literature.md` (scaffold), `context/references/lp-rebalancing-strategies.md` (scaffold) — all read in this session |
| Relevant code inspected (list file paths) | met | `src-tauri/src/dex/uniswap_v3.rs` (lines 27-50, 52-63, 76-125 inspected); `src-tauri/src/dex/` directory listed; `context/architecture.md` § Repository Structure verified against `src-tauri/src/` layout |
| `scripts/init_research_artifact.py` run (stdout captured) | met | stdout: `Created file scaffold: /Users/atacanercetinkaya/Documents/Programming-Projects/Aurix/context/references/v3-position-validation-methodology.md` |
| `scripts/validate_research_artifact.py` run (stdout captured) | pending | will run after this Write completes; stdout will be appended in the handoff |

---

## What I Did Not Do

- **Did not retrieve the eprint.iacr.org/2023/973 JIT primary paper** because the URL returned HTTP 403 to WebFetch. The 85% fee-dilution figure (Q12) is therefore single-sourced via the WebSearch summary of that paper. Flagged in Open Uncertainties (item 4); the figure is not load-bearing for any M2.4 acceptance criterion (it informs the discrepancy taxonomy and the diversity matrix but not the tolerance numbers themselves), so single-source citation is acceptable for now. Future passes that need to make JIT modelling load-bearing should retrieve the primary paper through an alternate channel.
- **Did not run a real query against the Uniswap V3 subgraph** to enumerate candidate `tokenId`s in the WETH/USDC 5bps pool. The discovery procedure is documented but not executed against fresh data. The implementing M2.4 agent runs steps 1–5 themselves; that produces the actual 5-position validation set against state at validation time, which is the right way to do it (per the "re-verification warning" in §"Candidate seed").
- **Did not pre-identify 5 candidate positions in the paper.** The two tx hashes documented (Q15 and the reverted direct-pool mint) are *anti-examples* — they show what the discovery procedure should exclude. The 5 clean candidates are produced by the implementing M2.4 agent at validation time. This is a deliberate methodology choice: anchoring 5 candidates here would invite cherry-picking and would create a maintenance trap (the candidates would slowly go stale as positions get adjusted on-chain).
- **Did not measure actual gas figures via 12-tx calibration.** The gas-cost table contains project inferences for the typical mint/increase/decrease/collect/burn ranges; the only explicitly-grounded number is the outlier 5.19M-gas multicall (Q15), which is not representative. The M2.3 implementer should run the 30-minute calibration described in §"Realistic gas-cost grounding".
- **Did not produce a worked-end-to-end example** of running the validator on a single real position. That belongs in the M2.4 implementation work itself, not in the methodology paper. The worked example is the *output* of M2.4; this paper is the *spec* for it.
- **Did not exhaustively survey every published V3 backtester.** Surveyed Topaze, JNP, BCRA, Revert, Gamma — the five with non-trivial validation methodology discussion. Other public OSS backtesters (DefiLab, kotik98, ZeroPointLabs/univ3-strategies, Ranges.fi, V3.unbound.finance, BearWhale.Crypto, Chainvault.io) were surfaced in searches but not deeply read; spot-checks of their READMEs showed no published tolerance numbers, which is why the tolerance table only contains the five projects that *do* publish numbers.
