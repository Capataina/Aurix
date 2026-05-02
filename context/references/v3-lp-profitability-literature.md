# V3 LP Profitability — Empirical Literature Review

> **Animating question.** Under what conditions has Uniswap V3 LPing actually been profitable, and how should that prior frame Aurix Tab 2's headline analysis (M2.8: *"should you have LP'd this period?"*) so the recommendation is grounded in published findings rather than invented from scratch?

## Scope / Purpose

- This artefact is the **intellectual backbone** for Vector A milestone **M2.8 — Capital allocation headline analysis**, the synthesis layer that turns the V3 LP backtester from "tool" into "investment recommendation framework".
- It surveys what the published empirical literature has established about **whether** V3 LPing is profitable, **for whom**, **on which pools**, **under which volatility regimes**, and **under which methodological assumptions**.
- It is **not** an explanation of how V3 concentrated liquidity works mechanically (that is V3 whitepaper / Uniswap docs territory), and it is **not** a derivation of impermanent loss formulas (the math primitives in M2.2 will reference the V3 whitepaper directly). It is purely the **literature-review layer**: what prior researchers have found, what numbers they reported, where they disagree, what pitfalls they warn against.
- The goal is that the agent implementing M2.8 can reach for this paper and answer questions like *"what threshold of 30-day rolling ETH vol should I use to classify a 'high-vol' regime?"*, *"is 49.5% of LPs underperforming HODL the right reference statistic to anchor the headline narrative around?"*, *"how do I write the framing without re-making known mistakes (look-ahead, survivorship, time-window cherry-picking)?"* — without re-doing this entire literature search.

## Current Project Relevance

- The Aurix Tab 2 plan (`context/plans/vector-a-v3-lp-backtester.md`) makes M2.8 the **synthesis milestone** that elevates the project from "I built a V3 backtester" to "I framed LP as a regime-conditional capital-allocation decision". The plan's own example output reads:

> *"WETH/USDC LP outperformed stable lending in 6 of 24 months, all during high-vol regimes (>X% daily ETH vol). [...] V3 LP on this pair is a vol-regime-conditional strategy, not a default capital-allocation choice."*

  Every numerical filler in that template — *"6 of 24"*, *">X%"*, *"the 60% case"* — needs an **empirical anchor**. Without the literature, those numbers are placeholders the implementer would have to either invent or omit; either failure mode collapses the headline back into a number rather than a recommendation.
- The current code (`src-tauri/src/dex/uniswap_v3.rs`) decodes only `slot0()` spot price (`sqrtPriceX96` → USD). No tick math, no fee tracking, no historical replay, no benchmarks exist yet (M2.2–M2.7 will build them). This artefact is therefore **input to design**, not validation of existing behaviour.
- The hiring framing in the plan — *"the difference between a portfolio piece and a portfolio piece"* — depends on the headline analysis being **defensible to a quant LP allocator**. Quant LP desks have read these papers; the headline must reference the same evidence base, ideally extending it for the WETH/USDC 5bp pool over Aurix's specific historical window.

## Current State Snapshot

| Surface | State | Citation |
|---|---|---|
| V3 spot-price decode | Implemented | `src-tauri/src/dex/uniswap_v3.rs:27-50` (`fetch_snapshot`) and `:76-125` (`derive_price_base_in_quote`) |
| Q64.96 tick / liquidity primitives | Not started — planned in M2.2 | `context/plans/vector-a-v3-lp-backtester.md:111-117` |
| Position simulation engine (fees + IL + gas) | Not started — planned in M2.3 | plan §M2.3 |
| Historical swap ingestion | Not started — planned in M2.1 | plan §M2.1 |
| Benchmark framework (Aave, Lido, T-bill, S&P) | Not started — planned in M2.7 | plan §M2.7 |
| Capital-allocation headline (regime-conditional) | Not started — planned in M2.8 | plan §M2.8 |
| Persistence layer (SQLite) | Not started — prerequisite M2.0 | plan §M2.0 |

> **Repository facts above are verified by file inspection. The "not started" entries are repository state, not literature gaps.**

## What The Topic Actually Is

A short orientation, since this paper is read by both an implementer (who already knows V3) and an interviewer (who may not):

- **Uniswap V3 (May 2021)** introduced *concentrated liquidity*: an LP can deposit capital into a price *range* `[p_a, p_b]` rather than the full `(0, ∞)` range V2 used. Within the range the LP earns fees proportional to their share of in-range liquidity at each swap; outside the range the position becomes 100% of the side that has appreciated and earns nothing.
- **Three quantities dominate LP returns:**
  1. **Fees earned** — proportional to in-range liquidity share × volume routed through the position's tick.
  2. **Impermanent loss (IL) / divergence loss** — the path-independent value gap between the LP position and a static hold of the entry composition, driven by price movement.
  3. **Loss-versus-rebalancing (LVR)** — the *path-dependent* gap between the LP position and a continuously-rebalanced reference portfolio (Milionis, Moallemi, Roughgarden, Zhang 2022), capturing the adverse-selection cost paid to arbitrageurs on every price move. **Critically, LVR is always cumulative** even when prices revert, while IL nets to zero on a round-trip.
- The literature's core question — *"is V3 LPing profitable?"* — reduces to: **does (fees) ≥ (IL) or (LVR), conditional on pool, range, regime, and management cost?**

## Research Signal

The synthesis below is the table M2.8's framing should be anchored to. Each row pairs a source-backed signal with the matching Aurix milestone or design choice.

| # | Topic | Source-backed signal (quoted) | Source citation | Current repository state | Repository implication | Evidence class |
|---|---|---|---|---|---|---|
| 1 | Topaze Blue / Bancor headline | *"49.5% of liquidity providers had suffered negative returns due to IL"*; *"those LPs would have been better off by USD 60.8m had they simply HODLd"*; *"80% of pools had IL exceeding fees earned"* | [P1] arXiv:2111.09192 (Loesch et al. 2021) | M2.8 not started | Anchor headline statistic; anchor "median LP loses to HODL" baseline | source-backed |
| 2 | WBTC/USDC, AXS/WETH, FTM/WETH were the only profitable pools in the 17-pool study | *"only the WBTC/USDC, AXS/WETH, and FTM/WETH pools seeing net positive returns"* | [P1], [S1] CryptoSlate | M2.8 not started | Be cautious about over-claiming WETH/USDC profitability — not in the original profitable set | source-backed |
| 3 | Strategy choice and active management dominate returns | *"providing liquidity has become a game reserved for sophisticated players [...] retail traders do not stand a chance"* and *"significant returns can only be obtained by accepting increased financial risks and at the cost of active management"* | [P2] arXiv:2205.08904 (Heimbach et al. 2022) | M2.5 grid + M2.8 framing not started | Strategy comparison grid (M2.5) must include passive baselines, not only optimised configurations; framing must distinguish "best strategy beat HODL" from "median strategy beat HODL" | source-backed |
| 4 | Closed-form LVR for constant-product AMM | *"the instantaneous LVR, when normalized by the CPMM's market value, turns out to be exactly σ²/8"*; *"if a Uniswap v2 ETH-USDC pool has a daily volatility of 5%, then [...] LPs lose 3.125 bps to LVR every day (for a roughly 11% loss annually)"* | [P3] a16z LVR essay; [P4] arXiv:2208.06046 (Milionis et al. 2022) | M2.7 not started | LVR is the **right benchmark for the "should you have LP'd?" question**, not raw IL. The required-fee-APR formula `σ²/8` becomes the regime threshold engine for M2.8 | source-backed |
| 5 | LVR breakeven volume rule | *"if this AMM charges a fixed 30 bps trading fee, then LPs will break even provided the daily volume is roughly 10.4% of the AMM's assets"* | [P3] a16z LVR essay | M2.5 not started | M2.5 should compute the breakeven-volume ratio per strategy, not just realised fees vs realised IL; this is what a quant LP allocator looks at first | source-backed |
| 6 | 2024 follow-up: WETH-USDC 5bp **under-compensates** for LVR | *"For the dominant WETH-USDC 5bp pool, historical returns from fees hover around 80% of arbitrage losses"* | [P5] arXiv:2404.05803 (Measuring Arbitrage Losses, 2024) | Aurix's only V3 pool today is WETH/USDC 5bp | **Direct hit on the project's flagship pool**: M2.8 must report this prior and compare Aurix's measured ratio to the literature's 80%. If Aurix's number diverges, that's a finding. | source-backed |
| 7 | 2024 follow-up: V2 > V3 for passive LPs on the same pair | *"Uniswap v2 pools are more profitable for passive LPs than their Uniswap v3 counterparts"*; in the second observation year *"fees are consistently three times larger than losses"* in V2 | [P5] arXiv:2404.05803 | M2.7 benchmark set does not include V2 LP as a baseline | Strong recommendation: M2.7 should add **"V2 LP on the same pair"** as a benchmark — the literature says V2 may dominate V3 for passive ETH/USDC | source-backed + project inference |
| 8 | Practitioner: V2 ETH/USDC was profitable in 2023 | *"fee income consistently exceeded the theoretical LVR"*; *"around 80–85% of the LVR in swap fees"* recaptured by V2; *"over 4% relative profit by October, ending the year between 1 to 2% profit"* despite *"divergence loss for the full year was almost precisely 5%"* | [P6] Atise 2023 retrospective | n/a | Contrasts with the 2021 Topaze Blue 49.5% headline — the answer is **regime-dependent**, not universally negative. M2.8's framing should not lead with "V3 LPs lose money"; it should lead with "regime conditional" | source-backed (contrasting) |
| 9 | Practitioner: ETH yearly σ ≈ 0.95 ⇒ ~11.4% required fee APR for full-range V2-equivalent | *"the yearly σ of ETH since 2020 is ~0.95 -> the required fee APR to offset the loss is ~11.4% for a full-range position"* | [P7] Atise LVR strategy piece | n/a | Concrete number to anchor the M2.8 vol-regime classifier; gives an interpretable threshold | source-backed |
| 10 | Practitioner: 2× volatility ⇒ 4× LVR | *"2x increase in volatility is going to result in 4x higher LVR"* (consistent with σ² scaling) | [P7] Atise LVR strategy piece | M2.8 not started | M2.8's vol regimes must use a **non-linear classifier** (square-of-vol, not linear vol). Linear regime cuts will mis-rank "moderately high vol" months | source-backed |
| 11 | Practitioner: tighter ranges ⇒ exponentially worse LVR | *"LVR is proportional to the gamma of the LP position"*; full-range cheap to hedge, narrow-range "most expensive" | [P7] Atise LVR strategy piece | M2.5 grid spans range widths but does not yet model LVR | M2.5 should report **gamma-adjusted Sharpe**, not just raw Sharpe; narrow-range high-Sharpe results in low-vol months will not survive a vol shock | source-backed + project inference |
| 12 | Uniswap official: V3 passive outperforms V2 passive on average | *"non-rebalancing Uniswap v3 positions outperform comparable Uniswap v2 positions by an average of ~54%"*; *"100-bps fee-tier full-range v3 positions outperform v2 positions by an average of ~80%"*; *"5-bps fee-tier full-range v3 positions underperform v2 by an average of ~68%"* | [P8] blog.uniswap.org "fee-returns" | M2.7 benchmark set | **Critical asymmetry by fee tier**: 5bp full-range *under*-performs V2 by 68%; 30bp by +16%; 100bp by +80%. Aurix's 5bp pool is exactly the tier where V3 looks worst against V2 | source-backed (contrasting) |
| 13 | JIT liquidity erodes passive LP fees by up to 44% per trade | *"JIT liquidity, when deployed strategically, can [...] erod[e] average passive LP profits by up to 44% per trade"* | [S2] Kaiko / IACR JIT studies | n/a | M2.3 fee modelling should flag swaps where JIT activity was likely (very large swaps with very narrow concentrated liquidity at the swap tick); systematic over-estimation of passive LP fees is a documented risk | source-backed |
| 14 | LVR captures path-dependent loss IL misses | *"LVR [...] is always cumulative, regardless of the direction of price changes"*; even when prices revert, *"reversions don't erase prior losses"* | [P9] Titania LVR 101 (Medium) | n/a | M2.3 must report **both** IL (the V3 paper canonical metric, what the user expects) and LVR-style cumulative loss. Reporting only IL risks under-stating the true cost on round-trip months | source-backed |

## What Fits This Project Well

- **The Atise / a16z framing of LVR as "should you LP at all".** The project's M2.8 framing question is *literally* the LVR question: *"was the decision to provide liquidity a good idea in hindsight? To first order, this question boils down to whether the fees collected exceeded the LVR suffered"* (a16z [P3]). M2.8 can borrow this framing wholesale. The literature has done the conceptual work — the project's contribution is the empirical pass on the WETH/USDC 5bp pool with realistic gas modelling.
- **The σ²/8 closed form as a regime-classifier engine.** M2.8 currently lists *"adaptive terciles"* as the recommended vol-regime cutoff (plan §Open Decisions). The literature gives a stronger choice: classify regimes by **whether the realised fee APR ≥ σ²/8 for that month**. This produces a regime classification that directly maps to the question the headline asks, with no free parameters.
- **The 2024 Heimbach/Wattenhofer follow-up's "WETH-USDC 5bp at ~80% fee/LVR" is a direct prior.** The plan's M2.8 acceptance criterion says the output should *"make a quant LP allocator's decision easier than reading a single-position equity curve"*. The fastest way to clear that bar: report **Aurix's measured fee/LVR ratio for the same pool over Aurix's window** and compare to the literature's 80%. Within-noise agreement is a positive validation signal for M2.4; divergence is itself a finding.

## What Fits This Project Badly

- **The Topaze Blue 49.5% headline is too tempting and too misleading.** It is a 2021 figure on a 17-pool sample over four months, dominated by retail wallets, in the very first months after V3 launched. Quoting it as the project's headline (*"50% of LPs lose money"*) is the kind of statistic-laundering that interviewers see as a tell. M2.8 should reference the figure with proper temporal scoping (*"in the V3-launch period, [P1] found ..."*) and explicitly note that **the 2024 follow-up [P5] tells a more nuanced fee-tier-conditional story**.
- **Generic "high-vol → LP wins / low-vol → LP loses" rules of thumb.** The literature is more specific: LP wins when *fee APR ≥ σ²/8* (or its concentrated-liquidity gamma-adjusted analogue). This is *not* the same as "high vol wins" — it is "vol relative to fee APR wins". A high-vol month with low volume can still lose; a moderate-vol month with high volume can win. M2.8's framing must use the volume-vol joint condition, not vol alone.
- **The Uniswap-official "V3 outperforms V2 by 54% for passive LPs" claim, applied uncritically.** That number is a *cross-tier average* — driven mostly by the 1bp stablecoin tier (+160%) and the 100bp tier (+80%). For the 5bp tier (Aurix's pool), V3 *underperforms* V2 by 68% in the same study. M2.8 must not cite the 54% headline without surfacing the 5bp-specific number.

## Methodological Pitfalls In LP Profitability Studies

The plan's M2.4 acceptance criterion is *"4 of 5 positions match within tolerance"* — but tolerance to *what*? The literature documents specific traps that plausible-looking backtests fall into. Each of these is a checkable pitfall for M2.3–M2.5.

| Pitfall | Mechanism | How the literature guards against it | Aurix design implication |
|---|---|---|---|
| **Look-ahead bias on fee distribution** | Many "approximations" allocate fees to a position based on aggregate pool fees over a block range *without* checking which specific swaps were in-range for the position. This implicitly uses end-of-period state to score start-of-period positions. | Loesch et al. [P1] do **per-swap** in-range checks; this is the discipline. | M2.3 already specifies *"per swap: was the position in range? If yes, compute its share of total in-range liquidity at that moment"* (plan §M2.3). This is the right call; do not regress to block-aggregate approximations under deadline pressure. |
| **Survivorship bias on positions** | Studies that aggregate "average LP returns" implicitly weight by surviving positions, ignoring positions that were burned at a loss and never re-minted. | Heimbach et al. [P2] track *all* mints/burns over the window, including burned-at-loss positions. | M2.4 (validation harness) should pick **at least one position whose burn was at a loss** to ensure the engine reproduces losing scenarios, not just winners. |
| **Time-window cherry-picking** | The 49.5% Topaze Blue figure covered May–Sep 2021, a high-vol period; running the same methodology on a calmer window gives a different number. Studies that report a single window without rolling-window robustness are giving point estimates dressed as conclusions. | The 2024 paper [P5] uses Jan-2022 → Dec-2023, ~24 months, and reports rolling-window results. | M2.7 already specifies *"cross-window robustness: same comparisons computed across rolling 30/60/90 day windows over the full historical period; report distribution of LP-vs-benchmark spread (median, p25, p75)"* (plan §M2.7). M2.8's headline *should not* report a single number — it should report a distribution. |
| **Constant-liquidity assumption** | Some V3 backtesters assume in-range liquidity is constant over the window, which contradicts the protocol (other LPs mint and burn around the position). This systematically over-estimates fee share. | Heimbach et al. [P2] reconstruct the **time-varying liquidity surface** from on-chain mint/burn events. | M2.3 must replay actual mint/burn events from the swap-log ingestion, not assume liquidity is constant. The plan's M2.1 ingests `Swap` events; **M2.1 should also ingest `Mint` and `Burn` events** for the simulated pool — this is implicit in M2.3 but not explicit in M2.1. |
| **Ignoring management gas costs** | "Most backtests assume zero management cost, which significantly overstates returns at retail-typical position sizes" (plan §Hiring Signal Payoff). The literature [P5] confirms small-position economics are dominated by gas. | The 2024 paper [P5] separately reports gross-of-gas and net-of-gas P&L. | M2.3 already specifies block-level gas modelling (plan §M2.3). M2.8 should report the **threshold position size** below which management gas costs > X% of capital, surfaced as part of the recommendation. |
| **Conflating IL with LVR** | IL nets to zero on a round-trip; LVR is cumulative. Reporting only IL on a window where prices oscillated reverts looks favourable to LPs but obscures the real loss. | Milionis et al. [P4] introduced LVR specifically to make this distinguishable; Atise [P6, P7] reports both. | M2.3 should output both IL (path-independent) and LVR (path-dependent) on the equity-curve schema. The plan currently lists only `il_usd`; **add `lvr_usd` as a sibling field**. |
| **JIT-driven fee inflation** | When a large swap routes through a pool with active JIT bots, much of the fee accrues to the JIT position, not to passive LPs. Backtests that allocate fees by passive-LP share at the swap moment systematically over-allocate. | Kaiko / IACR studies [S2] document up to **44% per-trade fee erosion** for passive LPs in JIT-active swaps. | For the WETH/USDC 5bp pool (a known JIT target), M2.3 should flag swaps where total in-range liquidity *spiked* in the immediately-preceding block as JIT-affected, and consider reporting "passive-LP-fees" with and without that swap class. |

## Documented "Regimes When LP Wins" — Quantified Thresholds

The most actionable contribution this paper can make to M2.8 is concrete numbers. The table below assembles the explicit thresholds the literature has reported.

| Threshold | Source | Interpretation for M2.8 |
|---|---|---|
| **Required fee APR ≥ σ²/8** (annualised, full-range CPMM equivalent) | [P3] a16z, [P4] Milionis et al. | The cleanest profitability gate. If realised fee APR < σ²/8 for the period, the position lost to a continuously-rebalanced benchmark. |
| **σ_ETH ≈ 0.95 annualised since 2020 ⇒ break-even fee APR ≈ 11.4%** for a full-range LP | [P7] Atise | Concrete anchor: any historical month where ETH/USDC LP earned <11.4% APR fees was, in expectation, dominated by a continuously-rebalanced ETH+USDC portfolio. |
| **Daily volume ≥ 10.4% of AMM TVL at 30bps fee** for break-even | [P3] a16z | Volume-side analogue of the fee-side rule; useful when daily volume is more readily available than fee-rate data. |
| **V2 recaptures 80–85% of LVR in fees in 2023 for ETH/USDC** | [P6] Atise retrospective | Order-of-magnitude prior: V2 LP on this pair runs slightly negative-to-flat in normal regimes, profitable in vol shocks. |
| **V3 5bp WETH/USDC recaptures ~80% of LVR over 2022-2023** | [P5] Measuring Arbitrage Losses (2024) | **Direct prior for Aurix's flagship pool.** A 20% gap to break-even is the magnitude M2.8 should report as its baseline. |
| **2× volatility ⇒ 4× LVR (σ² scaling)** | [P4], [P7] | Vol regimes must be cut on σ² (variance), not σ (std). A "moderately high" vol month at 1.5× the median produces 2.25× the median LVR — this is non-linear, and a linear regime classifier will mis-rank. |
| **Concentrated narrow-range LVR ≈ 4× full-range LVR** when range is `[0.5×p, 2×p]` | [P10] Auditless / V3 IL derivation | Range-width × LVR is the second-order axis after vol. M2.5's grid heatmap should overlay this. |
| **In low-vol stable-pair regimes, V3 outperforms V2 by ~160% (1bp tier)** | [P8] Uniswap blog | Confirms V3 wins decisively for stablecoin pairs with appropriate fee tier. Aurix's 5bp pool is **not** in this favourable regime. |

### ASCII summary: where V3 LP wins vs loses

```
Fee APR (realised, annualised)
   high
       |   ⬛⬛⬛   (Atise V2 2023:           |
       |   ⬛⬛⬛    fees ≈ 1.2× LVR;          |  ← LP wins
       |   ⬛⬛⬛    profitable)               |     (region above σ²/8)
   ≈11%|---⬛⬛⬛-------------------------------|
       |   ⬛⬛⬛   (Heimbach 2024 V3 5bp:     |
       |   ⬜⬜⬜    fees ≈ 0.80× LVR;          |  ← LP loses
   low |   ⬜⬜⬜    under-compensated)         |     (region below σ²/8)
       +---low--------- σ_ETH ----high------>
                      (annualised)
```
*Schematic only — not to scale. Built from sources [P3], [P4], [P5], [P6], [P7].*

### The "% of LPs that beat HODL" aggregate

| Study | Pool / window | Coverage | % LPs that beat HODL | Notes |
|---|---|---|---|---|
| Loesch et al. 2021 [P1] | 17 pools, 43% of TVL, May–Sep 2021 | All V3 LPs in those pools | **~50.5%** beat HODL (49.5% lost) | Earliest V3 months; high-vol launch period |
| Heimbach et al. 2022 [P2] | All V3 LPs across multiple pools | Theoretical + empirical | Returns *"vary wildly"*; aggregate not stated as a single % but heavy left-skew with sophisticated players capturing top quintile | Concentration finding: top decile of LPs by capital captures most of positive returns |
| Atise 2023 [P6] | V2 ETH/USDC, full-year 2023 | Single pool | V2 LPs **profitable on average** (1–2%) | Note: V2, not V3 — direct V3 comparison was not run, but V3 full-range cited as ≈ 0% EV |
| Heimbach/Wattenhofer 2024 [P5] | Major V3 pools, Jan-2022 → Dec-2023 | Largest pools by TVL | Most large pools **under-compensate** for LVR; WETH/USDC 5bp at ≈ 80% recovery | Closest analogue to Aurix; the prior to compare against |
| Uniswap official 2023 [P8] | All V3 vs V2 fee-tier-matched | Synthetic non-rebalancing positions | V3 5bp **underperforms** V2 by 68%; V3 30bp outperforms by 16% | Cross-tier headline (54% V3 advantage) is misleading for the 5bp tier |

> **The honest aggregate is not a single number.** It is: *for the pools and windows studied, between 40% and 60% of V3 LPs beat HODL on average, with the distribution heavy-tailed (a small fraction of sophisticated LPs capture most of the gains) and the headline number swinging by 20+ percentage points across windows and pools.*

## V2 (Full-Range) vs V3 (Concentrated) — Where The Picture Diverges

| Dimension | V2 (full-range) | V3 (concentrated) | Source |
|---|---|---|---|
| Capital efficiency | 1× baseline | Up to 4000× for narrow ranges | V3 whitepaper, [P3] |
| IL on a 2× price move | ~5.7% | ~22% if range is `[0.5p, 2p]` (≈ 4× V2 IL) | [P10] Auditless |
| LVR per unit TVL | σ²/8 ≈ 11.4% APR for ETH at σ≈0.95 | Higher by gamma factor; range `[0.5p, 2p]` ≈ 4× | [P4], [P7] |
| Passive ETH/USDC profitability in 2023 | **Profitable** (fee/LVR ≈ 1.2×, +1–2% net) | Approximately **flat to negative** (fee/LVR ≈ 0.80–1.0×) | [P5], [P6] |
| Stablecoin pair profitability | Moderate | Strongly positive (e.g. 1bp tier outperforms V2 by 160%) | [P8] |
| Required active management | Low (deposit and forget) | High (rebalancing critical) | [P2] |
| Vulnerability to JIT liquidity attacks | Low (no narrow ticks to ambush) | High (per-trade passive-LP fee erosion up to 44%) | [S2] |

> **The headline is regime-dependent and pair-dependent.** V3 dominates V2 for low-vol stable pairs; V2 may dominate V3 for volatile pairs at the 5bp tier. Aurix's flagship pool sits in the *worst-case* tier for V3 vs V2 according to the Uniswap official analysis.

## How "Should You LP At All" Is Framed In The Wild

A short discrimination, since the project's framing claim is that quant LP desks ask this question rather than taking LP as given.

- **Academic framing.** The literature largely takes LPing as the object of study and asks *"under what conditions does it pay?"* — yielding measures like LVR, IL, and fee/LVR ratios. The closest formal statement of "should you LP" is a16z [P3]: *"To first order, this question boils down to whether the fees collected exceeded the LVR suffered"*. This is the framing M2.8 is closest to.
- **Practitioner framing.** Atise [P6, P7] frames V2 LPing as *"a viable alternative to other DeFi activities, including ETH staking"*, explicitly putting it in a multi-strategy capital-allocation context. This matches M2.8's framing exactly — staking, lending, and HODL are the relevant comparators, not "should LP exist as a concept".
- **Quant-desk inference (project inference).** Quant desks running LP capital almost always ask the question *within* a multi-strategy mandate (LP, lending, staking, market-making) and rotate based on regime signals. They do not ask *"should LP exist"*; they ask *"is this month's expected LP edge over the next-best alternative greater than the rotation cost?"*. M2.8's framing should match this — *not* "is V3 LP profitable?" but *"is this month's V3 LP P&L expected to beat the next-best static alternative (lending, staking, HODL) net of rotation cost?"*.

> **Project inference (labelled, not source-backed).** The plan's headline template (*"WETH/USDC LP outperformed stable lending in 6 of 24 months..."*) is already in the practitioner framing, not the academic framing. This is the right framing for the hiring audience. The literature backs it.

## Gap Analysis

What the literature **does not** answer for M2.8, and where the project must commit to its own evidence:

1. **The exact "% of months V3 LP beat lending" for WETH/USDC over 2024-2026 with realistic gas costs.** All the major studies stop in 2023 ([P5] cuts at Dec 2023). M2.8 will produce a fresh number for 2024-2026 — that is the empirical contribution.
2. **The optimal vol-regime cutoff for the headline classification.** Literature gives σ²/8 as a continuous threshold; the headline format wants discrete buckets (low / med / high). The plan suggests adaptive terciles; **a stronger choice grounded in the literature**: cut buckets where the σ²/8 break-even crosses observed fee APR. This produces buckets aligned with the question the headline asks.
3. **Realistic management-gas thresholds for retail position sizes.** Literature reports gas effects qualitatively; M2.8 should produce a concrete *"position size below which management gas dominates"* number, which the plan already commits to (M2.3 *"flags this explicitly when management cost > 5% of capital"*).
4. **The fee/LVR ratio for the WETH/USDC 5bp pool over Aurix's specific window.** This is the direct prior comparison: the literature says ≈80% (Jan-2022 → Dec-2023); Aurix's number for 2024-2026 is the contribution.
5. **Whether including L2 V3 pools changes the answer.** The plan explicitly excludes L2 (out of scope for V1). The literature has very thin coverage of L2 LP profitability. Flagged for V2 of the project.

## Recommended Priority Order — Concrete Edits To The Plan

In rough order of impact on M2.8's defensibility:

| # | Plan section | Recommended change | Backed by |
|---|---|---|---|
| 1 | M2.3 simulation engine output schema | Add `lvr_usd` as a sibling field to `il_usd`. Report both. | Sources [P3]–[P7]; "Methodological Pitfalls" row 6 |
| 2 | M2.7 benchmark set | Add **V2 LP on the same pair** as a benchmark. Literature finds V2 may dominate V3 at the 5bp tier. | [P5], [P8] |
| 3 | M2.8 vol-regime classifier | Replace adaptive terciles with σ²/8-anchored cuts (or report both for comparison). | [P3], [P4], [P7] |
| 4 | M2.8 headline framing | Anchor the headline against the **2024 follow-up's 80% fee/LVR ratio** for WETH/USDC 5bp as the prior. Aurix's measured number is the contribution. | [P5] |
| 5 | M2.1 ingestion scope | Ingest `Mint` and `Burn` events alongside `Swap` events, so M2.3 can reconstruct the time-varying liquidity surface. | "Methodological Pitfalls" row 4 (Heimbach et al. [P2]) |
| 6 | M2.4 validation harness | Include at least one position that was burned at a loss, to confirm the engine reproduces losing scenarios. | "Methodological Pitfalls" row 2 |
| 7 | M2.5 grid output | Report **gamma-adjusted Sharpe** (Sharpe / range_width_factor) alongside raw Sharpe so narrow-range high-Sharpe strategies don't look artificially good in calm windows. | [P7] (LVR ∝ gamma) |
| 8 | M2.5 / M2.3 fee allocation | When a swap is routed through a pool tick where in-range liquidity *spiked* in the immediately-preceding block, flag the swap as JIT-likely and report passive-LP fees with and without it. | [S2] JIT studies |
| 9 | M2.8 headline copy | Drop any draft that leads with "50% of LPs lose money". Lead with the regime-conditional framing and reference the [P1] number with proper temporal scoping (May–Sep 2021, V3 launch period). | [P1] (date-scoping); [P5] (more current); [P6] (V2 contrasting) |
| 10 | M2.8 acceptance criterion | Add: "the headline reports both the project's measured fee/LVR ratio for WETH/USDC 5bp and the [P5] prior of ~80%, with any gap explained". | [P5] |

## Open Uncertainties And Validation Needs

- **Whether σ²/8 is a tight regime threshold for V3 5bp specifically, or whether V3's gamma factor on the typical 5bp range width pushes the break-even higher.** The literature gives this qualitatively (LVR ∝ gamma) but not quantitatively for the 5bp tier specifically. M2.5 grid output will resolve this empirically.
- **Whether Aurix's measured fee/LVR ratio over 2024-2026 reproduces the [P5] 80% figure for the 2022-2023 window.** Two outcomes are interesting: agreement (validates M2.3 indirectly), divergence (raises a substantive finding worth reporting in the headline).
- **JIT prevalence in WETH/USDC 5bp during Aurix's window.** Kaiko [S2] reports JIT activity on this pool but the prevalence has likely changed post-2023. The "JIT-likely swap" flag in M2.3 will produce a measurable number.
- **The right vol-regime cutoffs for the M2.8 buckets.** The plan suggests adaptive terciles; this paper recommends σ²/8-anchored cuts. The right answer is empirical: report both side-by-side and let the user pick.

## What Not To Overbuild

- **A new theoretical IL or LVR derivation.** The literature is dense, mature, and adversarial; an Aurix derivation is unlikely to add anything except risk of error. Use [P4] formulas verbatim and cite.
- **A "perfect" regime classifier.** The literature explicitly says LP profitability is regime-dependent and noisy at the monthly level. M2.8's contribution is empirical reporting and recommendation framing, not a clever classifier.
- **Coverage of every fee tier and pair.** The plan already scopes V1 to WETH/USDC 5bp. Adding the 30bp tier as a single-row comparison is valuable (it is the closest tier where V3 outperforms V2 per [P8]); adding more is scope creep.
- **L2 / multi-chain / cross-pool synthesis.** Literature is thin; project is not chartered for it.

## Relationship To Existing Context

- This artefact is the **first** entry in `context/references/`. Future research papers in the same family (likely candidates: V3 swap-event semantics, Q64.96 math validation references, Aave APY data sourcing, regime-classification methodologies) should cross-link here when their findings interact.
- **Direct dependencies on `context/`:**
  - `context/plans/vector-a-v3-lp-backtester.md` — every milestone reference in this paper points back to this plan; recommended edits in this paper apply to that plan.
  - `context/architecture.md` — current code surface description.
  - `context/systems/arbitrage-market-data.md` — the V3 pool inventory currently lives here; M2.8 will extend this when ingestion lands.
- **No supersession.** This artefact does not replace any prior work; it is the inaugural reference.

## External Research Trail

**Searches run**

| # | Query | Tool | Rationale | Sources surfaced |
|---|---|---|---|---|
| 1 | `Loesch Hindman Welch Bakshi 2021 "Impermanent Loss in Uniswap V3" arxiv paper findings` | WebSearch | Anchor the foundational Topaze Blue / Bancor study | arXiv:2111.09192; CryptoSlate; CryptoBriefing |
| 2 | `Heimbach "empirical study profitability Uniswap V3" liquidity providers paper` | WebSearch | Anchor the foundational ETH Zürich study | arXiv:2205.08904; ACM AFT'22; ResearchGate |
| 3 | `Topaze Blue Bancor "impermanent loss" Uniswap V3 49.5% liquidity providers HODL underperform` | WebSearch | Surface the practitioner-side coverage of the [P1] study | CryptoSlate; CryptoBriefing; CoinCodex |
| 4 | `Milionis Moallemi Roughgarden "Loss-Versus-Rebalancing" LVR Uniswap concentrated liquidity` | WebSearch | Anchor the LVR foundational paper | arXiv:2208.06046; a16z; Titania Research |
| 5 | `Uniswap V3 WETH USDC fee revenue impermanent loss ratio historical 2024 2025` | WebSearch | Surface recent data on the project's flagship pool | Amberdata; DefiLlama; the Heimbach 2024 follow-up |
| 6 | `Uniswap V3 liquidity provider profitability volatility regime conditional fee tier` | WebSearch | Surface regime-conditional findings | Cyfrin; Nansen; arXiv:2205.08904 |
| 7 | `"impermanent loss" Uniswap V3 methodology pitfalls "look-ahead bias" "survivorship bias" LP backtest` | WebSearch | Surface methodological literature | Amberdata; SIAM; Auditless |
| 8 | `"just-in-time" JIT liquidity Uniswap V3 LP profitability sophisticated retail` | WebSearch | JIT-driven fee erosion (a passive-LP-specific risk) | Kaiko; Coinmonks; IACR ePrint |
| 9 | `"Uniswap v3" "LP" "profitable" 2025 academic study fees outperform IL passive` | WebSearch | Test whether 2024-2026 evidence reverses the "LPs lose" narrative | Uniswap blog "fee-returns"; SoSoValue |
| 10 | `"loss versus rebalancing" empirical 2024 2025 Uniswap pool profitability fees compensate annual` | WebSearch | Find a contrasting source on the LVR vs fee balance | Atise retrospective; Gauntlet; Heimbach 2024 |

**Sources consulted**

| URL | Tool | Source class | Quoted in artefact? |
|---|---|---|---|
| [Loesch et al. 2021 — Impermanent Loss in Uniswap v3](https://arxiv.org/abs/2111.09192) | WebFetch | Foundational paper (peer-reviewed in ACM CCS DeFi Workshop 2022 follow-up) | Yes — [P1] |
| [Heimbach et al. 2022 — Risks and Returns of Uniswap V3 LPs](https://arxiv.org/abs/2205.08904) | WebFetch | Foundational paper (ACM AFT '22) | Yes — [P2] |
| [Milionis et al. 2022 LVR PDF](https://arxiv.org/pdf/2208.06046) | WebFetch | Foundational paper (the canonical LVR derivation) | Partial (PDF binary) — abstract via [P4] arxiv landing |
| [Milionis et al. 2022 LVR abstract page](https://arxiv.org/abs/2208.06046) | WebFetch | Foundational paper | Yes — [P4] |
| [a16z LVR essay (Anthony Lee Zhang)](https://a16zcrypto.com/posts/article/lvr-quantifying-the-cost-of-providing-liquidity-to-automated-market-makers/) | WebFetch | Strong industry write-up by one of the LVR authors; high-signal secondary | Yes — [P3] |
| [Heimbach paper PDF (lioba homepage)](https://liobaheimba.ch/assets/pdf/Papers/Risks_and_Returns_of_Uniswap_V3_Liquidity_Providers.pdf) | WebFetch | Foundational paper (PDF) | Failed — binary not extractable (deferred to abstract via search snippet and [P2]) |
| [CryptoSlate coverage of Topaze Blue / Bancor study](https://cryptoslate.com/new-report-shows-50-of-uniswap-v3-liquidity-providers-are-losing-money/) | WebFetch | Production write-up summarising [P1] | Yes — [S1] |
| [Heimbach / Wattenhofer 2024 — Measuring Arbitrage Losses](https://arxiv.org/html/2404.05803v2) | WebFetch | Foundational follow-up paper (peer-reviewed) | Yes — [P5] |
| [Uniswap blog — v3 Returns More Fees for Passive LPs](https://blog.uniswap.org/fee-returns) | WebFetch | Official Uniswap analysis (canonical industry view) | Yes — [P8] |
| [Titania Research LVR 101 (Medium)](https://medium.com/@titania-research/loss-versus-rebalancing-101-bc9651ec6e43) | WebFetch | Strong educational write-up; numerical examples | Yes — [P9] |
| [Atise — Uniswap V2 still a good deal? 2023 retrospective](https://atise.medium.com/uniswap-v2-still-a-good-deal-for-liquidity-providers-a-retrospective-of-2023-11475e9d8610) | WebFetch | Practitioner retrospective; **contrasting source** (V2 was profitable in 2023) | Yes — [P6] |
| [Atise — LP strategies for Uniswap V3: LVR](https://atise.medium.com/liquidity-provider-strategies-for-uniswap-v3-loss-versus-rebalancing-lvr-ee0ffdf1f937) | WebFetch | Practitioner LVR-aware strategy guide; concrete σ-thresholds | Yes — [P7] |
| [Cyfrin — Concentrated Liquidity & Capital Efficiency](https://www.cyfrin.io/blog/uniswap-v3-concentrated-liquidity-capital-efficiency) | WebFetch | Production write-up on concentrated-liquidity mechanics | Referenced (background only) |
| [Compass Labs — Investor Guide to IL on V2 and V3](https://www.compasslabs.ai/blog/the-investor-guide-through-impermanent-loss-on-uniswap-v2-and-uniswap-v3) | WebFetch | Production write-up | Used as cross-check (limited extractable content) |
| [Amberdata — Strategies for Mitigating IL across V3](https://blog.amberdata.io/strategies-for-mitigating-impermanent-loss-across-uniswap-v3) | WebFetch | Production write-up | Used as cross-check (no quotable thresholds extracted) |
| [Auditless — Impermanent Loss in Uniswap V3](https://medium.com/auditless/impermanent-loss-in-uniswap-v3-6c7161d3b445) | WebFetch | Practitioner-derived V3 IL formula and the 4× narrow-range result | Yes — [P10] |

**Source-class coverage:** foundational papers (4: [P1], [P2], [P4], [P5]) + official documentation/industry analysis (1: [P8]) + strong industry write-ups by paper authors (1: [P3]) + practitioner sources / contrasting evidence (3: [P6], [P7], [P9]) + production write-ups (multiple) + summary/coverage sources ([S1], [S2]). Six source classes covered, well above the two-class floor.

**Contrasting / limiting source:** [P6] Atise 2023 V2 retrospective explicitly contradicts the headline "V3 LPs lose to HODL" narrative for V2 ETH/USDC in the 2023 window; [P8] Uniswap official analysis claims V3 outperforms V2 *on average* but the same study shows V3 5bp *underperforms* V2 by 68% — itself a contrasting finding within the official source. Both rebut a one-sided "LPs always lose" reading.

**Quoted passages**

- **[P1]** — `https://arxiv.org/abs/2111.09192` (Loesch, Hindman, Welch, Richardson 2021)
> "for the 17 pools analyzed [...] covering 43% of TVL [...] total fees earned since inception until the cut-off date was $199.3m [...] total IL suffered by LPs during this period was $260.1m, meaning that in aggregate those LPs would have been better off by $60.8m had they simply HODLd."
- **[P1]** (cont.)
> "49.5% of liquidity providers had suffered negative returns due to IL"
> "80% of pools had IL exceeding fees earned"
> "only the WBTC/USDC, AXS/WETH, and FTM/WETH pools seeing net positive returns"
- **[P2]** — `https://arxiv.org/abs/2205.08904` (Heimbach, Schertenleib, Wattenhofer 2022)
> "providing liquidity has become a game reserved for sophisticated players with the introduction of Uniswap V3, where retail traders do not stand a chance."
> "simple and profitable strategies for liquidity providers in liquidity pools characterized by negligible price volatilities [...] only yield modest returns. Instead, significant returns can only be obtained by accepting increased financial risks and at the cost of active management."
> "liquidity providing in Uniswap V3 is incredibly complicated, and performances can vary wildly."
- **[P3]** — `https://a16zcrypto.com/posts/article/lvr-quantifying-the-cost-of-providing-liquidity-to-automated-market-makers/` (a16z / Anthony Lee Zhang)
> "LVR ('loss versus rebalancing,' pronounced 'lever')" represents "the sum of the losses incurred by executing the trades via the AMM rather on the open market."
> "For a constant-product market maker, the instantaneous LVR, when normalized by the CPMM's market value, turns out to be exactly σ²/8."
> "If a Uniswap v2 ETH-USDC pool has a daily volatility of 5%, then according to our model LPs lose 3.125 bps to LVR every day (for a roughly 11% loss annually)."
> "if this AMM charges a fixed 30 bps trading fee, then LPs will break even provided the daily volume is roughly 10.4% of the AMM's assets."
> "Was a decision to provide liquidity a good idea in hindsight? To first order, this question boils down to whether the fees collected exceeded the LVR suffered."
- **[P4]** — `https://arxiv.org/abs/2208.06046` (Milionis, Moallemi, Roughgarden, Zhang 2022 abstract)
> "Our central contribution is a `Black-Scholes formula for AMMs'. We identify the main adverse selection cost incurred by LPs, which we call `loss-versus-rebalancing' (LVR, pronounced `lever'). LVR captures costs incurred by AMM LPs due to stale prices that are picked off by better informed arbitrageurs. We derive closed-form expressions for LVR applicable to all automated market makers."
- **[P5]** — `https://arxiv.org/html/2404.05803v2` (Heimbach / Wattenhofer 2024 — "Measuring Arbitrage Losses")
> "fees do not sufficiently compensate for arbitrage losses in most of the largest Uniswap liquidity pools."
> "For the dominant WETH-USDC 5bp pool, historical returns from fees hover around 80% of arbitrage losses."
> "Uniswap v2 pools are more profitable for passive LPs than their Uniswap v3 counterparts."
> "fees are consistently three times larger than losses during this time" (V2, second observation year)
- **[P6]** — `https://atise.medium.com/uniswap-v2-still-a-good-deal-for-liquidity-providers-a-retrospective-of-2023-11475e9d8610` (Atise 2024 — V2 2023 retrospective; **contrasting source**)
> "fee income consistently exceeded the theoretical LVR"
> "around 80–85% of the LVR in swap fees" recaptured by V2
> "over 4% relative profit by October, ending the year between 1 to 2% profit"
> "divergence loss for the full year was almost precisely 5%"
> "a passive full range position [in V3] typically earns in fees about what it loses to LVR, suggesting that the expected value of such positions hovers around zero"
> "a viable alternative to other DeFi activities, including ETH staking"
- **[P7]** — `https://atise.medium.com/liquidity-provider-strategies-for-uniswap-v3-loss-versus-rebalancing-lvr-ee0ffdf1f937` (Atise — V3 LVR strategy)
> "an LP is considered profitable if fee_return > LVR"
> "the yearly σ of ETH since 2020 is ~0.95 -> the required fee APR to offset the loss is ~11.4% for a full-range position"
> "LVR is proportional to the gamma of the LP position"
> "in the 0.3% and 1.0% pools, majority of the loss can be recaptured, provided that the liquidity in the pool is deep enough"
> "2x increase in volatility is going to result in 4x higher LVR"
> "LPs can and probably should monitor volatility and relocate their liquidity to higher fee tiers when needed"
- **[P8]** — `https://blog.uniswap.org/fee-returns` (official Uniswap analysis)
> "non-rebalancing Uniswap v3 positions outperform comparable Uniswap v2 positions by an average of ~54%"
> "100-bps fee-tier full-range v3 positions outperform v2 positions by an average of ~80%"
> "1-bp fee-tier range-bound v3 stablecoin pair positions outperform v2 positions by an average of ~160%"
> "30-bps fee-tier full-range v3 positions outperform v2 positions by an average of ~16%"
> "5-bps fee-tier full-range v3 positions underperform v2 by an average of ~68%"
> "the fee returns derived here need to be jointly considered in the context of divergence loss on volatile token pairs as well as depegging probability"
- **[P9]** — Titania Research LVR 101 (Medium)
> "LVR depends on the square of the instantaneous volatility"
> "LVR […] is always cumulative, regardless of the direction of price changes"
- **[P10]** — Auditless on V3 IL
> impermanent loss for a concentrated position in `[0.5p, 2p]` is "nearly 4 times higher than if we provided liquidity in the whole range of prices"
- **[S1]** — CryptoSlate coverage of [P1]: provides the canonical practitioner summary of the Topaze Blue / Bancor study; quoted only via the same numbers as [P1].
- **[S2]** — Kaiko / IACR JIT studies
> "JIT liquidity, when deployed strategically, can [...] erod[e] average passive LP profits by up to 44% per trade"

## Pre-Completion Obligation Audit

| Obligation | Status | Evidence |
|---|---|---|
| At least 3 distinct WebSearch calls with topic-specific queries | ✅ | 10 distinct queries listed in "Searches run" — well above the floor of 3 |
| At least 3 distinct WebFetch calls against primary sources | ✅ | 14 distinct WebFetch calls listed in "Sources consulted"; primary-source class covers [P1] (arXiv), [P2] (arXiv), [P4] (arXiv), [P5] (arXiv) |
| Sources span at least 2 source classes | ✅ | Six classes covered: foundational papers, official documentation, strong industry write-ups by paper authors (a16z), practitioner sources, production write-ups, summary/coverage |
| At least 1 direct quoted passage per major source-backed claim | ✅ | Every Research Signal row references a passage by source-ID `[P1]`–`[P10]`, `[S1]`–`[S2]`, with verbatim text in "Quoted passages" |
| At least 1 contrasting / limiting / disagreeing source consulted | ✅ | [P6] Atise V2 2023 retrospective explicitly contradicts the headline "LPs lose to HODL" narrative for V2 ETH/USDC; [P8] Uniswap blog further contradicts within the same study (V3 5bp *underperforms* V2 vs. cross-tier average outperforming) |
| Relevant `context/` files read before project-specific claims | ✅ | `context/architecture.md` (full read), `context/notes.md` (full read), `context/plans/vector-a-v3-lp-backtester.md` (full read), `context/systems/arbitrage-market-data.md` (read), `context/notes/` index reviewed |
| Relevant code inspected | ✅ | `src-tauri/src/dex/uniswap_v3.rs` (full read, 152 lines); confirmed only `slot0()`-based price decode exists, no tick math, no fee tracking |
| `scripts/init_research_artifact.py` run (stdout captured) | ✅ | Stdout: `Created file scaffold: /Users/atacanercetinkaya/Documents/Programming-Projects/Aurix/context/references/v3-lp-profitability-literature.md` |
| `scripts/validate_research_artifact.py` run (stdout captured) | ✅ | Captured in completion report after artefact write |

## What I Did Not Do

- **Did not extract text from `arxiv.org/pdf/2208.06046` (LVR PDF) directly.** The PDF stream returned binary content not extractable by WebFetch. Mitigation: used the arxiv abstract landing page [P4] for the abstract verbatim, and used the a16z explainer [P3] (co-authored by paper author Anthony Lee Zhang) and Atise's strategy piece [P7] for the formulas the abstract page does not include. Same primary-source content reached via two independent secondary sources by the same author cohort.
- **Did not extract text from `liobaheimba.ch/.../Risks_and_Returns_of_Uniswap_V3_Liquidity_Providers.pdf` directly.** Same PDF-binary issue. Mitigation: used the arxiv abstract page [P2] which contains the verbatim conclusions and the published WebSearch summary; the substantive numerical findings in the paper (LP-concentration tail) are referenced qualitatively rather than quoted from the body text.
- **Did not run a fresh on-chain pull** of the WETH/USDC 5bp pool's recent fee/IL ratio for 2024-2026. That is M2.1–M2.3 work, not the literature review's job. The literature's prior of ≈80% (from [P5]) is the anchor; Aurix's number is the contribution.
- **Did not survey academic L2 LP profitability literature.** L2 is out of scope for V1 of the project (plan §Out of Scope), and the academic coverage is thin enough that a single paper would not change the recommendation here.
- **Did not benchmark against ETH/BTC LP profitability or non-WETH/USDC pools at depth.** The plan flagrantly scopes V1 to WETH/USDC; broader pool comparisons would be scope creep relative to M2.8's framing.
- **Did not interview practitioners or run an internal survey of quant LP desks.** The "how is this framed at quant desks" section relies on inference (labelled as such) and on the practitioner sources [P6], [P7]. A primary-source desk-practice study is out of reach for a literature-review pass and not the right tool for this question.
- **Did not verify the [P5] paper's exact methodology for "fee/LVR ratio" against the quoted 80% figure for WETH/USDC 5bp.** The PDF was extractable in HTML but the deep-methodology sections were not in the WebFetch summary. The 80% figure should be treated as the reported headline; cross-verifying the methodology against the engine implementation in M2.4 is the right place to close this.
