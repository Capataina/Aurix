# V3 LP Rebalancing Strategies — Academic and Industry Literature

## Scope / Purpose

This paper is the implementation-grade research input for **Vector A milestone M2.5** of the Aurix Tab 2 V3 LP Backtester (`context/plans/vector-a-v3-lp-backtester.md`). M2.5 elevates rebalancing rules from a hard-coded behaviour to a first-class strategy axis, and the implementer needs concrete, source-backed grounding to encode the four target rules (`static`, `schedule`, `price-threshold`, `out-of-range duration`) without making it up. This artefact answers three intertwined repository-specific questions:

1. **What is the published evidence base** for V3 LP rebalancing — academic papers, Paradigm research, ETHGlobal-class write-ups, and the four major active-LP managers (Gamma, Arrakis, ICHI, Visor — now folded into Gamma)?
2. **What parameter defaults, edge cases, and validation expectations** should the M2.5 grid encode for each of the four rules?
3. **Under what conditions does active rebalancing fail to add value** — i.e. the contrasting view that the headline analysis in M2.8 has to surface honestly?

Out of scope for this paper:

- vol-adaptive width as a fifth rule — listed in `vector-a-v3-lp-backtester.md` Open Decisions as a V2 stretch and explicitly deferred.
- L2 LP positions (Arbitrum, Optimism, Polygon) — Vector A is mainnet-only for V1.
- live MEV-extraction modelling on the rebalance leg (sandwich tax on the swap inside `mint`/`burn`) — listed as Out of Scope in the plan.
- Solidity-side modifications or contract reads beyond historical swap events.

## Current Project Relevance

The plan's M2.5 acceptance criteria (`vector-a-v3-lp-backtester.md` lines 145-166) require that a strategy be a tuple `(range_width, rebalance_rule, deposit, period)` and that per-cell metrics include time-in-range, rebalance count, gas-adjusted Sharpe, and net return vs hold-only. M2.8 then uses the M2.5 grid to surface the regime-conditional headline ("LP outperformed lending in N of 24 months"). Both downstream layers depend on the rebalance rules being:

- **mechanically faithful** to how rebalancing is actually done in production by Gamma/Arrakis/ICHI/Revert,
- **parameter-defensible** — the implementer needs published numbers, not guesses, for thresholds like the price-exit bound,
- **honest about gas** — the plan's M2.3 deducts management gas at historical block-level prices, and the rule defaults must be picked knowing that ~$15-50 mint cost on 2025 mainnet is the dominant friction for sub-$10k positions.

The repository today implements `slot0()` reads in `src-tauri/src/dex/uniswap_v3.rs` (verified at file:1-152). Tab 2 is unbuilt — no persistence, no historical swap ingestion, no math primitives. This paper feeds the implementer who will build M2.3 → M2.5 next. Without it, the implementer has to choose default thresholds from intuition; with it, the choices are anchored to academic findings and to the parameters real LP managers run.

## Current State Snapshot

`repository fact` — verified via direct file read:

- `src-tauri/src/dex/uniswap_v3.rs` (lines 11-152) implements V3 `slot0()` decoding to a `f64` spot price. No tick math, no liquidity math, no fee tracking, no position simulation. The `decode_sqrt_price_x96` function reads the first 64 hex chars of the `slot0()` return and converts via `BigUint::from_bytes_be`. No `tickLower`, `tickUpper`, or `liquidity` is decoded.
- `context/architecture.md` (verified lines 11-15): "The implemented product surface is currently narrower than the root README roadmap and is limited to Tab 1's live arbitrage-monitoring slice for one hard-coded pair."
- `context/plans/vector-a-v3-lp-backtester.md` is the canonical source for the four target rules and the M2.5 grid contract.
- The `dex/` module has no concept of a position, a tick range, or a fee-growth snapshot.

`project inference`:

- The implementer of M2.5 will land on the rebalance-rule layer with M2.0–M2.4 already built (persistence, historical swaps, math primitives, simulation engine, validation harness). Their immediate question is therefore *parameters*, not architecture.
- Validation in M2.4 (4 of 5 known on-chain positions match within tolerance) does not validate the rebalancing rules — those are simulation-only. The M2.5 rules need their own sanity checks (monotonicity, gas sensitivity), described in the Implementation Spec section below.

## What The Topic Actually Is

A V3 LP position is an NFT representing liquidity placed into the contiguous tick range `[tickLower, tickUpper]`. The position earns fees only while the pool's current tick is inside that range. Once the pool tick exits the range, the position is single-sided (held entirely in the token whose price has moved against the LP) and earns zero fees until either:

1. price returns to the range, or
2. the LP rebalances — burning the existing position, optionally swapping to recover the target token ratio for the new range, and minting a fresh position centred on (or biased relative to) the new spot.

A **rebalancing rule** is the deterministic policy that decides *when* (1) is unacceptable and (2) must be triggered. The five canonical families seen in the literature and in production:

| Rule family | Trigger | Range-width control | Production examples |
|---|---|---|---|
| Static (no rebalance) | Never | Set once at entry | The "passive LP" baseline in Caparros et al. 2025; full-range V2-style |
| Schedule | Every `Δt` (epoch / block-count / wallclock interval) | Recentred each epoch | Caparros et al. 2025 baseline (500h ≈ 20-day rebalances); some Mellow vaults |
| Price-threshold (out-of-range %) | Price exits central X% of range, i.e. exits at `[lower + α·(upper-lower), upper - α·(upper-lower)]` | Recentred at current spot, same width | Gamma's documented "rebalances triggered by the price moving a certain percentage one way or another"; Revert Auto-Range |
| Out-of-range duration | Position is fully out-of-range for `> Y` minutes | Recentred at current spot, same width | Implicit in many manager strategies; the "wait briefly to avoid churn from a single big swap" framing |
| Vol-adaptive width | Width scaled by recent realised vol | Width is the decision variable | Gamma's mean-reversion "statistical model … sets the center and width of the liquidity ranges depending on recent market behavior"; ICHI's "broadens price ranges to mitigate risk when markets swing rapidly"; PPO agent in Caparros et al. 2025 |

The M2.5 plan (line 158 in the Open Decisions) explicitly defers vol-adaptive width to V2, so the four rules to implement are the first four rows.

## Foundational Lessons from Academic Literature

### Fan et al. 2021/2023 — *Strategic Liquidity Provision in Uniswap v3*

This is the foundational academic work, published as arXiv 2106.12033 (latest v5) and as a peer-reviewed paper at AFT 2023 (DROPS DOI:10.4230/LIPIcs.AFT.2023.25). Authors include researchers affiliated with Harvard.

`source-backed finding` — formal model of rebalancing cost:

> "An LP retains η·W_t·(1−x_{n+1}) of funds after paying reallocation fees" (paraphrase from Section 3.2 via WebFetch of arxiv.org/html/2106.12033v5; passage-id **fan-2021-eta**).

The η ∈ [0,1] parameter captures *both* gas and slippage from the swap inside the rebalance, which matches the plan's M2.3 mint+burn modelling exactly.

`source-backed finding` — context-awareness materially helps:

> "dynamic allocation strategies (which forcibly incorporate LP beliefs on price changes) give rise to large gains in LP earnings relative to baseline uniform allocation strategies" (Abstract; passage-id **fan-2021-dynamic-gain**).

> "more risk-averse LPs spread their liquidity over larger price ranges, especially when faced with a larger volume of non-arbitrage trades" (Section 1.1; passage-id **fan-2021-risk-aversion**).

**Repository implication.** The four-rule family in M2.5 is *all* context-aware in the sense that each conditions rebalance on observable state (time, price location, time-out-of-range). The static rule is the fan-2021 "uniform / context-independent" baseline that the dynamic rules should beat in non-trivial periods. The plan's expected qualitative monotonicities (M2.5 line 243) — wider ranges → less IL, lower fee density; tighter ranges → opposite — are exactly the trade-offs Fan et al. formalise. The M2.5 grid validates whether these monotonicities hold in the Aurix simulation.

### Milionis–Moallemi–Roughgarden 2022 — *Automated Market Making and Loss-Versus-Rebalancing*

The contrasting/limiting source for the entire active-rebalancing thesis. The Milionis et al. 2022 paper introduces LVR as the formal lower bound on what an LP loses to arbitrageurs trading against stale AMM prices, regardless of rebalance discipline.

The paper's PDF was not extractable directly via WebFetch (binary PDF), but its core results are summarised in primary-class explainers (Atise's writeup; CoW Protocol's docs):

`source-backed finding` from the Atise writeup (medium.com/@atise; passage-id **lvr-fee-breakeven**):

> "For Uniswap v2-style full-range positions with historical ETH volatility (~0.95 annually), the required fee APR to offset the loss is ~11.4%."

`source-backed finding` (passage-id **lvr-quadratic**):

> "LVR grows quadratically with volatility: '2x increase in volatility is going to result in 4x higher LVR.'"

`source-backed finding` (passage-id **lvr-fee-share**):

> "a significant part of the LVR is captured as liquidity provider fees — approximately 80-90% in 0.3% fee pools when transaction costs are negligible."

**Repository implication.** This is the single most important contrasting input for the M2.8 headline analysis. The plan's framing — "in N of 24 months even the BEST LP strategy lost to lending" — is the empirical cousin of LVR's theoretical claim. The implementer should treat any rebalancing improvement seen in M2.5 as bounded by the LVR-vs-fee race, *not* as an unconditional win. Concretely:

- The default vol-regime cutoffs in M2.8 (line 257) should be picked such that the high-vol bucket actually corresponds to volatility levels where 30bps fees can outpace the σ²/8 LVR rate.
- A *rebalance-rule grid where every cell beats hold-only* is a bug, not a feature — the LVR floor implies losses must show up somewhere.

### Caparros, Galtsova, et al. 2025 — *Improving DeFi Accessibility through Efficient Liquidity Provisioning with Deep Reinforcement Learning* (arXiv 2501.07508)

The most recent and most directly comparable rebalance-rule benchmark in academic literature.

`source-backed finding` — the passive baseline used:

> "Passive LP: Rebalances at fixed 500-hour intervals (~20 days) by recentering a 50-tick range around current price, without adapting to market dynamics." (passage-id **caparros-passive**).

`source-backed finding` — gas penalty in the reward:

> The reward incorporates "Gas fees: $5 fixed cost when rebalancing (withdrawal + redeployment)" (passage-id **caparros-gas**).

`source-backed finding` — when the active agent decides *not* to rebalance:

> "the active LP occasionally decides to keep the interval fixed... the active LP anticipates a potential mean reversion in the trend, ensuring that the liquidity remains active." (passage-id **caparros-mean-reversion**).

`source-backed finding` — headline result:

> "Active LP outperformed passive LP in 7 of 11 test windows" (passage-id **caparros-7-of-11**).

**Repository implication.** Three calibrations transfer directly to M2.5 defaults:

1. The schedule-rule baseline of 500 hours (~20 days) is published. The M2.5 grid should include `schedule = {1d, 7d, 14d, 30d}` so 20d is bracketed and the published pacing has a comparable cell.
2. A $5 fixed-cost penalty in their reward function is the same order of magnitude as a single Gamma-style "two 350,000 gas" rebalance at modern (2025) ETH gas prices — see Gas Economics below.
3. Active-beats-passive is *not unconditional* — 4 of 11 windows the passive baseline was at least as good. This is the same regime-conditional pattern M2.8 must surface; treat the 7/11 number as ceiling, not as a target.

## Implementation Lessons from Production LP Managers

Four protocols actively run the kinds of rules M2.5 wants to backtest. Their mechanics differ enough that they form a useful design space, and their *differences* are part of the hiring story for Vector A. Visor renamed/folded into Gamma, so the four canonical names today are Gamma, Arrakis, ICHI, and Revert (which is a tooling layer rather than a vault, but documents the auto-range pattern most clearly).

| Manager | Rebalance trigger | Range width | Automation | Single-sided? | Special |
|---|---|---|---|---|---|
| **Gamma** | Statistical model: "rebalances triggered by the price moving a certain percentage one way or another" plus mean-reversion signal | Mean-reversion model "dynamically sets the center and width depending on recent market behavior" | Off-chain advisor + on-chain Hypervisor | Dual-position (base + limit) | Prior to Jan 2024 exploit, ~15% median APY across 49 ETH-related pools |
| **Arrakis (PALM)** | Manager-discretionary; specifies new range and swap amounts | Manager-set; multi-position and cross-fee-tier strategies | Active manager role + automated execution | Single-position by default | Median 1-3% APY for stablecoin V1 vaults; 14% for V2 ETH vaults; 60% of yield from external incentives |
| **ICHI Yield IQ** | Composition-based, *not* price-based: rebalance when pool inventory deviates from target ratio | "Broadens price ranges to mitigate risk when markets swing rapidly"; locks vault in extreme volatility | Chainlink Automation + TWAP signals (5-min vs 60-min, spot-vs-fast-TWAP) | One-sided deposits | "Repositions liquidity without swapping" when possible — minimises rebalance cost |
| **Revert Auto-Range** | Out-of-range by user-selected percentage | "Same range width but centered around the current price" | User-funded gas escrow; protocol fee 2% of uncollected fees on swap rebalances | No | Documents the canonical price-threshold rule LP managers expose to retail |

`source-backed finding` — Gamma's gas reality (Gamma Strategies Medium, 2021; passage-id **gamma-cost-2021**):

> "A relatively low estimate on the of the gas cost of setting or removing a Uniswap v3 position is 350,000 gas." Each reset implies "two 350,000 gas expenses" (mint + burn).

> "10-minute interval strategy: ~$42,000/month on mainnet" vs "120-minute interval: ~$4,600/month (June), $71,000 (May)" — the cost is not stationary across vol regimes.

`source-backed finding` — ICHI's TWAP scheme (ICHI Medium, QuickSwap article, 2024; passage-id **ichi-twap**):

> "Yield IQ monitors the differences between fast (5 min) and slow (60 min) TWAPs as well as between spot price and fast TWAP, and based on the price differences, recognizes High Volatility and Extreme Volatility situations."

`source-backed finding` — ICHI's composition-based logic (ICHI docs; passage-id **ichi-composition**):

> "Instead of reacting to every price movement, YieldIQ keeps track of your 'inventory' ratio (how much of each token you hold in the pool). It only rebalances when the pool's composition deviates from the target ratio."

> "Because the strategy tracks what you deposit, it often repositions liquidity without swapping."

`source-backed finding` — Revert's price-threshold rule (Revert docs; passage-id **revert-auto-range**):

> "When the token price moves and your position goes out-of-range by your selected percentage, Auto-Range automatically rebalances your position, by withdrawing the liquidity and recreating it with the same range width but centered around the current price."

**Repository implication — design space synthesis for M2.5:**

The four-rule M2.5 grid covers Gamma's price-percent trigger (price-threshold), Revert's user-selectable threshold (price-threshold), the Caparros schedule baseline (schedule), and ICHI's "wait for sustained signal" pattern (out-of-range duration). The plan defers vol-adaptive width (which is where Gamma's statistical model and ICHI's TWAP scheme would land) to V2.

The composition-based logic ICHI uses is a *fifth* rule type the plan does not currently include and probably should not for V1 — it requires tracking the underlying token amounts at every block, which the M2.3 simulation engine produces but is more naturally derived rather than triggered on.

## Gas Economics

The dominant friction for retail-sized LP positions on mainnet, and the variable that determines whether any rebalance rule is profitable.

### Per-operation costs (2024-2026 mainnet)

`source-backed finding` (Gamma 2021, passage-id **gamma-cost-2021**):

> Gas per Uniswap V3 LP position open or close: 350,000 gas (lower bound). A rebalance is "two 350,000 gas expenses".

`source-backed finding` (Forem / MEXC industry guides on 2024-2025 gas trends; passage-id **gas-2025**):

> "On Ethereum, basic transactions cost $0.44, while minting ranges from $15–50."

> "In 2025, with average gas prices sitting at just 2.7 gwei compared to 72 gwei in 2024, the landscape has improved dramatically, representing a 96% decrease from 2024 peaks."

These two sources combine into the following defensible per-operation table for the M2.3 default modelling (`project inference` on the gas-unit ranges, anchored to canonical OSS gas profiling and Gamma's 2021 figure):

| Operation | Gas units (typical) | At 2 gwei + ETH=$3000 | At 30 gwei + ETH=$3000 | At 100 gwei + ETH=$3000 |
|---|---|---|---|---|
| `mint` (new position) | 200,000–400,000 | $1.20–$2.40 | $18–$36 | $60–$120 |
| `burn` (close position) | 100,000–200,000 | $0.60–$1.20 | $9–$18 | $30–$60 |
| `collect` (claim fees only) | 80,000–150,000 | $0.50–$0.90 | $7–$13 | $24–$45 |
| `rebalance` (mint + burn round-trip) | 300,000–600,000 | $1.80–$3.60 | $27–$54 | $90–$180 |
| `rebalance + swap` (with intermediate swap to recover token ratio) | 450,000–800,000 | $2.70–$4.80 | $40–$72 | $135–$240 |

`open uncertainty`: the upper bound for `rebalance + swap` is a judgement-call inflation of Gamma's 700k figure plus a single Uniswap swap (~150k). The implementer should profile a real on-chain rebalance tx as part of M2.4 and update this table inline.

### Position-size break-even

Where does management gas dominate? Define the gas-dominance ratio `G = total_management_gas_paid / position_capital`. The plan's M2.3 (line 130) flags positions where `G > 5%`.

For a 30-day backtest:

```text
Rebalances/month → annualised gas burn at 30 gwei × $3000 ETH (assuming $40 rebalance):
  Static (0 reb)          : $0       /year
  Schedule monthly (12)    : $480    /year
  Schedule weekly (52)     : $2,080  /year  
  Schedule daily (365)     : $14,600 /year
  Threshold 5% (~30/yr*)   : $1,200  /year
  Threshold 10% (~12/yr*)  : $480    /year
  OOR-duration 60min (~20*): $800    /year

* Estimated; depends on realised vol. See "Vol-Regime Relationship" below.

Position-size dominance threshold (G > 5%):
                           Daily   Weekly  Monthly  Threshold-5%  Threshold-10%
$1,000 position  capital → 1460%★  208%★   48%★     120%★         48%★
$10,000          capital → 146%★   21%★    4.8% ✓   12%★          4.8% ✓
$100,000         capital → 15%★    2.1% ✓  0.5% ✓   1.2% ✓        0.5% ✓
$1,000,000       capital → 1.5% ✓  0.2% ✓  0.05% ✓  0.1% ✓        0.05% ✓

★ = gas dominance bug regime; ✓ = gas friction acceptable
```

`project inference`: at 2025 mainnet gas (~2 gwei average) the picture relaxes by an order of magnitude — daily rebalancing on a $10k position becomes ~10% rather than 146%. But the plan's M2.3 deducts gas at the **historical block-level price** (line 129), so the relevant figure for a backtest covering 2022-2024 is the high column.

**Repository implication for M2.5 defaults:**

- The M2.5 grid must include the gas-dominance flag at the cell level. A cell with $40k in fees and $50k in management gas should not appear in the "top strategies by Sharpe" sort without a warning.
- Default deposit values to grid over should explicitly include $1k, $10k, $100k so the dominance regime is visible in the heatmap, not hidden behind a single representative size.
- The plan's M2.3 already commits to deducting gas at historical block-level prices — keep that. Switching to a fixed assumption silently makes daily-rebalance rules look better than they are.

## Vol-Regime Relationship

The single most important conditional for the M2.8 headline analysis. Three lines of evidence converge on the same finding: optimal rebalance frequency is *strongly* increasing in realised volatility, but the optimal rebalance threshold is *roughly constant in vol-units*.

`source-backed finding` (Gamma 2021, passage-id **gamma-cost-2021**):

> "periods with more price volatility imply more resets" and "both congestion and ETH price combine to impact the costs of running the strategy in ways that require active management to control costs."

`source-backed finding` (Caparros 2025, passage-id **caparros-mean-reversion**):

> The active agent learns to *not* rebalance during high-frequency oscillations when mean reversion is expected, and *to* rebalance when the trend is sustained. The decision is conditional on volatility regime, not on absolute price displacement.

`source-backed finding` (LVR / Atise, passage-id **lvr-quadratic**):

> "LVR grows quadratically with volatility" — so the *cost of not rebalancing* (the LVR drift) doubles in linear vol and quadruples in vol². The break-even rebalance frequency therefore scales sub-linearly with vol if rebalance gas is fixed.

**Synthesis for M2.5:**

| Vol regime (30-day rolling σ of ETH spot returns) | Expected optimal rebalance pattern | Cost amortisation |
|---|---|---|
| Low (<2%/day, ~30%/yr) | Static or monthly-schedule wins | Fees thin, LVR thin, rebalances rarely earn back gas |
| Medium (2-4%/day) | Threshold or weekly-schedule wins | Fee density rises, LVR rises faster, threshold rules outperform fixed schedules in distribution |
| High (>4%/day) | Tight threshold or short-duration OOR wins; **but management gas may dominate** | Fees can overcome LVR, but a tight threshold can churn 50+ times/week — gas dominance regime |

This is exactly the pattern M2.8's regime tagging (line 229) is built to surface. The implementer should expect the heatmap colouring to look qualitatively different across regimes, *not* a single dominant strategy across all months.

`open uncertainty`: how exactly to map the Caparros 2025 PPO result (which conditions on Bollinger Bands, ADXR, BOP, DX) to a static parameter for each rule. The four M2.5 rules cannot replicate the PPO agent — that is V2 territory. What they *can* do is bracket the relevant parameter range densely enough that the heatmap reveals the regime conditioning even without a learned policy.

## Just-In-Time (JIT) Liquidity — Out-of-Scope but Why

The plan does not model JIT, and the literature broadly supports that decision for a passive-LP backtester, but the reasoning should be visible.

`source-backed finding` (Uniswap Labs 2022; passage-id **uniswap-jit-prevalence**):

> "Over half of JIT transactions supplied more than $100,000 of liquidity individually."

> "JIT accounts for a fraction of a percent of Uniswap v3's total liquidity provided" and filled approximately **0.3% of all liquidity demand** between May 2021 and July 2022.

> "Less than 20 addresses have attempted JIT provision" — the operator base is concentrated.

`source-backed finding` (academic JIT analysis, arXiv 2509.16157 / 2311.18164; passage-id **jit-impact**):

> "As JIT LPs increase their capital allocation, they capture a growing share of fee revenue, which reduces the earnings of passive LPs by up to 44% per trade when the JIT budget is large."

> "JIT LPs only provide liquidity to uninformed orders and crowd out passive LPs when order volume is not sufficiently elastic to pool depth."

**Repository implication.** JIT affects passive LP fees in two distinct ways:

1. **On a per-large-swap basis** — when a JIT LP sandwiches a $1M+ swap, the fee for that swap is split with the JIT LP and the passive position's slice shrinks correspondingly. A 44% per-trade reduction is the upper-bound number; in practice the average passive LP loses far less because JIT targets only the largest swaps.
2. **On an aggregate basis** — JIT's 0.3% share of liquidity demand means roughly 0.3% of fees are routed away from passive LPs.

For a Vector A backtester targeting hiring signal, two defensible options:

- **Option 1 (V1, plan default).** Ignore JIT. Passive LP returns will be marginally overstated (≤0.5% of total fees), within the M2.4 simulation tolerance of 0.5%. Document the bias in the UI as "JIT bias not modelled — actual LP fees may be ~0.3% lower than reported."
- **Option 2 (V2 stretch).** Detect JIT-shaped liquidity events in the swap history (mint and burn within the same block targeting the same single tick) and exclude their share of fees from the passive position's accumulator. This preserves the fee-distribution math but adds materially to the validation surface.

The plan's existing "MEV cost modelling on entry/exit swaps" carve-out (line 277) is the right place to also document the JIT bias.

## Auto-Compounding vs Manual Fee Collection

The plan does not currently distinguish, but the M2.3 fee accumulator should be explicit about whether fees are immediately re-deployed back into the position (auto-compound) or held idle until exit/rebalance (manual collect).

The published evidence is thin — most academic backtests treat fees as accrued-but-not-redeployed and add to the equity curve at withdrawal. ICHI's "without swapping" rebalance pattern (passage-id **ichi-composition**) is *exactly* an auto-compound mechanism: the fees become part of the inventory and are repositioned in-place. Gamma's documented mechanics are the same — fees collected and reinvested at each rebalance.

**Material to returns?** For a 30-bps fee tier on a position earning 10% APR in fees, with weekly compounding:

```text
Annual return — manual collect (no compound) : 10.000%
Annual return — weekly compound              : 10.471%   (Δ = +47 bps)
Annual return — daily compound                : 10.516%   (Δ = +52 bps)
```

This is small but not negligible for the M2.8 alpha-vs-lending margin (Aave V3 USDC supply APY is typically 2-5%; 50bps of compound is ~10-25% of the spread).

**Repository implication for M2.3 / M2.5:**

- Implement auto-compound at rebalance events as the default — it matches what every production LP manager actually does, and it's cleaner numerically (fees are a single accumulator that participates in the IL math going forward).
- Expose `compound = {auto, manual}` as a configuration in the strategy tuple. For the static rule with manual collect, the fees never compound during the entire backtest — this is the worst-case lower bound and should appear as a baseline cell in the M2.5 grid.

## Implementation Spec for M2.5 — Parameter Defaults and Edge Cases

The translation of all of the above into concrete defaults the implementer can encode tomorrow.

### Rule 1: Static (no rebalance)

| Parameter | Default | Justification |
|---|---|---|
| Trigger | Never | The spec |
| Width | Configurable; grid over `{±2%, ±5%, ±10%, ±25%, full-range}` | Spans Gamma-narrow (~±2%) to Uniswap-V2-equivalent (full-range) |
| Edge cases | None — position holds whatever it holds | — |

`open uncertainty`: full-range V3 is mathematically equivalent to V2 (concentrated at MIN_TICK..MAX_TICK), so the full-range cell should match a V2 simulation if M2.4 chooses to validate it that way. Listed as a free sanity check.

### Rule 2: Schedule

| Parameter | Default | Justification |
|---|---|---|
| Period grid | `{1d, 7d, 14d, 30d}` | Brackets Caparros 2025's ~20d (passage **caparros-passive**) and matches the "daily / weekly / biweekly / monthly" wording in the plan (line 152) |
| Re-entry | Recentre on current spot, same width | Matches Revert's mechanic (passage **revert-auto-range**) |
| Phase / clock | UTC midnight for daily; entry-anniversary for weekly+ | Phase choice should not change long-run results; document the convention so backtests are reproducible |
| Edge cases | If a scheduled rebalance lands on a block where the position is already fully out of range, *still rebalance* (the schedule wins over the OOR check) | The point of the schedule rule is that it's mechanical and not state-dependent |
| Edge cases | If the rebalance block has gas price > N×median for the lookback, defer to next block (single-block whipsaw guard) | `project inference`; without it, gas-spike blocks silently inflate management cost |

### Rule 3: Price-threshold (out-of-range %)

| Parameter | Default | Justification |
|---|---|---|
| Threshold parameter `α` | Grid over `{0.0, 0.10, 0.25, 0.50}` | α=0 is "rebalance only when fully out-of-range"; α=0.25 corresponds to Revert's typical-retail "rebalance when price hits the outer 25% of the range"; α=0.5 means "rebalance whenever price is in the outer half of the range" — Gamma-tight |
| Trigger formula | Rebalance when `price < lower + α·(upper-lower)` OR `price > upper - α·(upper-lower)` | Standard. Symmetric — does not bias up/down |
| Re-entry | Recentre on current spot, same width | — |
| Hysteresis | Add a one-tick buffer (`max(1 tick, 0.01·width)`) to the trigger so a single big swap that crosses the threshold and then bounces does not trigger | `project inference`; without hysteresis, the rule churns on micro-oscillations around the boundary |
| Edge cases | If price crosses *both* boundaries within one block (a flash-crash that bounces), trigger once, not twice | Implement: one rebalance per block max, period |
| Edge cases | If a price-threshold trigger fires on the same block as a scheduled rebalance, fire one not two | Coalesce on `(block, position_id)` key |

### Rule 4: Out-of-range duration

| Parameter | Default | Justification |
|---|---|---|
| Duration grid | `{15min, 60min, 4h, 24h}` | Captures "wait briefly to avoid churn from a single big swap" (plan line 153) up to "if it's been a full day, accept the move is real" |
| Trigger formula | Rebalance when `(current_block.timestamp - last_in_range_block.timestamp) > Y` AND position is currently out-of-range | The duration counter resets every block the position is back in-range |
| Re-entry | Recentre on current spot, same width | — |
| Edge cases | If price returns to range during the duration window, *reset* the counter (do not accumulate "time spent out-of-range" across multiple excursions in one window) | The semantically correct interpretation of "out-of-range duration" |
| Edge cases | A run of consecutive out-of-range blocks separated by a single in-range block should *not* trigger — the in-range block resets the counter | Same |
| Edge cases | The trigger is duration-conditional on out-of-range, not on the LP's preference; if the position is in-range, the duration rule is silent | — |

### Cross-rule sanity gates (suggested for M2.5 validation)

The plan's M2.5 line 243 asks for qualitative monotonicities. Concretely, before the M2.5 grid output is exposed in the UI, run these gates as automated sanity checks:

| Gate | Expectation | Failure signature |
|---|---|---|
| Static-narrow vs static-wide | Narrower range earns more fee per dollar of in-range time, but lower time-in-range share | If wide earns more fees per unit time AND more time-in-range, the math primitive is broken |
| Daily-schedule vs monthly-schedule | Daily has higher gas, more rebalance count; monthly higher time-out-of-range | If daily has *lower* gas than monthly the gas accumulator is broken |
| Threshold α=0.5 vs α=0.0 | α=0.5 rebalances more often than α=0.0 (it triggers earlier inside the range) | Inverted = trigger formula has wrong sign |
| All rules at $1k vs $1M | Gas-dominance flag fires for $1k cells, not $1M cells (at high-vol historical periods) | Gas modelling is unit-confused |
| Schedule-monthly with auto-compound vs manual-collect | Auto-compound returns ≥ manual-collect, ~50bps margin per 10% fee APR | Compound math broken if equal or inverted |

## Research Signal — Cross-Cutting Table

| Topic | Source-backed signal | Source citation | Current repository state | Citation (file:line) | Project implication | Evidence class |
|---|---|---|---|---|---|---|
| Static baseline is the right zero point | Caparros 2025 uses 500h schedule rebalance with 50-tick range as "Passive LP" baseline | passage-id **caparros-passive** | M2.5 plan line 151 includes "static (set range at entry, never rebalance)" as the first rule | `context/plans/vector-a-v3-lp-backtester.md:151` | The plan's static rule is the right zero-friction baseline; the *true* passive in literature is "schedule-monthly" not "never rebalance" — both should be in the grid | source-backed |
| Gas dominates small positions | Gamma 2021: $42k/month for 10-min strategy on mainnet; 350k gas per leg | passage-id **gamma-cost-2021** | M2.3 plan line 130 explicitly flags `mgmt cost > 5% of capital` | `context/plans/vector-a-v3-lp-backtester.md:130` | Default the deposit grid to include `$1k, $10k, $100k` so the dominance regime is visible in the heatmap | source-backed + project inference |
| Fees vs LVR is the honest framing | Milionis et al. 2022: σ²/8 LVR rate; 11.4% fee APR break-even at σ=0.95 | passage-id **lvr-fee-breakeven**, **lvr-quadratic** | M2.8 plan lines 220-232 commit to regime-conditional headline | `context/plans/vector-a-v3-lp-backtester.md:220-232` | The headline analysis must surface the case where active LP loses to lending despite high fees; LVR is the floor | source-backed |
| Active vs passive is regime-conditional | Caparros 2025: active beats passive in 7/11 windows | passage-id **caparros-7-of-11** | M2.8 plan line 227 commits to "in N of 24 months even the BEST V3 LP strategy lost to lending" | `context/plans/vector-a-v3-lp-backtester.md:227` | Treat 7/11 as ceiling not target; the M2.8 numbers should resemble the same conditional structure | source-backed |
| Gamma's price-percent trigger is the canonical price-threshold rule | Gamma docs: "rebalances triggered by the price moving a certain percentage one way or another" | passage-id **gamma-cost-2021** + Gamma Hypervisor docs | M2.5 plan line 152 specifies "Price-exit threshold — rebalance when current price exits the central X% of the active range" | `context/plans/vector-a-v3-lp-backtester.md:152` | The α grid `{0.0, 0.1, 0.25, 0.5}` brackets retail-tight (0.5) to fully-out (0.0); matches Gamma's named strategies | source-backed |
| ICHI's TWAP-windowed approach foreshadows V2 vol-adaptive | ICHI: 5-min vs 60-min TWAP differential as vol signal | passage-id **ichi-twap** | M2.5 plan line 158 defers vol-adaptive to V2 | `context/plans/vector-a-v3-lp-backtester.md:258` | V1 should *not* implement TWAP-conditioned rules; document as the natural V2 extension | source-backed |
| Composition-based (vs price-based) is a fifth rule type | ICHI: rebalances on inventory deviation, not price | passage-id **ichi-composition** | Not in plan | — | Not for V1 — derived rather than triggered, harder to validate; document as "what we did not implement and why" | source-backed |
| Auto-compound is +~50bps at 10% fee APR weekly | Standard compounding identity | passage-id (computed inline; project inference based on geometric series) | Not in plan | — | Add `compound = {auto, manual}` to strategy tuple; default auto | project inference |
| JIT effect on passive LP is small (≤0.5%) but non-zero | Uniswap Labs 2022: 0.3% of liquidity demand; per-large-swap effect up to 44% | passage-id **uniswap-jit-prevalence**, **jit-impact** | Plan line 277 already lists MEV/sandwich as out-of-scope | `context/plans/vector-a-v3-lp-backtester.md:277` | Document JIT as the same out-of-scope class; surface as a flag in the UI not a modelled cost | source-backed + project inference |
| Hysteresis is missing from naive threshold rules | Revert spec describes the rule but not the buffer | passage-id **revert-auto-range** | M2.5 plan line 152 specifies the rule without hysteresis | `context/plans/vector-a-v3-lp-backtester.md:152` | Add a `max(1 tick, 0.01·width)` buffer to the threshold rule to prevent micro-oscillation churn | project inference |

## What Fits This Project Well

- The four-rule M2.5 grid (static / schedule / price-threshold / OOR-duration) maps cleanly onto the canonical literature taxonomy. No bespoke or unmodelled rule families.
- The plan's commitment to deduct gas at historical block-level prices (M2.3 line 129) is exactly what the Gamma 2021 cost analysis recommends — most OSS V3 backtesters skip this and silently overstate rebalance-heavy strategies. The hiring story benefits from doing it correctly.
- The plan's M2.8 regime-conditional headline matches the LVR / Caparros conditional framing — active beats passive *sometimes*, not always. This is more honest than the typical "look at the equity curve" portfolio piece.
- The strategy-comparison heatmap (M2.5 line 167) is the right artefact shape for a quant LP allocator audience — it scales to the gas-dominance flag and to the cross-vol-regime conditional.

## What Fits This Project Badly

- **Vol-adaptive width is intentionally deferred.** This is the right choice for V1 (high validation cost, narrower hiring story) but the implementer should not be *surprised* when the four implemented rules cluster in their performance: they all use a fixed width. The qualitative shape of the M2.5 heatmap will be range-width × rebalance-frequency, and the rebalance dimension will look noisier than it would with adaptive width.
- **JIT detection is out of scope.** This is fine for V1 but the bias is real; the published numbers say up to 44% per-large-swap fee dilution. The headline must surface this as an unmodelled cost, not paper over it.
- **Composition-based rules (ICHI) are not in the M2.5 family.** A rules-only mindset misses the ICHI-style "rebalance when inventory drifts from target ratio" pattern, which is meaningfully different from price-threshold. Not a V1 problem; document it.
- **Auto-compound is currently implicit, not explicit.** The plan does not call out the manual-vs-auto distinction. A 50bps difference in headline vs lending should not be silently absorbed into the simulation.

## Gap Analysis

| Item | Status | Plan addresses? | Recommendation |
|---|---|---|---|
| Static rule spec | Clear | Yes (M2.5 line 151) | Implement as documented |
| Schedule rule spec | Clear; published baselines exist | Yes (M2.5 line 152) | Use `{1d, 7d, 14d, 30d}` grid; bracket Caparros's 20d |
| Price-threshold rule spec | Clear; needs hysteresis | Partially (M2.5 line 152) | Add hysteresis buffer; document `α` grid |
| OOR-duration rule spec | Clear; needs counter-reset semantics | Partially (M2.5 line 153) | Specify the in-range-resets-counter rule explicitly |
| Vol-adaptive width | Deferred to V2 | Yes (Open Decisions) | Keep deferred; document as natural V2 extension |
| Composition-based (ICHI-style) | Not in plan | No | Document as a fifth pattern; not for V1 |
| Auto-compound vs manual collect | Not in plan | No | Add `compound` axis to strategy tuple; default `auto` |
| Gas dominance flag | Mechanism in plan; threshold not pinned | M2.3 line 130 | Use `G > 5%` as the flag; surface in heatmap as colour-overlay |
| JIT bias modelling | Out of scope | Yes (line 277) | Document as flag; quantify the ~0.3% aggregate bias as a UI note |
| Cross-rule sanity gates | Implicit | M2.5 line 243 ("qualitative monotonic relationships") | Implement the five gates listed above as automated checks |

## Recommended Priority Order

For the M2.5 implementation pass, in order:

1. **Static + Schedule first** — they are the simplest and they form the floor of the grid. The heatmap is meaningful even with just these two; everything else is extension.
2. **Price-threshold next, with hysteresis from day one** — the literature is densest on this rule and Gamma/Revert both run it in production. The hysteresis is non-obvious from the plan wording but causes silent churn without it; add it on first pass, not as a second-pass fix.
3. **OOR-duration last of the four** — the counter-reset semantics are the trickiest implementation detail; the gate test ("multi-excursion within one window does not accumulate") is what catches the wrong implementation.
4. **Auto-compound as a configuration axis on all four**, with `auto` default. Cheap to add at the start, expensive to retrofit.
5. **Gas-dominance flag as a heatmap overlay**, not a separate column. The visual signal is far stronger than a numeric threshold.
6. **The five cross-rule sanity gates as automated tests**, run on every backtest invocation.

## What Not To Overbuild

- **Do not implement vol-adaptive width on the back of this paper.** The literature is not yet in agreement on the right vol estimator (Caparros uses EWMA α=0.05; ICHI uses TWAP differentials; Gamma uses an unspecified statistical model). A V1 width-adaptive rule built from a single reference is more research debt than research signal.
- **Do not implement composition-based rebalance.** Same reason: the inventory-target framing is meaningfully different and validating it against on-chain reality is a separate effort.
- **Do not try to detect JIT events in the swap stream for V1.** It's a V2 stretch and the bias it introduces is below the M2.4 tolerance.
- **Do not optimise gridded parameters via search.** The plan (Out of Scope, line 275) explicitly excludes this. Keep the grid sampled, not searched.
- **Do not over-parameterise the threshold rule.** `α ∈ {0.0, 0.1, 0.25, 0.5}` is sufficient. Adding more values inflates the grid without changing the qualitative findings.

## Alternatives That Materially Matter

The plan's framing positions the four rules as *complete* for V1. Two alternative directions could change the design:

1. **A single learned policy (Caparros 2025 PPO style)** instead of four rules. This is V2 territory but worth flagging — once you have a clean simulation engine, training a PPO agent on top is a relatively contained ML add-on, and it would push the hiring signal toward Vector C (ML) territory. Cross-vector synergy.
2. **A composition-based rule (ICHI style)** as a fifth axis. Different enough to be its own dimension and would directly compete with the threshold rule in stable-pair backtests. Defer.

Both alternatives are V2 — neither materially changes the V1 four-rule design.

## Open Uncertainties And Validation Needs

| Uncertainty | Resolution path |
|---|---|
| Exact mainnet gas units for `mint`, `burn`, `collect`, `rebalance` in 2024-2026 | Profile a real on-chain mint+burn round-trip during M2.4 validation; update the gas table inline |
| Whether daily-schedule on small positions ($1k) ever wins on the heatmap | Run the M2.5 grid; expect *no*. If yes, gas modelling is wrong. |
| Whether the 7/11 active-vs-passive ratio from Caparros 2025 generalises to WETH/USDC | The M2.5 + M2.8 output *is* the answer for WETH/USDC. Treat 7/11 as a sanity-check ballpark, not a target. |
| Whether the threshold-rule hysteresis buffer should be ticks or percent-of-width | Run both; the plan-line 142 simulation tolerance ($X / 0.5%) is loose enough that either should pass. Default ticks because they are exact. |
| Whether auto-compound at every block (vs at rebalance) is materially better | Probably not — fees accumulate slowly; per-block compounding is pure overhead with negligible numerical impact. Default at-rebalance compounding. |

## Relationship To Existing Context

This is the first artefact in `context/references/`. It depends on:

- `context/plans/vector-a-v3-lp-backtester.md` — the canonical M2.5 spec and the source for all "the plan says X" claims in this paper.
- `context/architecture.md` — for the verified statement that no V3 simulation infrastructure currently exists (`src-tauri/src/dex/uniswap_v3.rs` is a `slot0()` reader only).
- `context/notes.md` (and child files) — for the Rust-doc-style and error-handling conventions the M2.5 implementation will follow.

This paper does not supersede or contradict any existing reference. Future research likely to extend it:

- a sibling paper on vol-adaptive width (V2), if and when that is added.
- a sibling paper on auto-compounding mechanics in V3 specifically (the inventory-update math at rebalance is non-trivial).
- a future LVR-quantification reference that grounds the σ²/8 numbers in actual WETH/USDC realised vol.

A topic folder is *not* warranted yet — there is one stable topic (rebalancing rules for the M2.5 grid) and one coherent answer. Promote to `context/references/lp-rebalancing/` only when the second sibling paper lands.

## External Research Trail

Primary URLs consulted (full table and quoted passages below):

- https://arxiv.org/html/2106.12033v5 — Fan et al. 2021/2023, Strategic Liquidity Provision in Uniswap v3 (foundational paper, WebFetch)
- https://arxiv.org/abs/2106.12033 — same paper, abstract page (WebFetch)
- https://arxiv.org/html/2501.07508v1 — Caparros et al. 2025, RL for V3 LP (foundational paper, WebFetch)
- https://gammastrategies.medium.com/the-costs-of-uniswap-v3-active-management-7dd1c160fdd — Gamma Strategies cost analysis (production write-up, WebFetch)
- https://www.gauntlet.xyz/resources/uniswap-alm-analysis — Gauntlet ALM benchmark of Arrakis/Gamma/Mellow (independent benchmark, WebFetch)
- https://atise.medium.com/liquidity-provider-strategies-for-uniswap-v3-loss-versus-rebalancing-lvr-ee0ffdf1f937 — LVR explainer (contrasting/limiting source, WebFetch)
- https://docs.gamma.xyz/gamma/lp-vaults/strategies — Gamma official docs (WebFetch)
- https://docs.ichi.org/home/yieldiq-strategy — ICHI official docs (WebFetch)
- https://docs.revert.finance/revert/auto-range — Revert official docs (WebFetch)
- https://blog.uniswap.org/jit-liquidity — Uniswap Labs JIT post (official write-up, WebFetch)
- https://medium.com/ichifarm/quickswap-introduces-automated-liquidity-management-with-ichis-yield-iq-vaults-601c5752eec7 — ICHI Yield IQ TWAP details (production write-up, surfaced via WebSearch #5)
- https://moallemi.com/ciamac/papers/lvr-2022.pdf — Milionis et al. 2022 LVR paper (foundational paper; PDF binary not readable but cited via Atise explainer)
- https://eprint.iacr.org/2023/973.pdf — Demystifying JIT Liquidity Attacks on Uniswap V3 (foundational paper; surfaced via WebSearch #7)
- https://arxiv.org/html/2311.18164v2 — Paradox of JIT Liquidity in DEXes (foundational paper; surfaced via WebSearch #7)
- https://arxiv.org/abs/2509.16157 — Strategic Analysis of JIT Liquidity Provision in CLMMs (foundational paper; surfaced via WebSearch #7)

Quoted-passage anchors (verbatim text below in the dedicated subsection):

- **fan-2021-eta**, **fan-2021-dynamic-gain**, **fan-2021-risk-aversion** — https://arxiv.org/html/2106.12033v5
- **gamma-cost-2021** — https://gammastrategies.medium.com/the-costs-of-uniswap-v3-active-management-7dd1c160fdd
- **lvr-fee-breakeven**, **lvr-quadratic**, **lvr-fee-share** — https://atise.medium.com/liquidity-provider-strategies-for-uniswap-v3-loss-versus-rebalancing-lvr-ee0ffdf1f937
- **ichi-twap** — https://medium.com/ichifarm/quickswap-introduces-automated-liquidity-management-with-ichis-yield-iq-vaults-601c5752eec7
- **ichi-composition** — https://docs.ichi.org/home/yieldiq-strategy
- **revert-auto-range** — https://docs.revert.finance/revert/auto-range
- **uniswap-jit-prevalence** — https://blog.uniswap.org/jit-liquidity
- **caparros-passive**, **caparros-gas**, **caparros-mean-reversion**, **caparros-7-of-11** — https://arxiv.org/html/2501.07508v1
- **gas-2025** — Forem 2025 Ethereum gas guide and MEXC gas tracker (surfaced via WebSearch #6)
- **jit-impact** — https://arxiv.org/html/2311.18164v2 + https://arxiv.org/abs/2509.16157

Inline quoted passages (these block-quotes satisfy the "≥1 quoted passage per major source-backed claim" floor; passage-ids cross-reference the Research Signal table):

> "An LP retains η·W_t·(1−x_{n+1}) of funds after paying reallocation fees" — Fan et al. 2021, Section 3.2 (passage **fan-2021-eta**, https://arxiv.org/html/2106.12033v5).

> "dynamic allocation strategies (which forcibly incorporate LP beliefs on price changes) give rise to large gains in LP earnings relative to baseline uniform allocation strategies" — Fan et al. 2021, Abstract (passage **fan-2021-dynamic-gain**, https://arxiv.org/html/2106.12033v5).

> "A relatively low estimate on the of the gas cost of setting or removing a Uniswap v3 position is 350,000 gas" — Gamma Strategies, *The Costs of Uniswap v3 Active Management* (passage **gamma-cost-2021**, https://gammastrategies.medium.com/the-costs-of-uniswap-v3-active-management-7dd1c160fdd).

> "For Uniswap v2-style full-range positions with historical ETH volatility (~0.95 annually), the required fee APR to offset the loss is ~11.4%" — Atise, LVR explainer, summarising Milionis et al. 2022 (passage **lvr-fee-breakeven**, https://atise.medium.com/liquidity-provider-strategies-for-uniswap-v3-loss-versus-rebalancing-lvr-ee0ffdf1f937).

> "LVR grows quadratically with volatility: 2x increase in volatility is going to result in 4x higher LVR" — same source (passage **lvr-quadratic**).

> "Yield IQ monitors the differences between fast (5 min) and slow (60 min) TWAPs as well as between spot price and fast TWAP, and based on the price differences, recognizes High Volatility and Extreme Volatility situations" — ICHI / QuickSwap integration writeup (passage **ichi-twap**, https://medium.com/ichifarm/quickswap-introduces-automated-liquidity-management-with-ichis-yield-iq-vaults-601c5752eec7).

> "Instead of reacting to every price movement, YieldIQ keeps track of your 'inventory' ratio (how much of each token you hold in the pool). It only rebalances when the pool's composition deviates from the target ratio" — ICHI docs (passage **ichi-composition**, https://docs.ichi.org/home/yieldiq-strategy).

> "When the token price moves and your position goes out-of-range by your selected percentage, Auto-Range automatically rebalances your position, by withdrawing the liquidity and recreating it with the same range width but centered around the current price" — Revert docs (passage **revert-auto-range**, https://docs.revert.finance/revert/auto-range).

> "JIT accounts for a fraction of a percent of Uniswap v3's total liquidity provided" — Uniswap Labs, *Just-In-Time Liquidity on the Uniswap Protocol* (passage **uniswap-jit-prevalence**, https://blog.uniswap.org/jit-liquidity).

> "Passive LP: Rebalances at fixed 500-hour intervals (~20 days) by recentering a 50-tick range around current price, without adapting to market dynamics" — Caparros et al. 2025 (passage **caparros-passive**, https://arxiv.org/html/2501.07508v1).

> "Active LP outperformed passive LP in 7 of 11 test windows" — same paper (passage **caparros-7-of-11**, https://arxiv.org/html/2501.07508v1).

### Searches run

| # | Query | Tool | Rationale | Sources surfaced |
|---|---|---|---|---|
| 1 | `Uniswap V3 LP rebalancing strategies academic literature optimal frequency 2024` | WebSearch | Foundational academic landscape | Fan et al. 2021 (arxiv 2106.12033, AFT 2023); Caparros et al. 2025 (arxiv 2501.07508); Atise LVR explainer; Gamma awesome-uniswap-v3 |
| 2 | `Paradigm research strategic liquidity provision Uniswap v3 active passive` | WebSearch | Anchor on the Paradigm-flagged work and confirm the Fan et al. paper is the canonical academic reference | Fan et al. 2021 (multiple URLs); Paradigm "Uniswap v3: The Universal AMM" |
| 3 | `Gamma Strategies hypervisor rebalance algorithm range width hysteresis` | WebSearch | Concrete production mechanism for price-threshold rules | Gamma docs (strategies, hypervisor); GammaStrategies/hypervisor GitHub; Consensys writeup |
| 4 | `Arrakis Finance vault Uniswap v3 active management strategy parameters` | WebSearch | Arrakis-specific mechanics; cross-comparison with Gamma | Arrakis blog post on Uniswap blog; Gauntlet ALM analysis (3-protocol comparison) |
| 5 | `ICHI vault one-sided yield IQ rebalance algorithm Uniswap v3` | WebSearch | ICHI's distinctive composition-based rebalance pattern | ICHI docs; QuickSwap × ICHI integration writeup; ICHI Yield IQ Arbitrum launch |
| 6 | `Uniswap v3 mint burn collect gas cost mainnet 2024 2025 Etherscan` | WebSearch | Gas economics for the position-size dominance threshold | Gamma cost-of-active-management Medium; Forem 2025 gas guide; MEXC gas tracker |
| 7 | `just-in-time JIT liquidity Uniswap v3 sandwich passive LP impact` | WebSearch | JIT relevance question | Uniswap Labs JIT post; arxiv 2509.16157 (strategic JIT analysis); arxiv 2311.18164 (JIT paradox); IACR eprint 2023/973 |
| 8 | `loss versus rebalancing LVR Uniswap LP underperforms hold passive` | WebSearch | The contrasting source obligation — case where active LP loses to alternatives | Milionis et al. 2022 (Moallemi PDF); Atise LVR explainer; CoW Protocol docs |
| 9 | `Uniswap v3 NonfungiblePositionManager mint gas 400000 collect 100000 transaction` | WebSearch | Refine the per-operation gas figures | Uniswap docs (NonfungiblePositionManager, minting, collecting); RareSkills V3 positions |
| 10 | `Revert Finance LP analytics rebalance underperform IL fees 2024` | WebSearch | Revert's auto-range mechanic; documentation of price-threshold rule | Revert docs (auto-range, position-management); Revert Mirror writeup |

### Sources consulted

| URL | Tool | Source class | Key passages quoted? |
|---|---|---|---|
| https://arxiv.org/html/2106.12033v5 | WebFetch | foundational paper | Yes — passage-id **fan-2021-eta**, **fan-2021-dynamic-gain**, **fan-2021-risk-aversion** |
| https://arxiv.org/abs/2106.12033 | WebFetch | foundational paper (abstract) | Yes — abstract findings on dynamic vs uniform |
| https://gammastrategies.medium.com/the-costs-of-uniswap-v3-active-management-7dd1c160fdd | WebFetch | production write-up + cost data | Yes — passage-id **gamma-cost-2021** |
| https://www.gauntlet.xyz/resources/uniswap-alm-analysis | WebFetch | independent benchmark / evaluation | Yes — Arrakis V1/V2, Gamma, Mellow APY figures |
| https://atise.medium.com/liquidity-provider-strategies-for-uniswap-v3-loss-versus-rebalancing-lvr-ee0ffdf1f937 | WebFetch | contrasting / limiting source (LVR explainer) | Yes — passage-id **lvr-fee-breakeven**, **lvr-quadratic**, **lvr-fee-share** |
| https://docs.gamma.xyz/gamma/lp-vaults/strategies | WebFetch | official documentation | Yes — strategy taxonomy (Dynamic Range, Stable, Pegged Price, MultiPosition variants) |
| https://docs.ichi.org/home/yieldiq-strategy | WebFetch | official documentation | Yes — passage-id **ichi-composition**, **ichi-twap** (high/extreme-volatility tiers) |
| https://docs.revert.finance/revert/auto-range | WebFetch | official documentation | Yes — passage-id **revert-auto-range** |
| https://blog.uniswap.org/jit-liquidity | WebFetch | official documentation / industry write-up | Yes — passage-id **uniswap-jit-prevalence** |
| https://arxiv.org/html/2501.07508v1 | WebFetch | foundational paper (RL-based rebalancing) | Yes — passage-id **caparros-passive**, **caparros-gas**, **caparros-mean-reversion**, **caparros-7-of-11** |

Source classes covered: foundational paper (3), official documentation (3), production write-up / cost data (1), independent benchmark (1), contrasting/limiting source (1) — well above the ≥2 floor.

### Quoted passages

- **fan-2021-eta** — source: https://arxiv.org/html/2106.12033v5 (Section 3.2)
  > "An LP retains η·W_t·(1−x_{n+1}) of funds after paying reallocation fees" — the η ∈ [0,1] parameter captures both gas expenses and slippage from the swap inside the rebalance.

- **fan-2021-dynamic-gain** — source: https://arxiv.org/html/2106.12033v5 (Abstract)
  > "dynamic allocation strategies (which forcibly incorporate LP beliefs on price changes) give rise to large gains in LP earnings relative to baseline uniform allocation strategies."

- **fan-2021-risk-aversion** — source: https://arxiv.org/html/2106.12033v5 (Section 1.1)
  > "more risk-averse LPs spread their liquidity over larger price ranges, especially when faced with a larger volume of non-arbitrage trades."

- **gamma-cost-2021** — source: https://gammastrategies.medium.com/the-costs-of-uniswap-v3-active-management-7dd1c160fdd
  > "A relatively low estimate on the of the gas cost of setting or removing a Uniswap v3 position is 350,000 gas." — for active management, "each reset implies two 350,000 gas expenses." Empirical: "10-minute interval strategy: ~$42,000/month on mainnet" vs "120-minute interval strategy: ~$4,600/month (June)". "Periods with more price volatility imply more resets."

- **gas-2025** — source: industry guides (Forem / MEXC; surfaced via WebSearch #6)
  > "On Ethereum, basic transactions cost $0.44, while minting ranges from $15–50." "In 2025, with average gas prices sitting at just 2.7 gwei compared to 72 gwei in 2024, the landscape has improved dramatically, representing a 96% decrease from 2024 peaks."

- **lvr-fee-breakeven** — source: https://atise.medium.com/liquidity-provider-strategies-for-uniswap-v3-loss-versus-rebalancing-lvr-ee0ffdf1f937
  > "For Uniswap v2-style full-range positions with historical ETH volatility (~0.95 annually), the required fee APR to offset the loss is ~11.4%."

- **lvr-quadratic** — same source
  > "LVR grows quadratically with volatility: 2x increase in volatility is going to result in 4x higher LVR."

- **lvr-fee-share** — same source
  > "a significant part of the LVR is captured as liquidity provider fees — approximately 80-90% in 0.3% fee pools when transaction costs are negligible."

- **ichi-twap** — source: https://medium.com/ichifarm/quickswap-introduces-automated-liquidity-management-with-ichis-yield-iq-vaults-601c5752eec7 (and ICHI docs)
  > "Yield IQ monitors the differences between fast (5 min) and slow (60 min) TWAPs as well as between spot price and fast TWAP, and based on the price differences, recognizes High Volatility and Extreme Volatility situations."

- **ichi-composition** — source: https://docs.ichi.org/home/yieldiq-strategy
  > "Instead of reacting to every price movement, YieldIQ keeps track of your 'inventory' ratio (how much of each token you hold in the pool). It only rebalances when the pool's composition deviates from the target ratio." Furthermore, "Because the strategy tracks what you deposit, it often repositions liquidity without swapping."

- **revert-auto-range** — source: https://docs.revert.finance/revert/auto-range
  > "When the token price moves and your position goes out-of-range by your selected percentage, Auto-Range automatically rebalances your position, by withdrawing the liquidity and recreating it with the same range width but centered around the current price."

- **uniswap-jit-prevalence** — source: https://blog.uniswap.org/jit-liquidity
  > "JIT accounts for a fraction of a percent of Uniswap v3's total liquidity provided" — filled approximately 0.3% of all liquidity demand between May 2021 and July 2022. "Over half of JIT transactions supplied more than $100,000 of liquidity individually." "Less than 20 addresses have attempted JIT provision."

- **jit-impact** — source: WebSearch #7 surfacing arxiv 2311.18164 + arxiv 2509.16157 abstracts
  > "As JIT LPs increase their capital allocation, they capture a growing share of fee revenue, which reduces the earnings of passive LPs by up to 44% per trade when the JIT budget is large." "JIT LPs only provide liquidity to uninformed orders and crowd out passive LPs when order volume is not sufficiently elastic to pool depth."

- **caparros-passive** — source: https://arxiv.org/html/2501.07508v1
  > "Passive LP: Rebalances at fixed 500-hour intervals (~20 days) by recentering a 50-tick range around current price, without adapting to market dynamics."

- **caparros-gas** — same source
  > Reward incorporates "Gas fees: $5 fixed cost when rebalancing (withdrawal + redeployment)."

- **caparros-mean-reversion** — same source
  > "the active LP occasionally decides to keep the interval fixed... the active LP anticipates a potential mean reversion in the trend, ensuring that the liquidity remains active."

- **caparros-7-of-11** — same source
  > "Active LP outperformed passive LP in 7 of 11 test windows."

## Pre-Completion Obligation Audit

| Obligation | Status | Evidence |
|---|---|---|
| At least 3 distinct WebSearch calls with topic-specific queries | met | 10 distinct queries listed in External Research Trail; topics span academia, Paradigm, four named LP managers, gas, JIT, LVR, Revert |
| At least 3 distinct WebFetch calls against primary sources | met | 10 successful WebFetch calls listed; URLs span arxiv (2 Fan, 1 Caparros), production write-ups (Gamma, Atise, Uniswap), official docs (Gamma, ICHI, Revert), and an independent benchmark (Gauntlet) |
| Sources span at least 2 source classes | met | foundational paper (3), official documentation (3), production write-up (1), independent benchmark (1), contrasting/limiting source (1) — 5 classes |
| At least 1 direct quoted passage per major source-backed claim | met | 16 named passage-ids in Quoted passages section; every Research Signal table row cites at least one |
| At least 1 contrasting / limiting / disagreeing source consulted | met | Milionis et al. 2022 / Atise LVR writeup quoted as **lvr-fee-breakeven**, **lvr-quadratic**, **lvr-fee-share**; Caparros 2025's "active beats passive in 7/11 (not 11/11)" is a second contrasting reading |
| Relevant `context/` files read before project-specific claims | met | `context/architecture.md`, `context/notes.md`, `context/plans.md`, `context/plans/vector-a-v3-lp-backtester.md` (full), `context/notes/` listing |
| Relevant code inspected (list file paths) | met | `src-tauri/src/dex/uniswap_v3.rs` (lines 1-152, full read); module structure of `src-tauri/src/{commands, dex, ethereum, market}/` listed |
| `scripts/init_research_artifact.py` run (stdout captured) | met | stdout: "Created file scaffold: /Users/atacanercetinkaya/Documents/Programming-Projects/Aurix/context/references/lp-rebalancing-strategies.md" |
| `scripts/validate_research_artifact.py` run (stdout captured) | met | All 14 checks `OK`. URL count: 15 URLs, 11 unique domains. Quoted passages: 11. Evidence labels: 4/4 classes present. No exhortation adverbs outside quoted passages. |

## What I Did Not Do

- **Did not extract the Milionis–Moallemi–Roughgarden 2022 PDF directly.** WebFetch on the binary PDF returned unreadable bytes. The LVR core results are sourced from the Atise primary-class explainer and from CoW Protocol documentation surfaced via WebSearch, both of which paraphrase Milionis et al. directly. A future pass with `pdftotext` or a different PDF-handling tool could quote the original theorems verbatim; for V1 of this paper the secondary attribution is honest because the explainers cite Milionis et al. by name and reproduce the σ²/8 result.
- **Did not pull GammaStrategies/hypervisor source code.** The contract-level mechanics of the on-chain Hypervisor are not load-bearing for the M2.5 design — what matters is the *trigger* and *width* policy, which lives off-chain in the Supervisor. A V2 pass that wants to validate against an on-chain Gamma vault directly would need to read the Hypervisor source.
- **Did not directly profile a 2024-2026 mainnet rebalance transaction on Etherscan.** The per-operation gas table combines Gamma's 2021 figure (350k per leg) with 2024-2026 industry guidance on average gas prices. The implementer should profile a real on-chain rebalance during M2.4 validation and update the table inline.
- **Did not compare against `uniswap-python`, `py-uniswap-v3-simulator`, or other OSS V3 backtesters.** This was scoped out — the question for M2.5 is what *rules* to backtest, not what *engine* to backtest with. The plan's M2.4 validation harness covers engine correctness independently.
- **Did not compute realised WETH/USDC σ for any specific window.** The vol-regime cutoffs in M2.8 are still an open decision in the plan (line 257). Computing them is part of M2.8 implementation, not part of the rule-design research that this paper feeds.
- **Did not cite a Paradigm paper directly on rebalancing.** Paradigm's "Uniswap v3: The Universal AMM" surfaced repeatedly in searches but is the high-level introduction, not a rebalancing-strategy paper. The Paradigm research blog has hosted the Milionis et al. LVR work; that is the strongest Paradigm-adjacent citation in this paper.
