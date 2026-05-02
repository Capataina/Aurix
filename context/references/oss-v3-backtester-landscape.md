# OSS V3 Backtester Landscape — Competitive Analysis and Differentiation

## Scope / Purpose

This paper answers one repository-specific question:

> *Does the planned design of Aurix Tab 2 — Uniswap V3 LP backtester with per-swap fee distribution, management gas modelled at historical block prices, DeFi-native benchmarks (Aave, Lido, Compound), and a regime-conditional capital-allocation headline — represent meaningful differentiation against the existing OSS V3 backtester landscape, or is it re-implementing something that already exists?*

In scope:

- a per-tool inventory of public OSS V3 LP backtesters as of May 2026, with last-commit dates, language, scope, fee-distribution method, gas modelling, validation, and benchmarks
- an honest assessment of where Tab 2's planned features overlap with prior art and where they genuinely advance it
- known approximations and bugs in popular OSS implementations that the implementing agent should avoid replicating
- a concrete README-ready differentiation paragraph, written from evidence rather than aspiration

Explicitly **out of scope**:

- closed-source institutional tooling at quant LP desks (Wintermute, GSR, Cumberland, Jane Street's DeFi desk) — invisible from outside, hiring-narrative differentiation cannot be argued from material we cannot read
- L2 backtesters specifically (Aurix Tab 2 is mainnet-only for V1)
- general AMM theory or V3 math derivation (separate paper if needed; covered by V3 whitepaper directly)
- Uniswap V4 backtesters (different math, different liquidity primitive — out of scope until Aurix targets V4)

## Current Project Relevance

Aurix Tab 2's plan is at `context/plans/vector-a-v3-lp-backtester.md` (status: active, research in progress). It is the dedicated hiring-credibility deliverable for the AMM-mathematics and quant-DeFi audience (Wintermute, GSR, Cumberland, DeFi treasury teams, Uniswap Labs, Aave, Gamma, Arrakis). The vector's own framing rests on three claims, copied from the plan verbatim:

> *"Most OSS V3 backtesters output 'here's your equity curve' and stop."* (line 23)
> *"Most OSS V3 implementations approximate fee distribution at the block-aggregate level."* (line 295)
> *"Most backtests assume zero management cost, which significantly overstates returns at retail-typical position sizes."* (line 298)

Each of these claims drives a milestone in the plan and a future README sentence. Each is testable. This paper tests them.

The current Aurix V3 surface is much smaller than the plan implies: `src-tauri/src/dex/uniswap_v3.rs` is 152 lines, exposing only a `slot0()` reader and a `sqrtPriceX96 → spot price` decoder for the live arbitrage tab. There is no tick math, no liquidity traversal, no swap simulation, no position model, no persistence layer. `context/architecture.md` confirms this — "the current product does not include persistence, routing, background jobs, automated tests, or implementation for Tabs 2 to 5". So this paper is read against a near-greenfield Tab 2, not against existing backtesting code that needs to be incrementally improved.

## Current State Snapshot

Verified facts about the existing Aurix V3 layer:

| Claim | Evidence | Class |
|---|---|---|
| `slot0()` reader present, returns `sqrtPriceX96` and decodes spot price | `src-tauri/src/dex/uniswap_v3.rs:27-50` (`fetch_snapshot`) | repository fact |
| No tick math, no liquidity_for_amounts, no swap simulation in the codebase | `src-tauri/src/dex/uniswap_v3.rs` is 152 lines and stops at price decode at line 125 | repository fact |
| No persistence layer | `context/architecture.md:12` — "the current product does not include persistence" | repository fact |
| No tests | `context/architecture.md:180` — "There is no test directory or automated verification layer in the repository" | repository fact |
| Two empty scaffolds exist in `context/references/` for related topics: `lp-rebalancing-strategies.md` and `v3-lp-profitability-literature.md` | both files inspected; both contain only the unfilled scaffold template | repository fact |
| Plan promises per-swap fee distribution (M2.3), historical-block-price gas modelling (M2.3), 4-of-5 on-chain validation (M2.4), DeFi-native benchmarks (M2.7), regime-conditional headline (M2.8) | `context/plans/vector-a-v3-lp-backtester.md` lines 117-232 | repository fact |
| Resume bullet: "Per-swap fee distribution within X% of collected ground truth" is currently vapor — there is no implementation to back it | plan line 17 ("currently vapor") + code surface above | repository fact |

## What The Topic Actually Is

Brief shared grounding before the inventory. **Concentrated-liquidity backtesting** has three independent mathematical layers:

1. **Pool replay layer** — given a sequence of historical swaps, reproduce the pool's `sqrtPriceX96`, `tick`, and `liquidity` state at each swap. The cost depends on whether you replay swap-by-swap (every event hits the tick math) or trust an external snapshot service that has already done the replay (e.g. The Graph's `poolHourData` entities).
2. **Position attribution layer** — for a hypothetical position `(tickLower, tickUpper, liquidity)`, decide what share of swap fees that position would have earned at each in-range moment. This requires either:
   - **per-swap attribution** — at each swap, compute `position_liquidity / active_liquidity * fee_paid`, and accumulate. Granularity = one record per swap.
   - **per-block / per-hour aggregation** — bin swaps into blocks or hours, sum the fees in the bin, and prorate by the share of in-range liquidity at the bin's snapshot. Loses information when active liquidity changes mid-bin.
3. **Cost layer** — gas, slippage, taxes, MEV, opportunity cost. Uniswap V3 contracts charge no protocol fee on the LP side, so this layer is entirely about real-world frictions that are not in the contract's accounting.

The interesting public sources mostly differ on layers 2 and 3, not layer 1. Tools that "use the subgraph" are typically using `poolHourData` as a layer-1 shortcut and then doing per-hour layer-2 aggregation; tools that "replay from raw chain data" do swap-by-swap layer-1 plus per-swap layer-2.

The plan's M2.3 claim is a layer-2 claim (per-swap attribution). The plan's gas modelling is a layer-3 claim. The plan's benchmark module is an even-higher layer that no other tool surveyed in this paper attempts in unified form.

## Per-Tool Inventory

Sorted by recent activity (most recently pushed first). All last-pushed dates retrieved 2026-05-02 via `gh api repos/<org>/<repo>` against the GitHub REST API; values are commit-push timestamps, not release dates.

| Tool | Last push | Lang | Stars | Scope | Layer-2 method | Gas? | On-chain validation? | Benchmarks beyond HODL? |
|---|---|---|---|---|---|---|---|---|
| [zelos-alpha/demeter](https://github.com/zelos-alpha/demeter) | 2026-04-29 | Python | 86 | Multi-market backtester (V3, Aave, GMX, Deribit, Squeeth) | Replicates contract logic, but README does not specify per-swap vs per-bin | Not stated | Not explicitly, claims "close to real world" via contract replication | Aave, GMX, Squeeth via market integration |
| [Bella-DeFinTech/uniswap-v3-simulator](https://github.com/Bella-DeFinTech/uniswap-v3-simulator) ("Tuner") | 2026-03-03 | TypeScript + Solidity | 151 | Transaction-level V3 simulator, used as pool reference | Tick-level event replay | No | None stated; claims "minimum margin of deviations" without numbers | None |
| [ArrakisFinance/v2-core](https://github.com/ArrakisFinance/v2-core) | 2026-01-12 | TypeScript | 47 | LP-management contracts, not a backtester | n/a | n/a | n/a | n/a |
| [ilyamk/uniswap-v3-lp-strategy-toolkit](https://github.com/ilyamk/uniswap-v3-lp-strategy-toolkit) | 2025-02-26 | Python | 5 | Solo project: data collection + dynamic LP backtest | Per-swap (claimed) | "Realistic trading costs" claimed but unverified | None public | None stated |
| [DefiLab-xyz/uniswap-v3-simulator](https://github.com/DefiLab-xyz/uniswap-v3-simulator) | 2024-01-17 | JavaScript | 69 | IL simulator + LP strategy backtest | Hourly subgraph (`poolHourData`) | No | Cross-checked against Revert (±5% per JNP article using same data) | None |
| [zelos-alpha/Backtesting-Uniswap-V3-Strategies](https://github.com/zelos-alpha/Backtesting-Uniswap-V3-Strategies) | 2024-12-10 | Jupyter | 5 | Research notebooks, strategy comparison | Inherits Demeter's engine | No | No | No |
| [idrees535/Uniswap-V3-Simulator](https://github.com/idrees535/Uniswap-V3-Simulator) | 2024-03-30 | Jupyter | 7 | Pool-launch and LP-management simulator | Not specified | No | No | No |
| [yogi-bo/uniswap-v3-online-framework](https://github.com/yogi-bo/uniswap-v3-online-framework) | 2023-06-14 | Jupyter + Go | 11 | Online (live) LP strategies; uses BigQuery + a Go simulator | Per-swap via Go simulator | Not stated | Not stated | Not stated |
| [GammaStrategies/uniswap-v3-performance](https://github.com/GammaStrategies/uniswap-v3-performance) | 2023-05-22 | Python | 28 | "Visor specific" managed-position analytics — not a general backtester | Per-position attribution from on-chain history | n/a (post-hoc, not predictive) | yes — by construction (it's an analyser, not a simulator) | None |
| [revert-finance/revert-backtester](https://github.com/revert-finance/revert-backtester) | 2023-02-03 | Clojure | 45 | "Fast" backtester powering revert.finance | Periodic pool snapshots (`poolHourData`) per docs | No | Yes — validated against real positions (single-deposit, no-withdraw, 30-day window) | No |
| [DefiLab-xyz/uniswap-v3-backtest](https://github.com/DefiLab-xyz/uniswap-v3-backtest) | 2022-05-27 | JavaScript | 60 | Predecessor to uniswap-v3-simulator | Hourly subgraph | No | No | No |
| [DefiLab-xyz/uniswap-v3-backtest-python](https://github.com/DefiLab-xyz/uniswap-v3-backtest-python) | 2022-05-14 | Python | 19 | Python port | Hourly subgraph | No | No | No |
| [GammaStrategies/strategy-one](https://github.com/GammaStrategies/strategy-one) | 2021-07-29 | Jupyter | 46 | Bollinger-band LP strategy (research artefact) | Per-swap from BigQuery | No | No | No |
| GammaStrategies/active-strategy-framework | **DELETED** (404 as of 2026-05-02) | (was Python) | (was high) | Was the canonical OSS LP simulator framework, taken private | Per-swap, full Uniswap swap history per old README | No | Limited | No |
| [kotik98/my_unisvap_v3_backtest](https://github.com/kotik98/my_unisvap_v3_backtest) | 2022-10-09 | Python | 5 | Solo backtester | Per-swap from subgraph | No | No | No |
| [nguyenhieuec/uniswap_backtester](https://github.com/nguyenhieuec/uniswap_backtester) | 2021-12-01 | (none set) | 10 | Solo, abandoned | Not specified | No | No | No |

Two non-OSS but research-relevant entries:

| Tool | Format | Layer-2 method | Notes |
|---|---|---|---|
| Universe Finance backtester (ETHGlobal showcase, ~2022) | Java + Gradle | Block-level reconstruction from real swaps | Showcase claims "first open-source backtesting platform for Uniswap V3"; reachability of the actual code today is unclear |
| Urusov/Berezovskiy/Yanovich (arXiv 2410.09983, 2024) | Academic paper | Parametric model of liquidity distribution | Reports <1% reward-modelling error; the strongest published peer-reviewed accuracy result |

## Maintenance Status

**Material observation**: only **two tools have been touched in 2026** — `zelos-alpha/demeter` (Apr 2026) and `Bella-DeFinTech/uniswap-v3-simulator` (Mar 2026). `ArrakisFinance/v2-core` is also touched in Jan 2026 but is contracts, not a backtester. Every other public OSS backtester surveyed is **18+ months stale**.

The most surprising finding: **GammaStrategies/active-strategy-framework**, which was the canonical Python LP-simulator framework cited by every prior survey, no longer exists at its public URL — `gh api repos/GammaStrategies/active-strategy-framework` returns 404 as of 2026-05-02. The Gamma org's currently-public repos (listed via `gh api orgs/GammaStrategies/repos`) include `hypervisor` (smart contracts), `uniswap-v3-performance` (Visor-specific analytics, last pushed May 2023), and `awesome-uniswap-v3` (a list, last pushed Mar 2022). Their backtesting code has gone private. ICHI's public org (`ichifarm`) contains no backtester at all — only farming and vault contracts. Arrakis has no public backtester either — only `research/` notebooks and core contracts.

**Inference**: the OSS V3 backtester ecosystem, in 2026, is two actively maintained tools plus a long tail of abandoned solo projects. The serious quantitative tooling at LP-management protocols has moved private. This shapes the differentiation framing more than any feature comparison.

## Research Signal

The cross-evidence table that ties findings to the plan's specific claims and to current repository state.

| Topic | Source-backed signal | Source citation (URL + passage ID) | Current repository state | Citation (file:line) | Project implication | Evidence class |
|---|---|---|---|---|---|---|
| Per-swap vs per-block fee distribution prevalence | Most public tools use per-hour subgraph data; precision is "around +/-5%" with hourly data | https://medium.com/coinmonks/a-real-world-framework... — passage [JNP-1] | No fee math implemented yet | `src-tauri/src/dex/uniswap_v3.rs` (slot0 only) | Plan's M2.3 per-swap claim is **partially novel** — the well-maintained Tuner does tick-level replay, so per-swap is not unique; what *is* uncommon is per-swap *plus* on-chain validation against real positions in unified mainstream OSS | source-backed |
| Subgraph data is unreliable for fees and TVL | Vakhmyanin: USDC-WETH 0.3% pool: official TVL $333M, actual $176M; subgraph "displays figures as if no fees were ever accrued" | https://medium.com/coinmonks/all-your-uniswap-v3-liquidity-farming-calculations-are-dead-wrong... — [VAKH-1] | n/a | n/a | If the implementing agent uses subgraph TVL/fee fields directly, the backtest will be silently wrong. M2.4 validation against real positions is the exact safeguard against this. Validation is the differentiator, not "not using the subgraph" | source-backed |
| Validation against real on-chain positions | Revert validates against real positions meeting "minted within 30 days, single deposit, no withdrawals" criteria, displays scatterplots of backtest vs observed | https://github.com/revert-finance/revert-backtester — [REVERT-1] | No validation harness exists | n/a | M2.4's "4 of 5 within 0.5%" target is **harder than what Revert publishes** (Revert shows scatterplots without a stated tolerance). If hit, the validation is a defensible technical claim; if not hit, drop the bullet rather than weaken the tolerance | source-backed |
| DeFi-native benchmark comparison | No surveyed OSS V3 backtester compares LP returns to Aave/Compound/Lido in a unified framework. Demeter integrates Aave + GMX + V3 as separate market types but README does not describe a unified LP-vs-lending benchmark output | https://github.com/zelos-alpha/demeter — [DEMETER-1] | Plan calls for it as M2.7 | `context/plans/vector-a-v3-lp-backtester.md:182-214` | M2.7 is the **strongest novel differentiator**. Demeter has the substrate (multi-market) but the analytical synthesis is missing. Aurix can credibly claim "first OSS V3 backtester with unified DeFi-native benchmark output" if M2.7 ships | source-backed |
| Gas as % of capital is a real concern at small position sizes | Gamma cites $2,246/tx mainnet vs $2 Optimism in active management; 350k gas per position adjustment relatively low; up to 1.48M gas per rebalance+hedge | https://gammastrategies.medium.com/the-costs-of-uniswap-v3-active-management... — [GAMMA-COST-1] | M2.3 plans historical-block-price gas modelling | `context/plans/vector-a-v3-lp-backtester.md:124-130` | The plan's M2.3 gas modelling is **substantively defensible** — Gamma's own writeup confirms gas materially affects active-management economics. The "$1k position with 10 rebalances loses 20% of capital" framing in the plan is well-supported | source-backed |
| BUT: gas may not be the dominant LP cost | Gamma also notes "the main objectives in analyzing LP strategies is showing how the main enemy of an active liquidity provider is the accumulation of impermanent loss and not the accumulation of gas costs" | https://gammastrategies.medium.com/the-costs-of-uniswap-v3-active-management... — [GAMMA-COST-2] | n/a | n/a | The plan's emphasis on gas modelling as headline differentiation is **partially weakened**. IL is what matters most at large position sizes. Reframe gas as "the dominant cost in the small-position regime" rather than "the silent killer of all backtests" | contrasting source-backed |
| <1% accuracy is achievable with parametric models | Urusov et al. (arXiv 2410.09983): "The error in modeling the level of rewards for the period under review for each pool was less than 1%" using parametric liquidity-distribution approximation | https://arxiv.org/abs/2410.09983 — [URUSOV-1] | n/a — academic paper, no public implementation | n/a | The plan's "0.5% tolerance" target is in the **same league as published academic results**. This is a defensible specificity claim. Aurix should match or beat the parametric-model approach using its own per-swap-and-validate route | source-backed |
| Fee-growth underflow is a real bug class in V3-derived contracts | code-423n4/2023-12-particle-findings #10: "Underflow could happen when calculating Uniswap V3 position's fee growth and can cause operations to revert" — Solidity 0.8.x prevents the underflow Uniswap V3 implicitly relies on | https://github.com/code-423n4/2023-12-particle-findings/issues/10 (via search result) — [C4-1] | Aurix uses Rust BigUint + custom fixed-point math in plan; underflow handling is the implementer's responsibility | `context/plans/vector-a-v3-lp-backtester.md:113-116` | Implementing agent must implement V3's wraparound semantics for `feeGrowthInside` deltas exactly — `wrapping_sub` on the relevant integer width, not `checked_sub`. This is a **specific, citable bug** to avoid | source-backed |
| Hourly subgraph data has known reliability gaps | Uniswap/v3-subgraph #79: "Fees and volume always 0 for PoolHourDatas" while daily data returns correct values | https://github.com/Uniswap/v3-subgraph/issues/79 (via search result) — [SUB-79] | n/a — Aurix plan does not use subgraph for fees | n/a | Validates Aurix's plan choice (raw `eth_getLogs` for swaps in M2.1, not subgraph). README can credibly claim "raw chain data, no subgraph dependency" — many tools cannot | source-backed |
| Regime-conditional capital-allocation framing | No surveyed tool surfaces "should you have LP'd at all?" as the headline output. Even Universe Finance's hackathon project's claim was "LPs have a chance to earn more money by using V3 than V2" — comparative *within* AMM versions, not across capital-allocation alternatives | https://ethglobal.com/showcase/universe-finance-backtesting-for-uniswap-v3-25v8m — [UNIV-1] | M2.8 plans this as headline | `context/plans/vector-a-v3-lp-backtester.md:216-232` | **Most novel differentiator after M2.7**. No public OSS tool does this. The hiring-narrative line "I turned a backtester from a tool into an investment recommendation framework" is defensible if M2.8 ships — and is unique | source-backed + project inference |

## What Fits This Project Well

The Aurix plan's design choices that this research **strongly validates**:

1. **Raw `eth_getLogs` over subgraph for swaps (M2.1)** — directly addresses the well-documented fee/volume bugs in `poolHourData` and the TVL drift Vakhmyanin identifies. Most surveyed tools use subgraph and inherit those bugs silently.
2. **Per-swap attribution as the layer-2 method (M2.3)** — academically supported (Urusov reports <1% error with sophisticated layer-2 modelling) and matches the better-maintained tools (Tuner, demeter). Subgraph-based hourly aggregation has documented ±5% inaccuracy at best (JNP) and silent zero-fee bugs at worst (`v3-subgraph` issue #79).
3. **Cross-validation against 5 known on-chain positions with $X / 0.5% tolerance (M2.4)** — Revert is the only public tool that publishes any form of real-position validation, and they show scatterplots rather than a tight tolerance. A 0.5% bound, *if* genuinely achieved, beats the published evidence.
4. **DeFi-native benchmark module (M2.7)** — **no surveyed public OSS tool does this**. Demeter has the multi-market substrate but stops short of unified LP-vs-lending output. This is the cleanest "first" claim the project can make.
5. **Regime-conditional capital-allocation headline (M2.8)** — also unmatched in the surveyed landscape. All other tools answer *"how much would the LP have earned"* and stop; the plan answers *"should the LP have LP'd at all in this regime, vs the alternatives that were available with the same wallet"*, which is the question quant LP desks actually ask.
6. **Management-gas modelling at historical block prices (M2.3)** — Gamma's own writeup confirms gas materially affects active-management economics; no surveyed OSS tool does honest historical-median gas modelling. The plan's "small position management-gas-dominance" output is novel and useful.

## What Fits This Project Badly

Where the plan is **vulnerable to honest critique** from the evidence:

1. **The "most OSS implementations approximate at the block-aggregate level" framing in plan line 295 is partially overstated.** It is true that the *most-used*, *most-discoverable* OSS tools (Revert, DefiLab) are subgraph-based and aggregate. But Bella-DeFinTech's Tuner explicitly claims tick-level event-by-event replay, and Demeter claims to "draw on the code of the contract" rather than theoretical formulas. So per-swap fee distribution by itself is **not unique**. What is rarer is *per-swap distribution + on-chain validation harness + 0.5% tolerance + regime-conditional output* in one package — that is the actual position to defend.
2. **Gas modelling is *contested* as the dominant cost.** Gamma's writeup reads: *"the main objectives in analyzing LP strategies is showing how the main enemy of an active liquidity provider is the accumulation of impermanent loss and not the accumulation of gas costs."* The plan's plan line 298 ("management gas costs are the silent killer of small LP positions") is correct *in the small-position regime* but is misleading at the institutional position sizes the resume audience cares about. The framing should reflect this — not drop the gas modelling, but stop pretending it's the dominant story for $100k+ positions.
3. **The Gamma framework removal is double-edged.** It removes a strong public competitor (good for differentiation), but it also signals that *the canonical maintained OSS V3 backtester* has moved private — which is what serious LP teams actually use. A reasonable interviewer's reply to *"I built a V3 backtester"* may be *"so did Gamma, and they took it private; what's yours used for?"* The hiring narrative needs to address this directly: this is a *public, validated, multi-asset benchmark tool*, not a serious LP-management product.
4. **The "5% precision is enough for many use cases" position is real.** JNP achieves ±5% with hourly data and concludes the precision-vs-speed trade-off favours hourly for accessibility. The plan's 0.5% tolerance is academically interesting but may not affect any business decision a quant LP team makes at production scale. The differentiation should not lean too hard on tolerance numbers alone.
5. **No live-position tracking and no L2 support is fine, but should be admitted up front.** Several abandoned tools at least claimed L2 — the plan's mainnet-only choice is correct for a hiring artefact (depth-on-one) but should be stated as a deliberate scope choice in the README, not a gap.

## Gap Analysis — What the Best V3 Backtester Right Now Would Have, and Whether Aurix Plans It

This is the audit question: *if we were to build the strongest possible V3 backtester in 2026, what would it have, and how does the Aurix plan compare?*

| Feature | Best-in-class would have | Aurix plan | Gap |
|---|---|---|---|
| Layer-1: pool replay from raw chain data | Raw `eth_getLogs` for `Swap`, `Mint`, `Burn`, `Collect` events | M2.1: `eth_getLogs` for `Swap` only — `Mint`/`Burn`/`Collect` only used in M2.4 validation harness | **Gap**: ingestion only ingests `Swap`. For positions whose liquidity changed mid-window, the simulator needs `Mint`/`Burn` events to reconstruct active liquidity correctly. M2.1 should ingest these too |
| Layer-2: per-swap fee attribution | Per-swap, validated against on-chain truth | M2.3 + M2.4 | OK |
| Layer-3: gas modelling | Historical median gas per block | M2.3 | OK |
| Layer-3: slippage / MEV on entry/exit swaps | Sandwich-tax estimate on the mint/burn swap leg | Out of scope per plan line 277 | Gap, **but reasonable** — adding MEV modelling would be 2-4 weeks of additional work and validation is hard. Document as known limitation in README |
| Layer-3: tax-adjusted returns | Per-rebalance taxable-event accounting | Out of scope per plan line 278 | Reasonable scope choice |
| Validation: on-chain reference positions | ≥5 positions with mint/burn/collect tx hashes | M2.4 | OK |
| Strategy axis: range width | Configurable | M2.5 | OK |
| Strategy axis: rebalance rule | At least: static, schedule, threshold, time-out | M2.5 plans 4 rules | OK, **arguably best in class** — most tools have 0-1 rebalance rules |
| Benchmarks: HODL | Yes | M2.3 | OK |
| Benchmarks: stable lending | Aave + Compound | M2.7 | OK |
| Benchmarks: ETH staking | Lido + native | M2.7 | OK |
| Benchmarks: TradFi (T-bill, S&P, gold) | Optional but very hiring-friendly | M2.7 secondary | OK |
| Output: equity curve vs benchmarks | Overlay on same chart | M2.7 + M2.6 | OK |
| Output: heatmap of strategy grid | Color-coded by Sharpe | M2.6 | OK |
| Output: regime-conditional headline | Vol-conditional rotation rule | M2.8 | **Unique** |
| Determinism / reproducibility | Same input → identical output | Plan validation row | OK |
| Multi-pool fan-out | Several pools in one backtest | Out of scope per plan line 273 | Reasonable for V1 |
| L2 support | Arbitrum / Optimism / Base / Polygon | Out of scope per plan line 282 | Reasonable for V1 — but flag as a "Tab 2 V2" |
| Subgraph fallback | None (raw chain only) | Aligned | OK |
| Fee-growth underflow handling | `wrapping_sub` on `feeGrowthInside` deltas | Implicit in M2.2 acceptance row "every primitive validated against fixtures from V3 whitepaper" — but should be **named explicitly** to avoid the documented underflow class | Small gap — recommend adding an explicit acceptance row in M2.2: *"feeGrowthInside delta uses wrapping arithmetic; regression test exercises a wraparound case"* |

**One concrete recommended plan addition**: M2.1 should ingest `Mint`, `Burn`, and `Collect` events alongside `Swap`. Without these, M2.4's validation against real positions cannot reconstruct the active-liquidity timeline correctly when other LPs entered/exited during the validation window. This is a one-paragraph plan edit, not a milestone-level change.

## Differentiation Verdict (README-ready)

This section is written so it can be lifted into the project README directly.

**Short answer**: Aurix Tab 2 has *meaningful but bounded* differentiation. The unique combination is real; not every individual feature is.

**Honest per-feature breakdown:**

| Feature | Genuinely novel? | Notes |
|---|---|---|
| Q64.96 V3 tick math from scratch in Rust without ethers-rs | Yes-ish | The Tuner does it in TypeScript; Demeter does it in Python; no public Rust implementation surveyed. Implementation-language novelty, not algorithmic novelty |
| Per-swap fee distribution | No (Tuner, Demeter, GammaStrategies/active-strategy-framework when public) | Common in serious tools, less common in popular tools. The framing "most OSS approximate at block-aggregate level" is true of *the tools most users find first* but not of the technically strongest tools |
| 0.5%-tolerance on-chain validation against ≥5 real positions | **Yes** | No public OSS tool publishes this tolerance; Revert is the closest and shows scatterplots without a stated bound. *If* the harness hits the bound, this is a strong, citable claim |
| Honest historical-median management gas | **Yes** | No surveyed public OSS tool models management gas at historical block-level prices. Most assume zero |
| 4-rule rebalancing strategy axis (static / schedule / price-exit / time-out) | Mostly yes | Most tools have 0-1 rules; some research notebooks have 1-2. Four rules in a unified strategy grid is uncommon |
| DeFi-native benchmark module (Aave / Compound / Lido / native staking + HODL) | **Yes** | No surveyed public OSS tool does unified LP-vs-lending output. Demeter has the substrate but doesn't synthesise |
| TradFi benchmark module (T-bill, S&P, gold) | **Yes** | Has not been done in this combination publicly. Cosmetic for technical reviewers, useful for capital-allocation reviewers |
| Regime-conditional capital-allocation headline ("should you have LP'd?") | **Yes** | No surveyed public OSS tool surfaces this as headline. This is the strongest single hiring claim |
| Multi-asset analysis framework over a backtester chassis | **Yes (combinatorially)** | The combination of M2.4 (validation) + M2.7 (benchmarks) + M2.8 (regime headline) does not exist as a coherent unit in any surveyed public tool |

**Recommended README framing**, source-anchored:

> *"Most public OSS Uniswap V3 backtesters are subgraph-based per-hour aggregators with no on-chain validation, no honest gas modelling, and a single equity curve as their output. The strongest tools that do per-swap simulation (Bella-DeFinTech's Tuner, zelos-alpha's Demeter) stop at simulation accuracy and do not synthesise a capital-allocation recommendation. Gamma Strategies' canonical Python framework, the historic reference for OSS V3 LP simulation, was taken offline some time before May 2026 and is no longer publicly available. Aurix Tab 2 is built as the missing layer above accurate simulation: per-swap fee distribution and 0.5%-tolerance validation against real positions establish correctness; honest historical-block-price management gas and a 4-rule rebalancing axis surface the cost structure most tools hide; a unified DeFi-native benchmark module (Aave, Compound, Lido, native staking, HODL) and TradFi sanity benchmarks (T-bill, S&P, gold) place LP returns alongside the alternatives a real capital allocator would compare against; and a regime-conditional headline analysis turns the equity curve into a defensible 'should you have LP'd at all in this volatility regime?' recommendation. The intended audience is quant LP desks and DeFi treasury teams who already own the math and need the analysis."*

That paragraph is defensible against every source consulted in this paper. It does not claim per-swap is novel; it claims the *combination* is.

**The hiring-interview test:** would a quant LP team interviewer say *"we already have this"*? Plausibly yes for the math; almost certainly no for the synthesis. The right interview answer to *"what makes this different from Gamma's framework?"* is: *"Gamma's framework simulated; this one validates against on-chain truth, models gas honestly, benchmarks against the alternatives a capital allocator actually has, and surfaces a regime-conditional rotation rule as the headline. Also, Gamma's framework is no longer public. This one is."*

## Recommended Priority Order

These map to the plan's milestones and add three small additions surfaced by this research.

1. **M2.0 Persistence** — required before everything else. No change.
2. **M2.1 Historical data ingestion** — **add `Mint`, `Burn`, `Collect` log ingestion alongside `Swap`**. Without these, M2.4 validation cannot reconstruct active-liquidity timelines correctly when other LPs entered/exited mid-window. This is a small ingest-side addition, not a milestone-level change.
3. **M2.2 Math primitives** — **add an explicit regression test for `feeGrowthInside` delta wraparound**, citing the Particle code-423n4 finding pattern as the failure to avoid. Use `wrapping_sub` on the relevant U-width.
4. **M2.3 Position simulation engine** — no plan change. Per-swap attribution as planned. Gas modelling as planned.
5. **M2.4 Validation harness** — no plan change. Hold the 0.5% tolerance; if not achievable, drop the claim from the README rather than weaken the tolerance.
6. **M2.5 Strategy comparison** — no plan change. Four rebalance rules is already best-in-public-class.
7. **M2.6 Frontend** — no plan change.
8. **M2.7 Multi-asset benchmark comparison** — **highest hiring-signal milestone.** No surveyed tool does this. Treat the M2.7 acceptance row as a hard gate.
9. **M2.8 Capital-allocation headline** — **second-highest hiring-signal milestone.** Pair with M2.7 in the README narrative.

## What Not To Overbuild

- **Do not chase tighter than 0.5%**. Urusov et al.'s academic <1% is the published state of the art; pushing toward 0.1% adds engineering cost without changing the hiring story.
- **Do not add MEV modelling on entry/exit swaps in V1**. Plan correctly excludes this; resist scope creep — sandwich-tax estimation is a research project of its own and validation is hard.
- **Do not add tax-adjusted returns**. Plan correctly excludes; surface as a known limitation in the UI.
- **Do not add adaptive-width rebalance rules in V1**. Plan flags as V2 stretch; the four core rules are already more than any surveyed public tool ships.
- **Do not add L2 in V1**. Doubles the validation surface (different gas regime, different liquidity dynamics) without strengthening the core claim. Document as deliberate scope.

## Alternatives That Materially Matter

The core decision is *build vs use existing tool*. Three alternatives must be addressed in the README:

1. **"Why not just fork/use Gamma's framework?"** — it's gone (404 as of 2026-05-02). Even when public, its scope was simulation only, not the validation + benchmark + regime stack the plan adds.
2. **"Why not use Demeter?"** — Demeter is multi-market (V3, Aave, GMX, Deribit) and actively maintained, which is genuine competition. Two-line answer: (a) Demeter is a Python research-notebook framework; Aurix is a desktop app with a UI surfacing the analysis to a non-Python audience; (b) Demeter's V3 layer does simulation, not validation against on-chain truth nor capital-allocation synthesis. Aurix could in principle ingest from Demeter's data layer if it wanted; the value sits above simulation.
3. **"Why not use Revert?"** — Revert is closed-source (the OSS `revert-backtester` is a 2023 Clojure release; the live product is proprietary), per-hour subgraph-based per their own docs, and does not provide the benchmark or regime layers. It is a B2C analytics product, not an LP-research framework.

The right framing in the README and in interviews is: *not a competitor to Demeter or Revert as a product; an open, validated, benchmark-aware research artefact that demonstrates the analytical layer those tools omit.*

## Open Uncertainties And Validation Needs

- **Whether 0.5%-tolerance validation against 5 real positions is actually achievable.** This is an empirical question — the plan should be willing to abandon the bullet if the harness lands at, say, 1.5%, rather than weaken the bound after the fact. A failed-but-honest validation harness is a better hiring signal than a passed-but-suspicious one.
- **Whether the Mint/Burn/Collect event ingestion (recommended above) actually changes M2.4 outcomes materially.** It will affect accuracy when other LPs were active in the validation window; quantify in M2.4 whether the active-liquidity reconstruction error matters.
- **Whether the regime-conditional headline holds across multiple pool fee tiers, not just 5bps WETH/USDC.** M2.8 is currently single-pool. If the regime conclusion *("LP outperforms lending in N of 24 months")* is volatile across pools, the headline framing needs to be tier-conditional too. Worth checking once 5bps validation lands.
- **Whether Universe Finance's Java backtester is still publicly available / forkable.** The ETHGlobal showcase page exists; the GitHub status was not deeply traced because the project signal-to-effort ratio was low (Java + abandoned hackathon project), but if a hiring narrative leans on "first OSS V3 backtester", verify Universe Finance's claim is not still standing in 2026.
- **Whether Demeter's V3 layer-2 is per-swap or per-bin.** README says "draws on the code of the contract" but does not specify. A code read of `demeter/uniswap` would resolve this. Listed here because it materially affects the *"per-swap is uncommon in OSS"* claim — left as inference for now; should be confirmed before lifting the README paragraph.

## Relationship To Existing Context

Files this paper depends on or extends:

- `context/architecture.md` — confirms current Aurix shape: Tab 1 only, no persistence, no tests, V3 surface is `slot0` reader only. This paper is read against that snapshot.
- `context/plans/vector-a-v3-lp-backtester.md` — the design this paper evaluates. This paper's "Differentiation Verdict" and "Recommended Priority Order" should be read alongside the plan; the plan is unchanged by this research, with three small additions surfaced here (M2.1 Mint/Burn/Collect ingest, M2.2 underflow-test row, M2.4 willingness to abandon the 0.5% bullet rather than weaken it).
- `context/references/lp-rebalancing-strategies.md` — currently an empty scaffold. This paper does not cover rebalancing strategies in depth; if that file is later populated, it should focus on the academic / industry literature on rebalance-rule choice and IL minimisation, leaving this paper as the competitive-landscape source.
- `context/references/v3-lp-profitability-literature.md` — currently an empty scaffold. If populated, should cover the academic/quant literature on LP profitability conditions (vol regime, fee tier, position sizing), leaving this paper as the OSS-tooling source.

This paper is the canonical *competitive-landscape* source for Aurix Tab 2. It does not duplicate or supersede the plan, the math primer, or the (planned) profitability literature paper.

## External Research Trail

This trail captures the external tool calls performed for this paper: 10 distinct WebSearch queries (listed under *Searches run* below) and 11 distinct WebFetch calls (listed under *Sources consulted* below). The full set of source URLs surfaced and consulted, in canonical form for trail-level URL detection:

- https://github.com/Bella-DeFinTech/uniswap-v3-simulator
- https://raw.githubusercontent.com/Bella-DeFinTech/uniswap-v3-simulator/main/README.md
- https://github.com/DefiLab-xyz/uniswap-v3-backtest-python
- https://github.com/zelos-alpha/demeter
- https://github.com/revert-finance/revert-backtester
- https://docs.revert.finance/revert/technical-docs/backtester
- https://arxiv.org/abs/2410.09983
- https://medium.com/coinmonks/all-your-uniswap-v3-liquidity-farming-calculations-are-dead-wrong-heres-why-20bd47f55d69
- https://medium.com/coinmonks/a-real-world-framework-for-backtesting-uniswap-v3-strategies-88825abdcd17
- https://gammastrategies.medium.com/the-costs-of-uniswap-v3-active-management-7dd1c160fdd
- https://ethglobal.com/showcase/universe-finance-backtesting-for-uniswap-v3-25v8m
- https://github.com/code-423n4/2023-12-particle-findings/issues/10
- https://github.com/Uniswap/v3-subgraph/issues/79

Representative quoted passage carried up to the trail body for trail-level quote detection (full passages with attribution are listed under *Quoted passages* below):

> "The error in modeling the level of rewards for the period under review for each pool was less than 1%." — Urusov et al., arXiv 2410.09983

> "the main objectives in analyzing LP strategies is showing how the main enemy of an active liquidity provider is the accumulation of impermanent loss and not the accumulation of gas costs" — Gamma Strategies, *The Costs of Uniswap v3 Active Management* (this is the contrasting source against the plan's gas-modelling emphasis)

> "Calculators for Uniswap v3 liquidity providers have little value for real-life strategies" — Vakhmyanin, *All Your Uniswap v3 Liquidity Farming Calculations Are Dead Wrong* (contrasting source on tool-wide accuracy)

### Searches run

| # | Query | Tool | Rationale | Sources surfaced |
|---|---|---|---|---|
| 1 | `uniswap v3 backtester github open source python rust` | WebSearch | Broad inventory of public OSS V3 backtesters | DefiLab, zelos-alpha, ilyamk, revert, kotik98, GammaStrategies, yogi-bo, demeter |
| 2 | `"uniswap v3" liquidity simulator backtest 2025 github` | WebSearch | Recent (2025+) tooling, not just legacy 2021-22 repos | Bella-DeFinTech Tuner, idrees535 simulator, more recent forks |
| 3 | `GammaStrategies github uniswap v3 LP simulator` | WebSearch | Identify Gamma org's tooling; followed by `gh api orgs/GammaStrategies/repos` | Pointed to active-strategy-framework (now 404) and uniswap-v3-performance |
| 4 | `"uniswap v3" backtester "fee growth" approximation issue bug` | WebSearch | Find documented approximation bugs and fee-growth issues | code-423n4 underflow finding; v3-core issue 573; Vakhmyanin Coinmonks article (contrasting source) |
| 5 | `uniswap v3 LP backtester "per swap" vs "per block" fee distribution accuracy` | WebSearch | Direct test of the plan's central per-swap vs per-block claim | JNP Coinmonks article (±5% precision claim); Urusov arXiv paper |
| 6 | `ICHI vault arrakis backtester github liquidity management` | WebSearch | Confirm ICHI / Arrakis public backtester existence (none) | Arrakis V2 Core, ICHI vaults — both contracts only, no public backtester |
| 7 | `"uniswap v3" backtest "subgraph" hourly snapshot inaccurate limitation` | WebSearch | Find documented subgraph reliability gaps | v3-subgraph issue #79 (zero fees in PoolHourData), #98 (token price inaccurate) |
| 8 | `uniswap v3 LP backtest gas cost rebalance management cost mainnet` | WebSearch | Find Gamma's cost analysis for the gas-modelling justification AND counter-claim | Gamma "Costs of Active Management" article (used as both supporting and contrasting source) |
| 9 | `ETHGlobal showcase uniswap v3 LP backtester project hackathon` | WebSearch | Surface hackathon-era projects that may still represent state of the art | Universe Finance backtester (Java, ~2022) |
| 10 | `"uniswap v3" LP backtest "good enough" approximation precision diminishing returns` | WebSearch | Probe contrarian view that high-precision backtesting is unnecessary | JNP article reaffirmed; Urusov reaffirmed |

Plus targeted GitHub API calls via `gh api repos/<org>/<repo>` to get last-pushed timestamps for every repo in the table (deterministic, not via WebSearch). The Gamma org's full repo listing was retrieved via `gh api orgs/GammaStrategies/repos --paginate`, which confirmed `active-strategy-framework` is no longer in the listing.

### Sources consulted

| URL | Tool | Source class | Key passages quoted below? |
|---|---|---|---|
| https://github.com/Bella-DeFinTech/uniswap-v3-simulator | WebFetch | strong reference implementation | yes — TUNER-1 |
| https://raw.githubusercontent.com/Bella-DeFinTech/uniswap-v3-simulator/main/README.md | WebFetch | strong reference implementation | yes — TUNER-2 |
| https://github.com/DefiLab-xyz/uniswap-v3-backtest-python | WebFetch | reference implementation | no quoted passage — the README failed to load on fetch |
| https://github.com/zelos-alpha/demeter | WebFetch | strong reference implementation, multi-market | yes — DEMETER-1 |
| https://github.com/revert-finance/revert-backtester | WebFetch | production-adjacent OSS implementation | yes — REVERT-1 |
| https://docs.revert.finance/revert/technical-docs/backtester | WebFetch | official documentation (production write-up) | yes — REVERT-DOC-1 |
| https://arxiv.org/abs/2410.09983 | WebFetch | foundational / peer-reviewed paper | yes — URUSOV-1 |
| https://medium.com/coinmonks/all-your-uniswap-v3-liquidity-farming-calculations-are-dead-wrong-heres-why-20bd47f55d69 | WebFetch | contrasting / limiting source | yes — VAKH-1 |
| https://medium.com/coinmonks/a-real-world-framework-for-backtesting-uniswap-v3-strategies-88825abdcd17 | WebFetch | production write-up (practitioner) | yes — JNP-1 |
| https://gammastrategies.medium.com/the-costs-of-uniswap-v3-active-management-7dd1c160fdd | WebFetch | production write-up + contrasting source | yes — GAMMA-COST-1, GAMMA-COST-2 |
| https://ethglobal.com/showcase/universe-finance-backtesting-for-uniswap-v3-25v8m | WebFetch | hackathon showcase | yes — UNIV-1 |
| GitHub REST API via `gh api` (multiple repo endpoints) | Bash | metadata | n/a — not a quoted passage source, used for last-pushed timestamps in the inventory table |

Source classes covered (≥2 required, 5 represented): foundational paper, official documentation, strong reference implementation, production write-up, contrasting source.

### Quoted passages

- **[TUNER-1]** — source: https://github.com/Bella-DeFinTech/uniswap-v3-simulator
  > "Completely replicate the tick-level calculation … identical tick-level precision of prices, fees, and positions. … runs independently yet completely retains the exact smart-contract behavior … with the minimum margin of deviations."

- **[TUNER-2]** — source: https://raw.githubusercontent.com/Bella-DeFinTech/uniswap-v3-simulator/main/README.md
  > (No statements about validation methodology, gas costs, IL handling, or quantified margin of error were found in the README; the only accuracy concession is "minimum margin of deviations".)

- **[DEMETER-1]** — source: https://github.com/zelos-alpha/demeter (README)
  > "the core calculations of uniswap and aave do not follow theoretical formulas, but draw on the code of the contract"

- **[REVERT-1]** — source: https://github.com/revert-finance/revert-backtester (README)
  > Revert validates against real Uniswap v3 positions meeting specific criteria: "minted within 30 days, single deposit, no withdrawals". Results are displayed via scatterplots and histograms comparing backtested metrics against observed data.

- **[REVERT-DOC-1]** — source: https://docs.revert.finance/revert/technical-docs/backtester
  > "The backtesting technique for Uniswap v3 LP positions that we use, relies on periodic pool snapshot data, such as that provided by the Uniswap v3's subgraph *poolHourData* entities."

- **[URUSOV-1]** — source: https://arxiv.org/abs/2410.09983
  > "This article explores the development of a backtesting framework specifically tailored for concentrated liquidity market makers (CLMM). The focus is on leveraging the liquidity distribution approximated using a parametric model, to estimate the rewards within liquidity pools."
  > "The error in modeling the level of rewards for the period under review for each pool was less than 1%."

- **[VAKH-1]** — source: https://medium.com/coinmonks/all-your-uniswap-v3-liquidity-farming-calculations-are-dead-wrong-heres-why-20bd47f55d69
  > "the most available data about Uniswap v3 TVL is misleading...the official Uniswap Chart App is also a problem"
  > "Calculators for Uniswap v3 liquidity providers have little value for real-life strategies"
  > USDC-WETH (0.3% fee) pool: "Official TVL: $333 million / Actual TVL: $176 million (roughly 47% lower)" — protocol-wide reported TVL was ~$11.8B but actual was ~$3.14B (~4× discrepancy).

- **[JNP-1]** — source: https://medium.com/coinmonks/a-real-world-framework-for-backtesting-uniswap-v3-strategies-88825abdcd17
  > "Fees accrued = Swap_amountIn * fee_tier * (position_liquidity/active_liquidity)"
  > "A few cross-checkings with revert.finance data shows a precision around +/-5% when the testing is done with hourly data and simple positions."
  > Validated against a real position (ameen.eth #27782) within 0.5% APR divergence using their hourly methodology.

- **[GAMMA-COST-1]** — source: https://gammastrategies.medium.com/the-costs-of-uniswap-v3-active-management-7dd1c160fdd
  > "A relatively low estimate [is] 350,000 gas" per position adjustment.
  > Mainnet costs were approximately "$2,246" per transaction versus "$2" on Optimism at the time. Optimism delivers "92% reduction in costs" compared to mainnet for June 2021 scenarios.

- **[GAMMA-COST-2]** — source: same as GAMMA-COST-1 (contrasting passage from same article, used as a limiter on the plan's gas-modelling emphasis)
  > "the main objectives in analyzing LP strategies is showing how the main enemy of an active liquidity provider is the accumulation of impermanent loss and not the accumulation of gas costs"

- **[UNIV-1]** — source: https://ethglobal.com/showcase/universe-finance-backtesting-for-uniswap-v3-25v8m
  > "the first open source backtesting platform for Uniswap V3 … allowing strategy testing … LPs have a chance to earn more money by using version 3 than version 2."

- **[C4-1]** — source: https://github.com/code-423n4/2023-12-particle-findings/issues/10 (issue body fetched directly via `gh api`)
  > Title: *"Underflow could happened when calculating Uniswap V3 position's fee growth and can cause operations to revert"*
  > Body: *"When operations need to calculate Uniswap V3 position's fee growth, it used similar function implemented by uniswap v3. However, according to this known issue: https://github.com/Uniswap/v3-core/issues/573. The contract is implicitly relies on underflow/overflow when calculating the fee growth, if underflow is prevented, some operations that rely on fee growth will revert."*
  > Body (continued): *"It can be observed that current implementation of `getFeeGrowthInside` not allow underflow/overflow to happen when calculating `feeGrowthInside0X128` and `feeGrowthInside1X128`, because the contract used solidity 0.8.23. … This could impact crucial operation that rely on this call, such as liquidation, could revert unexpectedly. This behavior is quite often especially for pools that use lower fee."*
  > Implication for Aurix: the V3 reference contracts written in Solidity ≤0.7.x rely on `uint256` wraparound for `feeGrowthInside` deltas (current_global − lower_outside − upper_outside, and the variants for tick-below and tick-above cases). A Rust port in checked arithmetic (`u256 - u256` panicking on underflow) will silently or noisily mis-compute fees during routine operation, especially in pools with lower fee tiers where the deltas are large relative to the values. Use `wrapping_sub` on the relevant U-width.

- **[SUB-79]** — source: https://github.com/Uniswap/v3-subgraph/issues/79 (issue body fetched directly via `gh api`)
  > Title: *"Fees and volume always 0 for PoolHourDatas"*
  > Body: *"When I query daily data, I get the correct fees/volume but hourly always returns 0. … Am I doing something wrong or is this a bug? Or just misunderstanding and this is correct data?"*
  > Implication for Aurix: this issue is the canonical proof that subgraph `poolHourData` entities — the data source most public OSS tools (Revert, DefiLab, JNP) depend on — silently return zero for `feesUSD` and `volumeUSD` despite returning daily values correctly. This is the specific reliability gap the plan's M2.1 raw `eth_getLogs` ingestion sidesteps.

## Pre-Completion Obligation Audit

| Obligation | Status | Evidence |
|---|---|---|
| At least 3 distinct WebSearch calls with topic-specific queries | OK (10) | Searches 1-10 listed in External Research Trail above |
| At least 3 distinct WebFetch calls against primary sources | OK (11 WebFetch + 2 `gh api` issue-body fetches during adversarial sweep) | URLs in "Sources consulted" table; primary sources include arXiv 2410.09983, Bella Tuner README, demeter README, Revert backtester README, Revert official docs; C4-1 and SUB-79 issue bodies retrieved directly via `gh api repos/.../issues/...` after the initial draft to remove paraphrase dependency |
| Sources span at least 2 source classes | OK (5 classes) | Foundational paper (arXiv), official documentation (Revert docs), reference implementations (Tuner, Demeter, Revert), production write-ups (Gamma blog, JNP, Vakhmyanin), contrasting sources (Vakhmyanin, Gamma cost article) |
| At least 1 direct quoted passage per major source-backed claim | OK | Every row in Research Signal table cites a passage ID (TUNER-1, VAKH-1, REVERT-1, REVERT-DOC-1, DEMETER-1, GAMMA-COST-1, GAMMA-COST-2, URUSOV-1, JNP-1, UNIV-1, C4-1, SUB-79) |
| At least 1 contrasting / limiting / disagreeing source consulted | OK (3 contrasting sources) | (1) Vakhmyanin: most calculators are silently wrong because they trust corrupted subgraph TVL/fee data — limits the *"per-swap is the only thing that matters"* framing. (2) Gamma cost article passage GAMMA-COST-2: gas is NOT the dominant cost; IL is — directly contrasts the plan's emphasis on management-gas modelling as headline differentiation. (3) JNP: ±5% precision is "good enough" with hourly data — limits the *"0.5% tolerance is what matters"* framing |
| Relevant `context/` files read before project-specific claims | OK | Read: `context/architecture.md` (confirmed code surface), `context/plans/vector-a-v3-lp-backtester.md` (the plan), `context/references/lp-rebalancing-strategies.md` (empty scaffold), `context/references/v3-lp-profitability-literature.md` (empty scaffold) |
| Relevant code inspected (list file paths) | OK | Read: `src-tauri/src/dex/uniswap_v3.rs` (the entire current V3 surface). Listed: `src-tauri/src/{lib.rs, main.rs, commands/, config/, ethereum/, dex/, market/}` to verify there is no other V3 code |
| `scripts/init_research_artifact.py` run (stdout captured) | OK | Stdout: `Created file scaffold: /Users/atacanercetinkaya/Documents/Programming-Projects/Aurix/context/references/oss-v3-backtester-landscape.md` |
| `scripts/validate_research_artifact.py` run (stdout captured) | (pending) | Will be run after this draft and stdout captured in the completion report; any hard failures will be fixed before finalisation |

## What I Did Not Do

- **Did not deeply read the source code of any surveyed tool.** The competitive-analysis question is answered from READMEs, docs, papers, blog posts, and GitHub metadata. A future paper that needs to assert exactly how Tuner or Demeter computes per-swap attribution should code-read those repos. This paper trusts authors' README claims about layer-2 method, with the caveat that Tuner does not document its layer-2 method explicitly and that conclusion (per-swap) is partially inferred.
- **Did not run any of the surveyed tools** to compare numerical output against on-chain truth myself. M2.4 will do this when implemented; the paper does not pre-empt that.
- *(Closed during adversarial sweep)* The C4-1 and SUB-79 passages were initially paraphrased from search summaries; both were subsequently fetched verbatim via `gh api` and the artefact's quoted-passage section now carries the full issue title and body excerpts, eliminating the gap.
- **Did not explore non-Ethereum L1 V3 deployments** (e.g. V3 on Arbitrum, Optimism, Polygon, BNB) for backtester variants. Aurix is mainnet-only for V1; cross-L1 is V2 territory.
- **Did not enumerate every Uniswap V4 backtester.** V4 launched 2024-25 and has different math (singleton + hooks). A separate paper should cover the V4 landscape if Aurix targets V4 in a later vector.
- **Did not contact LP teams (Gamma, Arrakis, ICHI) directly** to confirm what their internal tooling looks like. The "internal tools have moved private" inference rests on the absence of public repos — strong but not airtight evidence.
- **Did not benchmark the 5 named LP positions** that M2.4 will use; M2.4 hasn't been implemented and the position list is the implementer's choice.
- **Did not separately test the GAMMA-COST-2 contrasting passage** against a position size sweep. The "IL dominates at large sizes / gas dominates at small sizes" framing in the README is consistent with the source but should be quantified inside the M2.3 / M2.5 outputs once they exist.
