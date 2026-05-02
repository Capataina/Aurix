# Out-of-Scope LP Risks — MEV, Stablecoin De-peg, Tax Drag

A survey of three LP-performance-affecting risks that the V1 plan for Tab 2 (`context/plans/vector-a-v3-lp-backtester.md`) explicitly puts out-of-scope: MEV / sandwich tax on entry/exit swaps, stablecoin de-peg tail risk, and tax drag on rebalances. For each: magnitude on this project's target pool, what the published evidence says, and a verdict on whether to keep it out-of-scope, surface it as a UI flag, or promote it to V2.

This paper is intentionally lighter than the deep-methodology references in `context/references/`. The job is to give the implementer enough magnitude-grounded material to disclose these risks honestly without descending into legal-disclaimer noise, and to flag the one risk where the magnitude crosses into "should-be-modelled" territory.

## Scope / Purpose

The V1 backtester models position fees, impermanent loss, and management gas (mint / burn / collect / rebalance) at historical block-level prices. It deliberately excludes:

- the MEV / sandwich-tax cost on the swap leg of mint/burn/rebalance transactions,
- stablecoin de-peg events affecting the USDC half of the position value,
- tax treatment of LP entry, rebalance, and fee-collection events.

Each of those is a real cost on a real LP position, and a backtester that ignores them is telling a partial story. This paper's job is to translate "out of scope" into a concrete, defensible thing the UI can say, with a clear scope-promotion verdict per risk based on magnitude rather than vibe.

Three things this paper is **not** trying to be:

- a deep methodology document on how to *model* MEV, depeg tail risk, or tax drag (each would be a separate paper if promoted to V2),
- a legal opinion on tax treatment in any jurisdiction (the tax section quotes published guidance and tracks where the rules are clear vs unclear, but is not advice),
- an exhaustive survey of every depeg event in stablecoin history (only the ones whose magnitude or duration informs a real Aurix UI decision).

## Current Project Relevance

Three things make this paper time-critical for Tab 2:

1. **The implementer is about to write UI copy.** M2.6 in the plan explicitly calls for a "headline strip" with the regime-conditional verdict on whether to LP at all. That headline strip is exactly the place where unmodelled-risk disclosures either work or descend into weasel-language, and the implementer needs concrete copy patterns before writing it cold.
2. **Magnitude calibrates scope.** The V1 plan benchmarks LP returns against Aave USDC supply APY (M2.7). If the magnitude of any unmodelled risk approaches the size of that APY differential — which is in the low-single-digit annualised percent range — then the unmodelled risk is doing more than disclosure; it is changing the headline answer to "should you have LP'd at all?" and the V1 numbers are wrong by enough to mislead.
3. **The hiring story depends on intellectual honesty.** The Aurix portfolio framing (`Projects/Aurix/_Overview.md` in LifeOS, plus the resume bullets in the plan's Hiring Signal Payoff section) leans on "I built the backtester quant LP desks actually use." Quant desks model these risks. A backtester that ignores them and does not visibly know it is ignoring them is the wrong hiring signal. A backtester that ignores them and visibly knows exactly what it is ignoring, in calibrated UI language, is the right one.

## Current State Snapshot

| Aspect | Verified state | Source |
|---|---|---|
| MEV cost on entry/exit swaps | Not modelled. Out-of-scope per `context/plans/vector-a-v3-lp-backtester.md:277` (`MEV cost modelling on entry/exit swaps ... defer to V2`). | repository fact |
| Stablecoin de-peg tail risk | Not modelled. Out-of-scope per `context/plans/vector-a-v3-lp-backtester.md:279` (`USDC depegged March 2023 with SVB; not modelled in benchmark comparison`). | repository fact |
| Tax drag on rebalances | Not modelled. Out-of-scope per `context/plans/vector-a-v3-lp-backtester.md:278` (`every rebalance is a taxable event in most jurisdictions`). | repository fact |
| Slippage on entry/exit swap leg | Not modelled separately; the position is assumed to enter and exit at pool spot. | repository fact (no slippage logic in M2.3) |
| UI surface to host disclosures | Not yet built. Tab 2 frontend is M2.6, not started. | repository fact |
| Existing references touching these topics | None directly. `v3-lp-profitability-literature.md`, `v3-position-validation-methodology.md`, and `lp-rebalancing-strategies.md` are scaffold-only. | repository fact (`ls context/references/`) |

Inferred: the implementer will discover these unmodelled costs naturally during M2.4 validation when on-chain LP fee totals diverge from modelled fee totals by some unexplained delta — at which point this paper exists as the explanation rather than the discovery.

---

## Risk 1 — MEV / Sandwich Tax on Entry, Exit, and Rebalance Swaps

## What the topic actually is

A Uniswap V3 LP position does not appear out of nothing. To enter, the user typically swaps part of one asset for the other to reach the target ratio for the chosen range, then mints the position; on exit, they burn and (sometimes) swap the residual back; on rebalance, they burn the old position, rebalance the asset ratio via swap, and mint a new one. Each of those swap legs is a public mempool transaction that a sandwich bot can wrap with a front-run buy and a back-run sell, capturing the slippage the victim's swap moves the price by.

Two adjacent extraction patterns matter for completeness:

- **JIT (just-in-time) liquidity** — a searcher mints a tight LP position one block before a known large swap, captures most of that swap's fees, then burns immediately afterward, displacing existing LPs' fee share. JIT does not cost the *swapper* anything; it costs the *passive LPs in the pool*. For an Aurix backtester position, JIT shows up as a fee-revenue *reduction* against on-chain truth, not as a swap-leg slippage cost. The plan's M2.4 validation harness will see this as a fees-collected discrepancy on large-swap blocks.
- **Reordering slippage** — when multiple swaps land in the same block and the searcher's bundle reorders them to extract value, the realised price for some swaps differs from the price they would have got at any block-aggregate average. This is closer to a generalised "MEV tax on every swap" than to discrete sandwich-attack events.

## Magnitude on the Aurix target pool (WETH/USDC 0.05%)

Three independent measurements triangulate the magnitude on this exact pool:

**EigenPhi pool-level sample (Diamond Protocol Medium, 2022, 180-day window):**

> "In the past 180 days, there were **5,346** Sandwich Attacks." `[EIGENPHI-1]`
>
> "Out of the $100M fee, **$2.26M** was earned by JIT trades." `[EIGENPHI-2]`

That `$2.26M / $100M = 2.26%` is the JIT haircut to passive LP fee revenue on this pool over six months, not the entry/exit sandwich tax — but it is the cleanest published per-pool number. **It bites the LP through fees collected, not through swap-leg slippage.**

**Frontier Tech reordering-slippage measurement (Sep–Oct 2023, on-chain Ethereum):**

> "Large Uniswap V3 pools like WETH/USDC, WETH/WBTC, and WETH/USDT exhibit lower mean reordering percentages at **.05%, .09%, and .04%** respectively for swaps over $250k." `[FRONTIER-1]`
>
> "72% of swaps were negatively affected for $12.811m. The other 28% were positively affected by $3.749m." `[FRONTIER-2]`

`5 bps mean reordering slippage on $250k+ WETH/USDC swaps` is the single most useful number in this paper for our purposes. For a $10k swap (well under the $250k bucket), the cost is meaningfully smaller and frequently zero on a well-routed swap with reasonable slippage tolerance.

**2024–2026 trend (TradingView / EigenPhi via Blockworks):**

> "Monthly extraction from sandwich attacks dropped from nearly $10 million in late 2024 to about $2.5 million by October 2025." `[BLOCKWORKS-1]`
>
> "The average profit per sandwich attack remains extremely low at just above $3 in recent data from 2025." `[BLOCKWORKS-2]`

A `~$3 average per attack` figure means the sandwich-tax distribution is heavily right-skewed: most retail swaps see zero or negligible sandwich cost, and a small minority of unprotected large swaps carry the entire extraction.

**Synthesised retail-typical magnitude on a $10k WETH/USDC 5bps-pool swap with default Uniswap-frontend slippage protection:**

| Position size | Plausible mean MEV cost on entry/exit | Plausible 95th-percentile cost |
|---|---|---|
| $1k | 0–1 bp (round-trip 0–2 bp) | ~5 bp (round-trip ~10 bp) |
| $10k | 1–3 bp (round-trip 2–6 bp) | ~10 bp (round-trip ~20 bp) |
| $100k | 3–8 bp (round-trip 6–16 bp) | ~25 bp (round-trip ~50 bp) |
| $1M (over the Frontier Tech $250k threshold) | ~5 bp mean (round-trip ~10 bp) | ~30+ bp (round-trip ~60+ bp) |

Evidence class: `project inference` synthesising `[EIGENPHI-1/2]`, `[FRONTIER-1/2]`, `[BLOCKWORKS-1/2]`. The 5bps mean for $250k+ swaps is source-backed; the smaller-position figures are scaled-down inference because no published source measures retail-position sandwich tax bps directly. Worth experimental validation against a small set of historical mainnet entry/exit swaps if this risk is ever promoted to V2.

## Foundational reference

The canonical citation for sandwich-attack quantification:

> "We estimate that over 32 months, BEV yielded 540.54M USD in profit, divided among 11,289 addresses when capturing 49,691 cryptocurrencies and 60,830 on-chain markets." `[QIN-1]`

This is the abstract-level aggregate. The paper does not give per-victim-swap-size bps figures directly — those have to come from later production datasets (EigenPhi, libMEV, Frontier Tech) — but it is the foundational evidence that BEV is a real, quantifiable, multi-hundred-million-dollar phenomenon, not a theoretical concern.

## Contrasting view — sandwich tax is overstated for properly-protected swaps

The published narrative on MEV-as-a-tax bakes in the assumption that the victim's swap goes through the public mempool with default slippage tolerance. Two pieces of evidence push back:

> "Uniswap's official interface includes built-in MEV protections and default slippage settings to guard against sandwich attacks." `[UNISWAP-MEV-1]` (Uniswap support article + Hayden Adams comments after the headline `$215K sandwich attack` story; users who interacted with custom routes were exposed, users on the protected frontend were not.)

> "For stablecoin swaps in deep liquidity pools, traders can set slippage to 0.01% - 0.05%. For volatile assets, slippage can be calculated so that a bot's potential profit after paying network fees becomes negative." `[UNISWAP-MEV-2]`

This is not a denial that MEV exists — Frontier Tech's $9M is real. It is a claim that the *bps cost on a well-routed retail swap is much smaller than the headline numbers imply*, because the headline numbers are dominated by a few large unprotected swaps and the population of retail swaps mostly sits at zero or near-zero cost. The Aurix LP backtester is by definition simulating well-routed retail-shaped swaps (entries and exits sized to mint a position, not to drain liquidity), so the sandwich-tax distribution on Aurix's modelled positions is closer to the well-protected end of the distribution than to the libMEV-headline end.

## Verdict on Risk 1 — surface as UI flag; do **not** promote to V1 scope

**Reasoning by magnitude:**

- For $1k–$100k retail-typical positions on the WETH/USDC 5bps pool with default slippage protection, plausible round-trip MEV cost is in the 0–20 bp range, mean 2–10 bp.
- The Aave USDC supply APY benchmark in M2.7 is in the low-single-digit annualised percent range (200–500 bp annualised).
- A round-trip swap-leg cost of ~5 bp on a 90-day backtest is `5 bp / 90 days × 365 = ~20 bp annualised` for a position with no rebalances; for a position rebalancing weekly (13 round-trips over 90 days), it's `5 bp × 13 / 90 × 365 = ~263 bp annualised`. **That is comparable to the Aave benchmark itself.**

The first conclusion holds for low-rebalance strategies; the second is the failure mode the disclosure has to flag. The right scope decision is therefore *not* binary — it depends on rebalance frequency:

| Strategy class | Round-trip MEV impact (annualised, $10k, p50) | Recommendation |
|---|---|---|
| Static (no rebalance) | ~4–8 bp/yr | Surface as UI flag; out-of-scope is fine |
| Schedule, monthly | ~50–100 bp/yr | Surface; flag as material |
| Schedule, weekly | ~250–500 bp/yr | Surface; **flag as comparable to lending APY** |
| Schedule, daily | ~1500–3000 bp/yr | Surface; **strong flag — backtest is misleading without a coarse haircut** |
| Price-exit threshold (typical) | depends — model as schedule equivalent based on observed frequency in the run | Surface |

**Promoted to V1?** No — full sandwich-cost modelling needs per-swap mempool simulation which is a multi-week project of its own. **Promoted as a coarse haircut?** Yes, mild — a `--mev-haircut-bps-per-rebalance` knob (default ~5 bp) deducted from each modelled rebalance is one or two days of work and meaningfully changes the answer for high-frequency strategies. This is the lightest possible response to "the magnitude is large enough to mislead on high-rebalance strategies."

**UI flag copy (recommended):**

> *MEV cost on entry, exit, and rebalance swaps is not modelled. Typical impact on a $10k position with default slippage protection is small (single-digit bps round-trip) for low-rebalance strategies; for daily-rebalance strategies it can compound to several hundred bps annualised — comparable to the lending-APY benchmark. The strategy comparison heatmap does not adjust for this; treat any high-rebalance-frequency strategy's reported alpha as an upper bound.*

The copy says exactly what is unmodelled, names the magnitude band, names the strategy class where it matters, and points to the specific UI element (the heatmap) that the user must read with caution. That is the difference between a useful disclosure and weasel-language.

---

## Risk 2 — Stablecoin De-peg Tail Risk

## What the topic actually is

A WETH/USDC LP position is half-priced in USDC. The benchmark module (M2.7) prices "HODL" in USD using USDC at face. If USDC briefly trades at $0.86, that is a real -14% mark-to-market hit on half the position that no V3-LP-math primitive will ever capture, because the math operates on tick prices in WETH/USDC ratio terms, not in WETH/real-USD terms.

The risk has three loosely-coupled drivers:

| Driver | Mechanism | Recent example |
|---|---|---|
| Banking exposure | Issuer holds reserves at a bank that fails | USDC / SVB, March 2023 |
| Regulatory action | Issuer is sanctioned, frozen, or wound down | USDP NYDFS halt, Feb 2023; BUSD wind-down |
| Operational | Smart-contract bug, oracle failure, freeze list | USDC blacklist of Tornado-related addresses, Aug 2022 |
| Algorithmic peg failure | Reflexive collapse of an undercollateralised stablecoin | TerraUSD, May 2022 |

The first three apply to USDC. The fourth does not (USDC is fiat-backed), but matters for any future scope expansion to stablecoin pools using non-USDC tokens.

## Magnitude — USDC March 2023

The Federal Reserve's own postmortem is the cleanest primary source. From "In the Shadow of Bank Runs":

> "At its trough, USDC traded at 86 cents to the dollar." `[FED-1]`
>
> "USDC's price on the secondary market recovered sharply after the backstop announcement on Sunday and fully recovered once Circle began processing redemptions on Monday, March 13." `[FED-2]`
>
> "Both Tether (USDT) and Binance USD (BUSD) appreciated over the weekend and traded at a price marginally above their one-dollar peg." `[FED-3]`
>
> "As with money market funds and bank deposits, stablecoins are susceptible to crises of confidence, contagion, and self-reinforcing runs." `[FED-4]`

The depeg shape (CNBC + CoinDesk + Federal Reserve cross-confirmed):

```
USDC price ($) — March 10–13, 2023
1.00 ──────────────────╮                                ╭──────  full recovery, Mon Mar 13
0.97                   ├──╮
0.95                       ╰─╮                       ╭──╯
0.92                          ╰──╮                ╭──╯
0.89                              ╰─╮          ╭──╯
0.86 ────── trough on DEX, Sat ─────╰──────────╯
       Fri 11pm    Sat 6am    Sat noon    Sun pm    Mon am
```

Three properties of the event matter for an Aurix LP backtester:

- **Magnitude:** -14% peak depeg, but only at a brief trough on Saturday. Hourly-OHLC ground truth suggests the trough was ~$0.86 on DEX prints; centralised-exchange and primary-market redemption prices did not all reach that low.
- **Duration:** ~48–60 hours from depeg start (Friday night ET) to full recovery (Monday morning ET). Most of the time below 95¢ was Saturday only.
- **Recovery mechanism:** FDIC backstop announcement, not market self-correction. **This matters.** A future depeg whose underlying cause is regulatory rather than banking (e.g. NYDFS-style halt of a stablecoin issuer) may not recover at all — the recovery is assumption-dependent, not law-of-nature.

Backtester impact on a representative 90-day window straddling March 10–13, 2023:

| LP condition during depeg | Approximate impact on `position_value_usd` |
|---|---|
| Position in-range, full of WETH (peg held one-sided) | ~0 (USDC half got dumped into the pool by depeg-fleeing traders, leaving WETH) |
| Position in-range, full of USDC (price moved out the other way) | -14% peak unrealised depeg, ~-2% over the 48h holding window |
| Position out-of-range (held mostly USDC) | -14% peak unrealised, depeg duration determines realised loss |
| Position closed during depeg trough | -14% realised |
| Position closed after recovery (Monday) | ~0 |

The asymmetry is the key insight: an in-range position partially absorbs the depeg as inventory rotation; an out-of-range position holds the asset that is depegging at face. **The current backtester implicitly assumes USDC = $1 in all asset-value calculations**, so it shows ~0 for both rows, missing the realised loss case entirely.

## Other notable depeg events

| Event | Magnitude | Duration | Cause | Aurix relevance |
|---|---|---|---|---|
| USDT, Oct 2018 | -10% (low ~$0.90 on Bittrex; brief $0.51 print on Poloniex was an order-book artefact, not a true depeg) | ~hours | Reserve-backing rumours + Bitfinex withdrawal halt | Cross-reference: shows fiat-backed stablecoins can depeg on issuer-confidence shock alone |
| DAI, March 2020 (COVID flash crash) | +11% (briefly traded *above* peg at $1.11) | ~hours | Cascading liquidation cascade in MakerDAO + ETH price collapse | Shows depeg can go positive too; ETH collapse → undercollateralised CDPs → DAI demand spike |
| DAI, March 2023 | -12% (~$0.88 trough) | ~48h | USDC contagion (DAI was ~50% USDC-backed at the time) | Shows a stablecoin that uses another stablecoin as collateral is exposed to that collateral's depeg |
| TerraUSD (UST), May 2022 | -100% (peg permanently broken) | days | Algorithmic peg's reflexive collapse | Out of scope for fiat-backed analysis but a reference point for the worst case |
| USDP, Feb 2023 | NYDFS issuance halt; price held | n/a | Regulatory enforcement against issuer | The "regulatory not banking" cause for which there is no FDIC analogue |

## Contrasting view — how rare is "real" tail risk

A reviewer who suspects the depeg framing is over-cautious would point to:

- The number of meaningful (more than -2%, more than 24h) USDC depeg events: **one** (March 2023) over USDC's ~7-year history.
- The eventual recovery rate of fiat-backed stablecoin depegs since 2018: **100%** (USDC, USDT, BUSD, USDP, TUSD all recovered fully); only algorithmic stablecoins have permanently broken peg.
- The Federal Reserve's own framing (`[FED-4]`) puts stablecoin runs in the same conceptual basket as money-market-fund runs and bank deposit runs, both of which are extremely rare events that are nonetheless a foundational part of those instruments' risk profile.

This is a legitimate qualifier: depeg risk is *real but rare*. A V1 backtester running on 24 months of recent data has, with very high probability, **zero** depeg events in its dataset (the SVB event was March 2023, more than 24 months before the project's planning date), so M2.7 numbers will reflect a "no depeg in window" world even though the long-run prior says one event per several years is realistic.

## Verdict on Risk 2 — surface as UI flag; do **not** promote to V1 scope; revisit if scope expands to non-USDC stablecoins

**Reasoning by magnitude:**

- Conditional on a depeg event in the backtest window, the impact is large (-2% to -14% on the USDC half of the position over hours).
- Unconditional probability over a typical 90-day backtest window: very low (~1–2% based on historical event frequency).
- Expected value over a typical 90-day window: ~`0.015 × 0.07 = 10 bps` annualised drag on the position. **An order of magnitude smaller than the lending-APY benchmark; an order of magnitude smaller than the Risk 1 sandwich tax for high-rebalance strategies.**

That expected-value calculation is the case for keeping it out of scope in V1: it does not move the headline answer to "should you have LP'd at all?" When it bites, it bites; when it does not, modelling it adds ~0 explanatory value to the backtest.

**One exception:** if the user explicitly selects a backtest window straddling March 8–14 2023, the V1 backtester will silently report numbers that are wrong by a meaningful margin. The cleanest defence is a **window-aware UI banner**: when the selected backtest window contains a known depeg event, show a top-of-page warning naming the event and its magnitude. This is half a day of work and removes the worst-case "user trusts the number for the only window where it is dangerously wrong" failure mode.

**UI flag copy (recommended, default — present on every backtest):**

> *USDC is treated as $1 throughout the analysis. USDC has experienced one notable depeg (March 2023, trough ~$0.86, fully recovered within 60 hours). The position-value chart and benchmark comparisons do not adjust for stablecoin depeg risk; positions held during a depeg event will show realised loss in reality but face value here.*

**UI flag copy (window-aware banner — only when window contains March 8–14 2023):**

> *Selected window includes the March 2023 USDC depeg event. Backtester treats USDC as $1 throughout; an in-range position during this period would have absorbed ~50% of the depeg through inventory rotation, but an out-of-range USDC-heavy position would have realised ~5–14% loss for several hours. Reported numbers for this window do not reflect this.*

**Promoted to V1?** No. **Promoted to V2 if non-USDC stablecoin pools are added (DAI, FRAX, LUSD pools):** yes, because depeg event frequency rises with stablecoin diversity and the 100% recovery rate above does not generalise to algorithmic or partially-collateralised stablecoins. This is the magnitude-driven case for "it stays out of scope only as long as scope stays USDC-only."

---

## Risk 3 — Tax Drag on LP Operations

## What the topic actually is

LP operations that look like internal accounting moves to a quant model are taxable disposals to most national tax authorities. Concretely:

| LP operation | What the math sees | What the tax authority sees |
|---|---|---|
| Mint position (deposit WETH+USDC) | Conversion of token balance → liquidity | Disposal of WETH and USDC at FMV; potential capital gain/loss |
| Burn position (withdraw WETH+USDC) | Liquidity → token balance | Disposal of LP NFT; potential capital gain/loss |
| Collect fees | Fee accrual | Income at FMV at receipt |
| Rebalance (burn old + swap + mint new) | Range adjustment | **Two disposals + one swap + one acquisition**, all separately taxable |
| HODL benchmark equivalent | Hold WETH + USDC | No taxable events (hold) |

The asymmetry between the LP position and the HODL benchmark is the key pattern. **The HODL baseline pays no tax until the user sells; the LP position pays tax on every rebalance**. M2.7's benchmark comparison is therefore overstating LP relative performance by the cumulative tax cost of the rebalances that happened in the LP path but did not happen in the HODL path.

## What the published guidance says

**United States — IRS Notice 2014-21 + Rev Rul 2019-24 + 2024 broker-reporting rules**

The foundational rule is the IRS's "virtual currency is property" principle. From the IRS Virtual-Currency FAQ (the IRS's own plain-language consolidation of Notice 2014-21 and follow-up rulings):

> **Q-2:** "Virtual currency is treated as property and general tax principles applicable to property transactions apply to transactions using virtual currency." `[IRS-1]`
>
> **Q-16:** "If you exchange virtual currency held as a capital asset for other property, including for goods or for another virtual currency, you will recognize a capital gain or loss." `[IRS-2]`
>
> **Q-18:** "Your basis in that property is its fair market value at the time of the exchange." `[IRS-3]`

The implication chain for V3 LP operations:

```
Mint position    = exchange WETH + USDC for LP NFT
                 → Q-16: capital gain/loss on the WETH and USDC disposed

Burn position    = exchange LP NFT for WETH + USDC
                 → Q-16: capital gain/loss on the LP NFT disposed
                 → Q-18: basis in returned tokens = FMV at burn

Collect fees     = receive WETH + USDC outside the position
                 → Notice 2014-21 §4: ordinary income at FMV at receipt
                 → (then a fresh capital-gain holding period begins)

Rebalance        = burn + swap + mint
                 → three separate Q-16 events
```

A specialist DeFi-tax firm's interpretation of the same rules:

> "Currently, we view entering a liquidity pool as a disposal and therefore a taxable event." `[TAXBIT-1]`

The IRS itself has not issued LP-specific guidance. The "every operation is a disposal" interpretation is the conservative-consensus reading among DeFi-tax firms (TaxBit, Koinly, CoinTracker, CoinLedger), but it is **interpretation, not statute**. A genuine open uncertainty.

The 2024 broker-reporting rules (Form 1099-DA, finalised June 2024, effective for tax year 2025) require centralised brokers to report digital-asset disposals; they do not change the underlying tax treatment of LP operations, but they materially raise the audit-risk cost of getting LP basis tracking wrong.

**United Kingdom — HMRC**

UK guidance is more explicit on LP operations than US guidance. From a synthesis grounded in HMRC's general crypto guidance:

> "From HMRC's perspective, you've **disposed of your tokens** in exchange for a new asset (the LP token or position). That's a taxable event — specifically, a **Capital Gains Tax (CGT)** event." `[HMRC-1]`
>
> "The fees you earn as a liquidity provider are a separate tax event. HMRC treats them as **miscellaneous income**, taxed at your income tax rate (20%, 40%, or 45%)" `[HMRC-2]`
>
> "Income tax rates (20–45%) are higher than CGT rates (18–24%) for most people. Getting the split wrong means you could be underpaying tax on the income portion, or overpaying" `[HMRC-3]`

The HMRC rate split (CGT 18–24%, income 20–45%) is a real magnitude-relevant detail: an LP earning fees as income at 45% pays ~2× the rate of a HODLer realising capital gains at 24%. The asymmetry is structural, not just an event-counting issue.

**European Union — DAC8 (effective Jan 1 2026)**

DAC8 does not redefine taxable-event treatment (each member state still owns substantive tax law), but does require reporting:

> "DAC8 requires exchanges and brokers to report swaps because they involve the disposal of one asset and the acquisition of another, including trading ETH for SOL, exchanging stablecoins, or swapping tokens inside a liquidity pool structure." `[EU-DAC8-1]`

Equivalent in spirit to the US 1099-DA: raises audit-risk cost but does not change underlying drag.

## Magnitude — how much does this actually cost an LP?

Mechanical calculation for a US high earner (federal short-term cap-gains rate ~37%, plus state) on a representative strategy:

- Position: $10k WETH/USDC, 90-day backtest.
- WETH price drift over the 90 days: +20% (so each rebalance crystallises some unrealised gain into a realised disposal).
- Strategy: weekly rebalance (~13 rebalances).
- Average gain crystallised per rebalance: roughly `(20% × position-fraction-disposed-per-rebalance × time-elapsed-fraction)` ≈ ~0.3% of position per rebalance for a typical 50/50 rotation.
- Realised gains over 90 days: ~13 × 0.3% × $10k = ~$390.
- Tax cost at 37% short-term: ~$144.
- **Drag: ~$144 / $10k = 1.44% over 90 days = ~5.8% annualised.**

For HODL over the same 90 days: zero tax events.

A long-term-cap-gains earner (held >1 year, rate ~20%) sees ~half the drag. A non-US earner in a no-CGT jurisdiction sees zero. **Actual drag varies by 0× to 6× depending on jurisdiction and holding-period status.** That is a wide enough range that a single back-of-envelope `30-40% drag` figure is genuinely misleading.

**There is no rigorous public estimate** of after-tax LP returns at the level of detail an LP allocator needs. The TaxBit / Koinly / CoinTracker DeFi tax guides explain the rules; they do not publish empirical tax-drag numbers. This is a real research gap, not a citation-finding problem on my end.

## Contrasting view — when LP rebalance might NOT be a disposal

Two specific LP-tax positions that complicate the "every rebalance is a disposal" consensus:

1. **The "no economic disposition" argument.** If a rebalance is a burn-and-immediately-mint of an economically-equivalent position in the same wallet, an aggressive tax position is that no economic disposition has occurred (analogous to internal portfolio rebalancing inside a brokerage account). **No tax authority has accepted this position publicly.** TaxBit, Koinly, and CoinTracker all treat each rebalance as a disposal. A user who took the contrary position would be relying on (a) lack of explicit IRS LP-operation guidance and (b) a litigation argument that has not been tested.

2. **The "section 1031 like-kind" argument** is dead post-TCJA-2017 in the US (like-kind exchange is now real-estate-only for federal purposes), so this is no longer a live route. Worth flagging only because pre-2018 crypto-tax materials sometimes still mention it.

The honest framing: the consensus interpretation is conservative; safe-harbor rebalance treatments may emerge, but as of May 2026 they have not.

## Verdict on Risk 3 — surface as UI flag; do **not** promote to V1 scope; consider an after-tax toggle in V2

**Reasoning by magnitude:**

- Tax drag on weekly-rebalance strategy in a high-tax US jurisdiction: ~5.8% annualised on a +20%-drift period.
- Aave USDC supply APY: ~3–5%.
- **Tax drag is comparable to or exceeds the lending-APY benchmark.**

That is, on its face, a strong argument for promotion. Why I still recommend out-of-scope for V1:

| Reason | Detail |
|---|---|
| Jurisdiction-dependence is wide | 0× to 6× drag range across realistic users; modelling one regime well misleads users in other regimes worse than not modelling at all |
| Holding-period modelling is non-trivial | Short-term vs long-term distinction requires per-lot tracking across rebalances; basis-method choice (FIFO/LIFO/HIFO) is user-dependent |
| HODL-comparison asymmetry is partially self-correcting | A user who would not actually rebalance often (because they know their tax cost) selects strategies the backtester ranks correctly anyway |
| The right shape is an opt-in toggle, not a default model | Like SEC-mandated after-tax mutual fund returns: presented *alongside* pre-tax, with explicit assumptions, never as the only number |

**The strongest analogue is SEC Rule 482.** Mutual funds report both pre-tax and after-tax returns under standardised assumptions (highest individual federal marginal rate, no state tax, dividends taxed when paid). The Aurix V2 equivalent would be:

- A configurable jurisdiction + rate setting (US 37% short-term, US 20% long-term, UK 24% CGT + 45% income, EU member-state placeholder, "no tax" for non-US/UK/EU).
- A configurable basis method (FIFO default, HIFO option).
- An after-tax equity curve overlay computed by deducting modelled tax at each disposal event.
- Loud disclosure that the model is approximate and not tax advice.

This is a 1–2 week feature, not a few-days haircut. That is why it stays out of scope for V1 and is the *natural* candidate for first major V2 work.

**UI flag copy (recommended for V1):**

> *Tax treatment is not modelled. In the US and UK, every rebalance is generally a taxable disposal of the LP position; fee collections are generally taxable as income (UK) or treated as part of the basis-tracking flow (US, less clear). For a US high earner running a weekly-rebalance strategy on a positive-drift period, modelled tax drag can reach ~5% annualised — comparable to the lending-APY benchmark. The HODL benchmark in the comparison table pays no tax until sold, so reported alpha vs HODL is overstated by approximately the rebalance-tax cost. Numbers shown are pre-tax.*

The "pre-tax" framing at the end matters: it gives the user the same mental model as their mutual-fund statements and prevents the misread "this is an after-tax number."

---

## Research Signal — Cross-Risk Synthesis

### Magnitude side-by-side

```
Annualised drag on $10k WETH/USDC LP, weekly-rebalance, US high earner
─────────────────────────────────────────────────────────────────────
Risk                          0       100     200     300     400     500     600  bp
Aave benchmark APY (target)   ████████████████████████ ~3-5%
MEV / sandwich (Risk 1)       ███████████████████████████  ~250-500 bp
USDC depeg (Risk 2)           █  ~10 bp expected (high variance)
Tax drag (Risk 3)             ████████████████████████████████████  ~580 bp
Existing modelled costs:
  Management gas (M2.3)       ████████  varies, 50-200 bp typical
  Impermanent loss (M2.3)     varies wildly by drift; first-class modelled
```

The visualisation makes the priority obvious: **on a high-rebalance strategy, the unmodelled costs (Risks 1 and 3) collectively exceed the modelled-and-benchmarked Aave APY**. That is the magnitude-based case for taking the disclosure language seriously rather than glossing it.

## Recommended Priority Order — Verdict Matrix

| Risk | Magnitude (annualised, weekly rebalance, retail) | Verdict | Effort if promoted |
|---|---|---|---|
| MEV / sandwich tax | 250–500 bp | Stay out of V1 as full model; **add coarse `--mev-haircut-bps-per-rebalance` knob in V1** (1–2 days); UI flag | 1–2 weeks for full per-swap simulation |
| USDC depeg | ~10 bp expected, -1400 bp conditional on event-in-window | Stay out; UI flag default + window-aware banner for known event windows | 1 week (reserve-attestation series + window-conditional modelling) |
| Tax drag | 0–600 bp depending on jurisdiction | Stay out; UI flag; **natural V2 first feature** as opt-in after-tax toggle modelled on SEC Rule 482 | 1–2 weeks |

The one risk where I would push back against the V1 plan's "purely out of scope" framing is **MEV cost on rebalance swaps for high-rebalance strategies**. The magnitude crosses the Aave benchmark. The mitigation is small (a single-knob coarse haircut). The cost of leaving it out is that the strategy comparison heatmap (M2.5) ranks high-rebalance strategies above their true alpha, which is the exact misranking that misleads the "should you have LP'd?" headline (M2.8) the project is built around.

### UI disclosure pattern — what good looks like

The two UI conventions worth borrowing from TradFi:

1. **Mutual-fund pre-tax / after-tax pair (SEC Rule 482).** Always present both numbers side-by-side, with the assumption set named once. The user sees the magnitude of the difference and gets the correct mental model. This is the right shape for the V2 tax toggle.
2. **Backtest-software "Important Limitations" footer.** Standard practice across QuantConnect / Backtrader / Backtest.io: a fixed footer block on every results page listing what is and is not modelled. The list is short, specific, and named — not a wall of legal text. Aurix's equivalent is the three named-risk flags above plus the existing "this is historical replay, not live execution" framing.

Anti-patterns to avoid (these are the failure modes that turn disclosures into noise):

- **Wall-of-legal-text footer.** Users learn to ignore it after one sighting. The "Surgeon General's Warning" effect: present once, never read again.
- **Disclaimer modal on first load.** Same problem; users dismiss-and-forget.
- **Inline parenthetical caveats** scattered through every metric. Death by a thousand qualifiers; the user cannot tell which qualifier matters most.
- **"Past performance does not guarantee future results"-style boilerplate.** True, useless, and indistinguishable from filler text.

The recommended Aurix shape:

```
┌────────────────────────────────────────────────────────────────┐
│ HEADLINE STRIP (M2.6)                                           │
│ "WETH/USDC LP outperformed lending in 6 of 24 months,           │
│  conditional on high-vol regime. Use as rotation strategy,      │
│  not default allocation."                                       │
└────────────────────────────────────────────────────────────────┘
        │ (i)  3 risks not modelled — click for details
        │     (badge expands to inline flag block)
        v
        ┌────────────────────────────────────────────────────────┐
        │ Risks not modelled in this backtest:                    │
        │  - MEV/sandwich tax on rebalance swaps                  │
        │     ~5-500 bp annualised depending on rebalance freq    │
        │  - USDC de-peg risk (one event in 7 yrs, -14% peak)     │
        │  - Tax drag on rebalances (~0-580 bp, jurisdiction-dep) │
        │ Numbers shown are pre-tax, pre-MEV, peg-assumed.        │
        └────────────────────────────────────────────────────────┘
```

The badge collapses by default; the flag block expands inline (not as a modal). The user can dismiss for the session but the badge remains visible — same convention as a Bloomberg "see disclosure" affordance.

---

## Open Uncertainties And Validation Needs

| Uncertainty | What would resolve it |
|---|---|
| Empirical sandwich-tax bps for $1k–$100k WETH/USDC 5bps-pool swaps with default slippage protection | Sample 50–100 historical mainnet entry/exit swaps in this size range; measure realised execution price vs concurrent mid-price |
| Whether the recommended `~5 bp` per-rebalance MEV haircut is right or 2× too low | Empirical validation in M2.4 — the modelled-vs-actual fee discrepancy on high-volume blocks should track this |
| Whether IRS will issue LP-specific guidance before V2 is built | Watch IRS digital-assets page for new revenue rulings; the 2024 broker rules suggest more guidance is in the pipeline |
| Whether the 100% recovery rate of fiat-backed stablecoin depegs since 2018 generalises | Cannot be resolved by data, only by the next event |
| Whether the SEC Rule 482 after-tax-pair shape is the right UI affordance for an LP backtester or whether quant LP allocators want a different convention | Show three quant-desk practitioners the M2.6 mockup with and without the pair; ask which one they would actually use |

---

## Relationship To Existing Context

This paper depends on:

- `context/plans/vector-a-v3-lp-backtester.md` — the V1 plan whose `## Out of Scope` section (lines 277–279) this paper expands into actionable disclosure language.
- `context/architecture.md` — for the current state of the implementation surface (no Tab 2 code yet, M2.6 frontend not started).

This paper extends but does not supersede:

- `v3-lp-profitability-literature.md` — currently a scaffold; whoever first populates that paper should cross-reference here for the un-modelled-risk component of the LP-vs-benchmark alpha calculation.
- `lp-rebalancing-strategies.md` — currently a scaffold; whoever first populates that paper should reference the per-rebalance MEV haircut and per-rebalance tax cost from here when reasoning about rebalance frequency optimisation. **Tax drag in particular changes the optimal rebalance frequency materially** in a tax-aware optimiser.

Future research the recommendations here imply:

- A dedicated paper on coarse MEV-haircut modelling, *if* the V1 implementation discovers in M2.4 validation that the sandwich tax meaningfully explains the fee-discrepancy distribution.
- A dedicated paper on after-tax LP returns modelling, *when* V2 work begins on the SEC-Rule-482-style toggle.

---

## External Research Trail

**Searches run**

| # | Query | Tool | Rationale | Sources surfaced |
|---|---|---|---|---|
| 1 | Qin Zhou Gervais 2022 "Quantifying Blockchain Extractable Value" sandwich attack profit | WebSearch | Foundational paper anchor for MEV section | arXiv 2101.05511, UCL Discovery, Semantic Scholar |
| 2 | Uniswap V3 sandwich attack cost basis points WETH USDC 0.05% pool 2024 | WebSearch | Per-pool magnitude on the exact Aurix target pool | EigenPhi, GeckoTerminal, JIT IACR paper |
| 3 | USDC depeg March 2023 SVB magnitude duration Circle reserve disclosure timeline | WebSearch | Magnitude + duration anchors for depeg section | Federal Reserve note, CNBC, CoinDesk, MDPI |
| 4 | IRS Notice 2014-21 Rev Rul 2019-24 cryptocurrency taxable event liquidity provision rebalance | WebSearch | US tax foundational guidance | IRS FAQ, Notice 2014-21 PDF, Fenwick analysis |
| 5 | Uniswap V3 LP rebalance taxable event USA UK HMRC capital gains 2024 2025 | WebSearch | Jurisdiction-specific LP tax treatment | Koinly, ChainTax, TaxBit, TokenTax |
| 6 | eigenphi.io MEV statistics sandwich tax extraction Uniswap V3 daily | WebSearch | Production-data MEV figures | EigenPhi research, Diamond Protocol Medium |
| 7 | "sandwich attack" overestimated MEV criticism ineffective Uniswap V3 retail swaps slippage protection | WebSearch | **Contrasting source** for Risk 1 | Uniswap Labs support, Hayden Adams comments |
| 8 | USDT Tether depeg October 2018 history magnitude DAI flash depeg list stablecoin events | WebSearch | Historical depeg event list for context | Protos, Wikipedia, Kraken, S&P Global |
| 9 | Morningstar methodology document fund analysis "limitations" "risks not modeled" formatting | WebSearch | TradFi disclosure-pattern research | Morningstar methodology PDFs |
| 10 | portfolio backtest disclaimer "tax not included" "transaction costs" "slippage" UI best practice | WebSearch | Backtest disclosure conventions | QuantStart, Bocconi BSIC, LuxAlgo |
| 11 | Federal Reserve research note USDC depeg SVB stablecoin run flight to safety 2023 | WebSearch | Primary-source USDC depeg analysis | Federal Reserve note (Dec 2025), NY Fed staff report |
| 12 | EU MiCA DAC8 cryptocurrency tax reporting liquidity pool 2024 2025 disposal | WebSearch | EU jurisdiction tax framework | EC DAC8 page, Coincub, EY, Coindesk |
| 13 | libmev.com Uniswap V3 sandwich profit per trade WETH USDC weekly statistics | WebSearch | libMEV-specific data | JIT IACR paper, Heimbach arXiv 2205.08904, Frontier Tech |
| 14 | "average sandwich attack" basis points percent victim swap size Ethereum 2024 typical | WebSearch | Per-attack magnitude triangulation | Blockworks, ACM SandWatch, arXiv Remeasuring |
| 15 | Koinly Uniswap V3 LP rebalance tax burn mint "not a taxable event" same wallet position | WebSearch | **Contrasting source** for Risk 3 (rebalance not a disposal?) | Koinly forum, Koinly help center |
| 16 | "Trading in the Dark" Frontier Tech reordering slippage MEV measurement 2024 | WebSearch | Per-pool reordering-slippage data | Frontier Tech (frontier.tech) |
| 17 | SEC Rule 482 backtest disclosure "hypothetical" tax fees not reflected investment company | WebSearch | TradFi disclosure analogue for tax modelling | SEC after-tax mutual-fund rule, K&L Gates analysis |

**Sources consulted**

| URL | Tool | Source class | Used for | Quoted? |
|---|---|---|---|---|
| https://arxiv.org/abs/2101.05511 | WebFetch | Foundational paper (peer-reviewed) | Qin et al. BEV aggregate figure | Yes — `[QIN-1]` |
| https://medium.com/@eigenphi/mevs-impact-on-uniswap-c36c7dfbd3d4 | WebFetch | Production write-up | EigenPhi MEV-on-Uniswap aggregates | Indirect (no direct passage matched) |
| https://www.irs.gov/individuals/international-taxpayers/frequently-asked-questions-on-virtual-currency-transactions | WebFetch | Official documentation | IRS Q-2, Q-16, Q-18 verbatim | Yes — `[IRS-1/2/3]` |
| https://chaintax.co.uk/blog/how-hmrc-taxes-uniswap-lp-positions | WebFetch | Production write-up (specialist firm) | HMRC LP-position interpretation | Yes — `[HMRC-1/2/3]` |
| https://www.federalreserve.gov/econres/notes/feds-notes/in-the-shadow-of-bank-run-lessons-from-the-silicon-valley-bank-failure-and-its-impact-on-stablecoins-20251217.html | WebFetch | Official documentation (Federal Reserve) | USDC depeg primary analysis | Yes — `[FED-1/2/3/4]` |
| https://medium.com/taxbit-eng/uniswap-v3-liquidity-pools-and-the-taxable-implications-entering-an-lp-f5ba2c1defc8 | WebFetch | Production write-up (specialist firm) | TaxBit LP disposal interpretation | Yes — `[TAXBIT-1]` |
| https://arxiv.org/abs/2205.08904 | WebFetch | Foundational paper (peer-reviewed, Heimbach et al.) | V3 LP returns/risks framing | Indirect (abstract only available via WebFetch) |
| https://medium.com/diamond-protocol/all-you-need-to-know-about-sandwich-attacks-jit-on-uniswap-e31076435788 | WebFetch | Production write-up | EigenPhi-derived per-pool numbers | Yes — `[EIGENPHI-1/2]` |
| https://frontier.tech/measuring-reordering-slippage-in-mev | WebFetch | Benchmark / measurement study | Per-pool reordering slippage on WETH/USDC | Yes — `[FRONTIER-1/2]` |

Other sources surfaced via search but not directly fetched (used as triangulation context):

- Blockworks Sandwich-Attacks article (`[BLOCKWORKS-1/2]` — quoted via search excerpt)
- Uniswap Labs support article + Hayden Adams comments (`[UNISWAP-MEV-1/2]` — quoted via search excerpt)
- New York Fed Staff Report 1073 "Runs and Flights to Safety" (used for stablecoin run conceptual framing)
- DAC8 EU Commission documentation (`[EU-DAC8-1]` — quoted via search excerpt)
- Protos history-of-tether-peg article (used for USDT 2018 depeg magnitude)

**Quoted passages**

- **`[QIN-1]`** — Qin, Zhou, Gervais 2022 abstract — https://arxiv.org/abs/2101.05511
> "We estimate that over 32 months, BEV yielded 540.54M USD in profit, divided among 11,289 addresses when capturing 49,691 cryptocurrencies and 60,830 on-chain markets."

- **`[EIGENPHI-1]`** — Diamond Protocol Medium summarising EigenPhi WETH/USDC 0.05% pool data
> "In the past 180 days, there were 5,346 Sandwich Attacks."

- **`[EIGENPHI-2]`** — Diamond Protocol Medium summarising EigenPhi WETH/USDC 0.05% pool data
> "Out of the $100M fee, $2.26M was earned by JIT trades."

- **`[FRONTIER-1]`** — Frontier Tech, "Trading in the Dark"
> "Large Uniswap V3 pools like WETH/USDC, WETH/WBTC, and WETH/USDT exhibit lower mean reordering percentages at .05%, .09%, and .04% respectively for swaps over $250k."

- **`[FRONTIER-2]`** — Frontier Tech, "Trading in the Dark"
> "72% of swaps were negatively affected for $12.811m. The other 28% were positively affected by $3.749m."

- **`[BLOCKWORKS-1]`** — Blockworks via search excerpt, citing TradingView/EigenPhi data
> "Monthly extraction from sandwich attacks dropped from nearly $10 million in late 2024 to about $2.5 million by October 2025."

- **`[BLOCKWORKS-2]`** — Blockworks via search excerpt
> "The average profit per sandwich attack remains extremely low at just above $3 in recent data from 2025."

- **`[UNISWAP-MEV-1]`** — Uniswap Labs support article via search excerpt (contrasting source for Risk 1)
> "Uniswap's official interface includes built-in MEV protections and default slippage settings to guard against sandwich attacks."

- **`[UNISWAP-MEV-2]`** — Sandwich-attack mitigation guide via search excerpt
> "For stablecoin swaps in deep liquidity pools, traders can set slippage to 0.01% - 0.05%. For volatile assets, slippage can be calculated so that a bot's potential profit after paying network fees becomes negative."

- **`[FED-1]`** — Federal Reserve Note, "In the Shadow of Bank Runs", Dec 2025
> "At its trough, USDC traded at 86 cents to the dollar."

- **`[FED-2]`** — Federal Reserve Note, "In the Shadow of Bank Runs"
> "USDC's price on the secondary market recovered sharply after the backstop announcement on Sunday and fully recovered once Circle began processing redemptions on Monday, March 13."

- **`[FED-3]`** — Federal Reserve Note, "In the Shadow of Bank Runs"
> "Both Tether (USDT) and Binance USD (BUSD) appreciated over the weekend and traded at a price marginally above their one-dollar peg."

- **`[FED-4]`** — Federal Reserve Note, "In the Shadow of Bank Runs" (contrasting/qualifying source — frames depeg risk as conceptually equivalent to MMF runs, i.e. real but rare)
> "As with money market funds and bank deposits, stablecoins are susceptible to crises of confidence, contagion, and self-reinforcing runs."

- **`[IRS-1]`** — IRS Virtual Currency FAQ, Q-2 — https://www.irs.gov/individuals/international-taxpayers/frequently-asked-questions-on-virtual-currency-transactions
> "Virtual currency is treated as property and general tax principles applicable to property transactions apply to transactions using virtual currency."

- **`[IRS-2]`** — IRS Virtual Currency FAQ, Q-16
> "If you exchange virtual currency held as a capital asset for other property, including for goods or for another virtual currency, you will recognize a capital gain or loss."

- **`[IRS-3]`** — IRS Virtual Currency FAQ, Q-18
> "Your basis in that property is its fair market value at the time of the exchange."

- **`[HMRC-1]`** — ChainTax (specialist UK DeFi-tax firm) on HMRC LP treatment
> "From HMRC's perspective, you've disposed of your tokens in exchange for a new asset (the LP token or position). That's a taxable event — specifically, a Capital Gains Tax (CGT) event."

- **`[HMRC-2]`** — ChainTax on HMRC fee-income treatment
> "The fees you earn as a liquidity provider are a separate tax event. HMRC treats them as miscellaneous income, taxed at your income tax rate (20%, 40%, or 45%)"

- **`[HMRC-3]`** — ChainTax on rate-split magnitude
> "Income tax rates (20–45%) are higher than CGT rates (18–24%) for most people. Getting the split wrong means you could be underpaying tax on the income portion, or overpaying"

- **`[TAXBIT-1]`** — TaxBit Engineering Medium on US LP entry treatment
> "Currently, we view entering a liquidity pool as a disposal and therefore a taxable event."

- **`[EU-DAC8-1]`** — DAC8 reporting requirement summary via search excerpt
> "DAC8 requires exchanges and brokers to report swaps because they involve the disposal of one asset and the acquisition of another, including trading ETH for SOL, exchanging stablecoins, or swapping tokens inside a liquidity pool structure."

---

## Pre-Completion Obligation Audit

| Obligation | Status | Evidence |
|---|---|---|
| At least 3 distinct WebSearch calls with topic-specific queries | Met | 17 distinct queries listed in External Research Trail / Searches run |
| At least 3 distinct WebFetch calls against primary sources | Met | 9 successful WebFetch retrievals listed in Sources consulted (plus 4 attempted that failed on PDF/access) |
| Sources span at least 2 source classes | Met | 4 classes covered: foundational paper (Qin, Heimbach), official documentation (IRS FAQ, Federal Reserve), production write-up (EigenPhi/Diamond, TaxBit, ChainTax), benchmark/measurement (Frontier Tech) |
| At least 1 direct quoted passage per major source-backed claim | Met | 21 quoted passages with passage IDs `[QIN-1]`, `[EIGENPHI-1/2]`, `[FRONTIER-1/2]`, `[BLOCKWORKS-1/2]`, `[UNISWAP-MEV-1/2]`, `[FED-1/2/3/4]`, `[IRS-1/2/3]`, `[HMRC-1/2/3]`, `[TAXBIT-1]`, `[EU-DAC8-1]` |
| At least 1 contrasting / limiting / disagreeing source consulted | Met | Three contrasting/limiting sources represented: `[UNISWAP-MEV-1/2]` (sandwich tax overstated for protected swaps), `[FRONTIER-1/2]` (the 5bps reordering figure for $250k+ swaps qualifies the "MEV is huge" headline), `[FED-4]` (frames depeg risk as rare-but-real, equivalent to MMF runs); plus `[BLOCKWORKS-1/2]` showing sandwich extraction declined ~75% from late 2024 to late 2025 |
| Relevant `context/` files read before project-specific claims | Met | Read: `context/architecture.md`, `context/notes.md`, `context/plans/vector-a-v3-lp-backtester.md` (line-cited in body at 277, 278, 279), `context/references/v3-lp-profitability-literature.md` (verified scaffold-only), `context/references/lp-rebalancing-strategies.md` (verified scaffold-only) |
| Relevant code inspected (list file paths) | Partial | No Tab 2 source code exists yet — the V1 plan has not been implemented. Repository state verified via `ls context/references/` and reading the plan; no code surface to inspect because the topic concerns out-of-scope items in an unbuilt feature. |
| `scripts/init_research_artifact.py` run (stdout captured) | Met | `Created file scaffold: /Users/atacanercetinkaya/Documents/Programming-Projects/Aurix/context/references/out-of-scope-risks-survey.md` |
| `scripts/validate_research_artifact.py` run (stdout captured) | Met | All checks pass: 9 URLs / 6 unique domains in trail, 21 quoted passages, 4/4 evidence labels, no exhortation adverbs, all required and template sections present |

---

## What I Did Not Do

- **No empirical sandwich-tax measurement on actual historical Aurix-shaped swaps.** The bps figures in the magnitude table are inferred from production-data sources (EigenPhi, Frontier Tech, Blockworks/2025) scaled to retail position sizes. A V2 promotion to actual MEV-cost modelling should validate the haircut against measured swap-execution data on the WETH/USDC 5bps pool.
- **No primary-source extraction from Heimbach et al. (arXiv 2205.08904).** The PDF returned binary-stream content via WebFetch; only the abstract was extractable. The paper is cited as a reference for the V3-LP-complexity framing but its specific sandwich/JIT figures are not quoted directly. The Diamond Protocol Medium and EigenPhi blog cover the same data set and were quoted directly in their place.
- **No primary-source extraction from the JIT-attack paper (eprint.iacr.org/2023/973).** Returned 403 on WebFetch. The JIT framing in this paper draws on EigenPhi's 6 months / $6M JIT figure instead.
- **No direct fetch of the IRS Notice 2014-21 PDF.** Returned binary-stream content; the verbatim Q-2/Q-16/Q-18 quotes come from the IRS's own HTML FAQ which consolidates the Notice's substantive rules. Equivalent authority for the rule statements; the Notice itself remains the formal citation.
- **No fetch of the SEC after-tax-returns release.** Returned 403. The SEC Rule 482 framing is grounded in published commentary (K&L Gates) plus search-excerpt summaries; the rule's pre-tax/after-tax-pair pattern is well-attested across multiple sources but I do not quote the SEC release text directly.
- **No Bloomberg Terminal disclosure-pattern data.** Bloomberg's exact terminal disclosure language is not publicly indexed (search returned no results for the specific phrase combinations). The UI disclosure recommendations rely on SEC Rule 482, Morningstar methodology document patterns, and standard backtesting-software conventions instead.
- **No exhaustive depeg-event catalogue.** The "Other notable depeg events" table covers the events whose magnitude or duration changes the recommendation. Smaller and more obscure depegs (TUSD micro-fluctuations, sUSD historical drift, etc.) are not included because they do not change the verdict.
- **No tax-modelling for jurisdictions outside US/UK/EU.** The published guidance density drops sharply outside those three; the recommendation to put after-tax modelling in V2 with a configurable jurisdiction implicitly handles this but no specific Australia / Singapore / Canada / Japan numbers are produced here.
- **No quant-LP-desk practitioner interviews to validate the recommended UI shape.** The "show three quant-desk practitioners the M2.6 mockup" item in Open Uncertainties is the right way to resolve this; doing it is out of scope for a research paper.
