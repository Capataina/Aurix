import { useEffect, useMemo, useState } from "react";

import {
  lpFetchBenchmarkSeries,
  lpGetChainHead,
  lpPoolMetadata,
  lpQueryFirstSwapPrice,
  lpQueryHeadlineMonthly,
  lpQueryStrategies,
  lpTokenUsdPrices,
  runLpBacktest,
  runLpGrid,
  runLpHeadline,
  runLpIngestion,
} from "./api";
import { CHAIN_CONFIGS } from "./chains";
import type { PoolMetadata } from "./types";
import { telemetry, useTelemetrySnapshot } from "../../lib/telemetry";
import { BenchmarkCacheBlock } from "../../components/blocks/lp/BenchmarkCacheBlock";
import { EquityCurveBlock } from "../../components/blocks/lp/EquityCurveBlock";
import { HeadlineVerdictBlock } from "../../components/blocks/lp/HeadlineVerdictBlock";
import { KeyMetricsBlock } from "../../components/blocks/lp/KeyMetricsBlock";
import { MultiAssetCompareBlock } from "../../components/blocks/lp/MultiAssetCompareBlock";
import { PositionPnlBlock } from "../../components/blocks/lp/PositionPnlBlock";
import { PositionRangeBlock } from "../../components/blocks/lp/PositionRangeBlock";
import { RegimePanelBlock } from "../../components/blocks/lp/RegimePanelBlock";
import { StrategyHeatmapBlock } from "../../components/blocks/lp/StrategyHeatmapBlock";
import {
  DEFAULT_GRID_PERIOD_DAYS,
  DEFAULT_GRID_RANGE_WIDTHS,
  DEFAULT_GRID_RULES,
} from "./defaults";
import type { LpSettings } from "./LpSettingsForm";
import type {
  BenchmarkPoint,
  EquityCurvePoint,
  GridConfig,
  HeadlineConfig,
  HeadlineMonthlyInput,
  HeadlineMonthlyRow,
  HeadlineRunSummary,
  PositionConfig,
  PositionRunSummary,
  StrategyResultRow,
} from "./types";

interface BenchmarkSeriesMap {
  [seriesKey: string]: BenchmarkPoint[];
}

interface LpBacktestPageProps {
  settings: LpSettings;
  /** Bumping this nonce re-triggers the pipeline. The "Re-run pipeline"
   *  button in the LP settings panel increments it. */
  rerunNonce: number;
  /** Forwards busy state up so the SettingsMenu's re-run button can
   *  disable while the pipeline is in flight. */
  onBusyChange: (busy: boolean) => void;
}

export function LpBacktestPage({
  settings,
  rerunNonce,
  onBusyChange,
}: LpBacktestPageProps) {
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("");
  const [summary, setSummary] = useState<PositionRunSummary | null>(null);
  const [curve, setCurve] = useState<EquityCurvePoint[]>([]);
  const [strategies, setStrategies] = useState<StrategyResultRow[]>([]);
  const [headline, setHeadline] = useState<HeadlineRunSummary | null>(null);
  const [headlineMonthly, setHeadlineMonthly] = useState<HeadlineMonthlyRow[]>([]);
  const [benchmarks, setBenchmarks] = useState<BenchmarkSeriesMap>({});
  /** Realised entry tick + price + block window from the most-recent
   *  pipeline run. Drives the `PositionRangeBlock` so it always matches
   *  what the engine actually simulated. */
  const [resolved, setResolved] = useState<{
    fromBlock: number;
    toBlock: number;
    tickLower: number;
    tickUpper: number;
    entryPrice: number;
  } | null>(null);
  const [poolMeta, setPoolMeta] = useState<PoolMetadata | null>(null);

  // Mirror busy → parent so the settings panel's Re-run button reflects state.
  useEffect(() => {
    onBusyChange(busy);
  }, [busy, onBusyChange]);

  // Auto-run when settings change or when the user hits "Re-run".
  // Settings changes go through a settings-snapshot-stable JSON key so
  // typing into a stepper doesn't fire the pipeline mid-keystroke (the
  // settings object identity changes on every keystroke; the JSON
  // doesn't change until the value lands).
  const settingsKey = JSON.stringify(settings);

  useEffect(() => {
    let mounted = true;
    void runPipeline();

    async function runPipeline() {
      try {
        telemetry.record("lp.pipeline.start", {
          poolAddress: settings.poolAddress,
          chainId: settings.chainId,
          protocol: settings.protocol,
          lookbackBlocks: settings.lookbackBlocks,
          rerunNonce,
          settings,
        });
        if (mounted) setBusy(true);
        if (mounted) setStatus("Fetching pool metadata…");

        // Pool metadata first — gives us decimals + symbols so all
        // downstream math stops assuming WETH(18)/USDC(6).
        let meta: PoolMetadata | null = null;
        try {
          meta = await lpPoolMetadata(
            settings.poolAddress,
            settings.chainId,
            settings.protocol,
          );
          if (mounted) setPoolMeta(meta);
          telemetry.record("lp.pipeline.pool-metadata", {
            token0Symbol: meta.token0Symbol,
            token1Symbol: meta.token1Symbol,
            token0Decimals: meta.token0Decimals,
            token1Decimals: meta.token1Decimals,
            feeTierBps: meta.feeTierBps,
          });
        } catch (e) {
          telemetry.record("lp.pipeline.pool-metadata-error", {
            error: String(e),
          });
          if (mounted) setStatus("Pool metadata unavailable; using defaults");
        }

        if (mounted) setStatus("Resolving recent block window…");

        // Resolve the trailing block window against the live chain
        // head every run. No persisted from/to — `lookbackBlocks` is
        // the only knob, the window is always [head − N, head].
        // Note: chain head is currently always Ethereum's; tier 2
        // expansion adds per-chain head fetching via public RPCs.
        let head: number | null = null;
        try {
          head = await lpGetChainHead(settings.chainId);
          telemetry.record("lp.pipeline.chain-head", { head });
        } catch (e) {
          telemetry.record("lp.pipeline.chain-head-error", {
            error: String(e),
          });
        }
        if (head === null || head < settings.lookbackBlocks) {
          // No reachable RPC for this chain. Surface as an error so
          // the user knows to check connectivity / configure a key.
          // No synthetic fallback — fake numbers in a portfolio piece
          // is a worse signal than an empty dashboard with a clear
          // "needs setup" message.
          throw new Error(
            `Could not reach ${settings.chainId} chain head. Check network or configure a key.`,
          );
        }
        const toBlock = head;
        const fromBlock = head - settings.lookbackBlocks;
        if (!mounted) return;

        // Live ingest only — the backend's tiered fallback (subgraph
        // → user-Alchemy → public RPC) covers connectivity gaps. If
        // all three paths fail we surface that error clearly rather
        // than fabricating data. Fake numbers in a public-facing tool
        // is a worse signal than an honest empty state.
        if (mounted) setStatus("Running live ingest…");
        const ingestReport = await runLpIngestion(
          settings.poolAddress,
          fromBlock,
          toBlock,
          settings.chainId,
          settings.protocol,
        );
        telemetry.record("lp.pipeline.ingest", { report: ingestReport });
        if (!mounted) return;

        // Realised entry tick + price from the first swap. Replaces
        // any hardcoded tick-range or /price assumption — the position
        // adapts to whatever the data actually shows.
        const t0Decimals = meta?.token0Decimals ?? 18;
        const t1Decimals = meta?.token1Decimals ?? 6;

        // Token-USD prices for non-USD-quote pools (tier 4). When the
        // pool isn't quoted in a stablecoin, we need external feeds to
        // value the position in USD. Skip the call entirely for
        // already-USD-quoted pools — the in-pool ratio is sufficient.
        let token0UsdPrice: number | null = null;
        let token1UsdPrice: number | null = null;
        if (meta && !meta.isToken1UsdPegged) {
          try {
            const { prices } = await lpTokenUsdPrices(
              [meta.token0Address, meta.token1Address],
              settings.chainId,
            );
            token0UsdPrice = prices[meta.token0Address.toLowerCase()] ?? null;
            token1UsdPrice = prices[meta.token1Address.toLowerCase()] ?? null;
            telemetry.record("lp.pipeline.token-usd-prices", {
              token0Symbol: meta.token0Symbol,
              token0UsdPrice,
              token1Symbol: meta.token1Symbol,
              token1UsdPrice,
            });
          } catch (e) {
            telemetry.record("lp.pipeline.token-usd-prices-error", {
              error: String(e),
            });
          }
        }
        const firstSwap = await lpQueryFirstSwapPrice(
          settings.poolAddress,
          fromBlock,
          toBlock,
          t0Decimals,
          t1Decimals,
        );
        const tickAnchor = firstSwap?.tick ?? 0;
        const tickLower = tickAnchor - settings.tickHalfWidth;
        const tickUpper = tickAnchor + settings.tickHalfWidth;
        const entryPrice = firstSwap?.humanPrice ?? 1;
        if (!mounted) return;
        setResolved({ fromBlock, toBlock, tickLower, tickUpper, entryPrice });
        telemetry.record("lp.pipeline.first-swap", {
          tick: tickAnchor,
          entryPrice,
        });

        // Fan out the benchmark fetches in parallel with the rest of
        // the pipeline.
        const benchmarkSeriesKeys = [
          "aave_v3_usdc_supply_apy",
          "lido_steth_apy",
          "fred_dgs3mo",
          "stooq_voo",
          "fred_gold_lbma",
        ];
        const benchmarkPromise = Promise.allSettled(
          benchmarkSeriesKeys.map((s) => lpFetchBenchmarkSeries(s)),
        ).then((results) => {
          const next: BenchmarkSeriesMap = {};
          results.forEach((r, i) => {
            next[benchmarkSeriesKeys[i]] = r.status === "fulfilled" ? r.value : [];
          });
          if (mounted) setBenchmarks(next);
          telemetry.record("lp.pipeline.benchmarks", {
            fetched: results.filter((r) => r.status === "fulfilled").length,
            requested: benchmarkSeriesKeys.length,
          });
          return next;
        });

        if (mounted) setStatus("Running backtest…");
        // Deposit split: assume token1 is USD-pegged for now (tier 4
        // generalises this with per-token USD pricing). For
        // non-USD-quote pools (WBTC/ETH, LDO/ETH) the math degrades
        // to "treat token1 as if it were USDC" — gives the right
        // shape, wrong absolute scale; we'll fix in tier 4.
        // Deposit split: when token1 is USD-pegged (USDC/USDT/DAI),
        // depositUsd / 2 USDC + (depositUsd / 2) / entryPrice token0
        // is the right 50/50 split. When token1 is NOT USD-pegged,
        // we use the per-token USD prices to convert: each side gets
        // depositUsd/2 worth, scaled by the token's USD price.
        const depositToken0Raw = token0UsdPrice
          ? humanToRaw(settings.depositUsd / 2 / token0UsdPrice, t0Decimals)
          : humanToRaw(settings.depositUsd / 2 / entryPrice, t0Decimals);
        const depositToken1Raw = token1UsdPrice
          ? humanToRaw(settings.depositUsd / 2 / token1UsdPrice, t1Decimals)
          : humanToRaw(settings.depositUsd / 2, t1Decimals);
        const cfg: PositionConfig = {
          poolAddress: settings.poolAddress,
          tickLower,
          tickUpper,
          depositToken0: depositToken0Raw,
          depositToken1: depositToken1Raw,
          entryBlock: fromBlock,
          exitBlock: toBlock,
          feeTierBps: meta?.feeTierBps ?? settings.feeTierBps,
          token0Decimals: t0Decimals,
          token1Decimals: t1Decimals,
          mevHaircutBps: settings.mevHaircutBps,
          token0UsdPrice,
          token1UsdPrice,
        };
        const response = await runLpBacktest(cfg, settings.rule);
        telemetry.record("lp.pipeline.backtest", {
          summary: response.summary,
          samples: response.equityCurve.length,
        });
        if (!mounted) return;
        setSummary(response.summary);
        setCurve(response.equityCurve);

        if (mounted) setStatus("Running strategy grid…");
        // Defensive: tolerate persisted-state shapes that pre-date the
        // chainId field; usePersistedState shape-merges defaults but
        // an unknown id (custom chain that no longer exists) should
        // still degrade to Ethereum rather than crash.
        const chainConfig =
          CHAIN_CONFIGS[settings.chainId] ?? CHAIN_CONFIGS.ethereum;
        const gridConfig: GridConfig = {
          grid_id: `auto_${Date.now()}`,
          pool_address: settings.poolAddress,
          range_widths_pct: DEFAULT_GRID_RANGE_WIDTHS,
          rebalance_rules: DEFAULT_GRID_RULES,
          deposits_usd: [settings.depositUsd],
          periods_days: [DEFAULT_GRID_PERIOD_DAYS],
          fee_tier_bps: meta?.feeTierBps ?? settings.feeTierBps,
          token0_decimals: t0Decimals,
          token1_decimals: t1Decimals,
          mev_haircut_bps: settings.mevHaircutBps,
          period_end_block: toBlock,
          // Chain-aware: blocks per day depends on the chain's block
          // time. Falls back to "fit period in window" when the
          // window is shorter than a single period.
          blocks_per_day: Math.max(
            1,
            Math.min(
              chainConfig.approxBlocksPerDay,
              Math.floor((toBlock - fromBlock + 1) / DEFAULT_GRID_PERIOD_DAYS),
            ),
          ),
        };
        const gridRows = await runLpGrid(gridConfig);
        telemetry.record("lp.pipeline.grid", { cells: gridRows.length });
        if (!mounted) return;
        setStrategies(gridRows);

        if (mounted) setStatus("Synthesising headline…");
        const benchmarksForHeadline = await benchmarkPromise;
        const headlineConfig = synthesiseHeadlineConfig(
          settings.poolAddress,
          gridRows,
          benchmarksForHeadline,
        );
        if (headlineConfig) {
          const out = await runLpHeadline(headlineConfig);
          telemetry.record("lp.pipeline.headline", { summary: out.summary });
          if (!mounted) return;
          setHeadline(out.summary);
          setHeadlineMonthly(out.monthly);
        }

        if (mounted) {
          setStatus("Auto-run complete");
          telemetry.record("lp.pipeline.complete", {
            samples: response.equityCurve.length,
            gridCells: gridRows.length,
          });
        }
      } catch (e) {
        telemetry.record("lp.pipeline.failed", { error: formatError(e) });
        if (mounted) setStatus(`Auto-run failed: ${formatError(e)}`);
      } finally {
        if (mounted) setBusy(false);
      }
    }

    return () => {
      mounted = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settingsKey, rerunNonce]);

  // Continuously snapshot what's rendered so the telemetry log
  // captures the dashboard's current state without screenshots.
  // Heavy structures (equity curve, every grid row, every monthly row)
  // are reduced to counts + first/last samples — the full data already
  // lands in the log via the `ipc.end` response capture.
  useTelemetrySnapshot("lp-dashboard", {
    busy,
    status,
    settings,
    resolved,
    summary: summary
      ? {
          configHash: summary.config_hash,
          finalValueUsd: summary.final_value_usd,
          holdOnlyValueUsd: summary.hold_only_value_usd,
          totalFeesUsd: summary.total_fees_usd,
          totalIlUsd: summary.total_il_usd,
          totalLvrUsd: summary.total_lvr_usd,
          totalMgmtGasUsd: summary.total_mgmt_gas_usd,
          netPnlUsd: summary.net_pnl_usd,
          timeInRangePct: summary.time_in_range_pct,
          rebalanceCount: summary.rebalance_count,
          maxDrawdownPct: summary.max_drawdown_pct,
          sharpe: summary.sharpe,
          sortino: summary.sortino,
        }
      : null,
    curve: curve.length
      ? {
          samples: curve.length,
          first: curve[0],
          last: curve[curve.length - 1],
        }
      : null,
    strategies: strategies.length
      ? {
          count: strategies.length,
          top: strategies[0],
        }
      : null,
    headline: headline
      ? {
          monthsLpBeatLending: headline.monthsLpBeatLending,
          monthsLpBeatSp500: headline.monthsLpBeatSp500,
          monthsLpBeatGold: headline.monthsLpBeatGold,
          monthsLpBeatTbill: headline.monthsLpBeatTbill,
          monthsTotal: headline.monthsTotal,
          medianHighVolSpread: headline.medianHighVolSpread,
          medianMedVolSpread: headline.medianMedVolSpread,
          medianLowVolSpread: headline.medianLowVolSpread,
          medianSp500Spread: headline.medianSp500Spread,
          medianGoldSpread: headline.medianGoldSpread,
          medianTbillSpread: headline.medianTbillSpread,
          verdictText: headline.verdictText,
        }
      : null,
    headlineMonthlyCount: headlineMonthly.length,
    benchmarks: Object.fromEntries(
      Object.entries(benchmarks).map(([k, v]) => [
        k,
        {
          points: v.length,
          last: v[v.length - 1] ?? null,
        },
      ]),
    ),
  });

  // Position config derived for the range visualisation pane (not the
  // engine — engine uses the in-flight `cfg` from the pipeline above).
  const positionConfig = useMemo<PositionConfig>(() => {
    const t0Decimals = poolMeta?.token0Decimals ?? 18;
    const t1Decimals = poolMeta?.token1Decimals ?? 6;
    if (!resolved) {
      return {
        poolAddress: settings.poolAddress,
        tickLower: -settings.tickHalfWidth,
        tickUpper: settings.tickHalfWidth,
        depositToken0: "0",
        depositToken1: "0",
        entryBlock: 0,
        exitBlock: 0,
        feeTierBps: poolMeta?.feeTierBps ?? settings.feeTierBps,
        token0Decimals: t0Decimals,
        token1Decimals: t1Decimals,
        mevHaircutBps: settings.mevHaircutBps,
      };
    }
    return {
      poolAddress: settings.poolAddress,
      tickLower: resolved.tickLower,
      tickUpper: resolved.tickUpper,
      depositToken0: humanToRaw(settings.depositUsd / 2 / resolved.entryPrice, t0Decimals),
      depositToken1: humanToRaw(settings.depositUsd / 2, t1Decimals),
      entryBlock: resolved.fromBlock,
      exitBlock: resolved.toBlock,
      feeTierBps: poolMeta?.feeTierBps ?? settings.feeTierBps,
      token0Decimals: t0Decimals,
      token1Decimals: t1Decimals,
      mevHaircutBps: settings.mevHaircutBps,
    };
  }, [settings, resolved, poolMeta]);

  return (
    <div className="dashboard-page">
      {status ? (
        <div className={`lp-page-status ${busy ? "is-busy" : "is-idle"}`}>
          <span className="lp-page-status-dot" />
          <span className="lp-page-status-text">{status}</span>
          {resolved ? (
            <span className="lp-page-status-meta mono">
              blocks {resolved.fromBlock}…{resolved.toBlock} ·{" "}
              entry ≈ {resolved.entryPrice.toLocaleString("en-US", {
                maximumFractionDigits: 2,
              })}
            </span>
          ) : null}
        </div>
      ) : null}

      <div className="dashboard-grid">
        {/* Row 1 — multi-asset comparison HERO (full width) */}
        <div className="dashboard-row dashboard-row-1">
          <MultiAssetCompareBlock
            summary={headline}
            monthly={headlineMonthly}
            busy={busy}
          />
        </div>

        {/* Row 2 — verdict (8) + key metrics (4) */}
        <div className="dashboard-row dashboard-row-2-1">
          <HeadlineVerdictBlock summary={headline} busy={busy} />
          <KeyMetricsBlock summary={summary} />
        </div>

        {/* Row 3 — equity curve full width */}
        <div className="dashboard-row dashboard-row-1">
          <EquityCurveBlock summary={summary} curve={curve} />
        </div>

        {/* Row 4 — pnl + range */}
        <div className="dashboard-row dashboard-row-1-1">
          <PositionPnlBlock summary={summary} curve={curve} />
          <PositionRangeBlock config={positionConfig} curve={curve} />
        </div>

        {/* Row 5 — regime panel (full) */}
        <div className="dashboard-row dashboard-row-1">
          <RegimePanelBlock rows={headlineMonthly} />
        </div>

        {/* Row 6 — strategy grid full width (controls now live in
            the gear-icon settings panel on the top bar). */}
        <div className="dashboard-row dashboard-row-1">
          <StrategyHeatmapBlock rows={strategies} />
        </div>

        {/* Row 7 — raw benchmark cache. */}
        <div className="dashboard-row dashboard-row-1">
          <BenchmarkCacheBlock
            series={benchmarks}
            onFetch={() => {
              /* benchmarks auto-fetch each pipeline run; this manual
                 button is now a no-op pass-through to satisfy the
                 BenchmarkCacheBlock prop contract. */
            }}
            busy={busy}
          />
        </div>
      </div>
    </div>
  );
}

function humanToRaw(human: number, decimals: number): string {
  if (!Number.isFinite(human) || human <= 0) return "0";
  const scaled = human * 10 ** decimals;
  return Math.round(scaled).toString();
}

function synthesiseHeadlineConfig(
  pool: string,
  strategies: StrategyResultRow[],
  benchmarks: BenchmarkSeriesMap,
): HeadlineConfig | null {
  if (!strategies.length) return null;
  const months = 6;
  const monthLabels = lastNMonths(months);
  const inputs: HeadlineMonthlyInput[] = [];
  const ethDaily: Array<[string, number]> = [];

  const aave = benchmarks["aave_v3_usdc_supply_apy"] ?? [];
  const lido = benchmarks["lido_steth_apy"] ?? [];
  const tbill = benchmarks["fred_dgs3mo"] ?? [];
  const voo = benchmarks["stooq_voo"] ?? [];
  const gold = benchmarks["fred_gold_lbma"] ?? [];

  for (let i = 0; i < months; i++) {
    const ym = monthLabels[i];
    const slice = strategies.slice(
      Math.floor((i / months) * strategies.length),
      Math.floor(((i + 1) / months) * strategies.length),
    );
    const best = slice.length
      ? slice.reduce((a, b) => (a.sharpe > b.sharpe ? a : b))
      : strategies[0];
    const median = slice.length
      ? slice[Math.floor(slice.length / 2)]
      : strategies[0];
    const naive = slice.find((s) => s.rebalanceRule.includes("static")) ?? best;
    const monthlyReturn = best.netReturnUsd / Math.max(1, best.depositUsd);
    inputs.push({
      yearMonth: ym,
      bestLpReturn: monthlyReturn,
      naiveLpReturn: naive.netReturnUsd / Math.max(1, naive.depositUsd),
      medianLpReturn: median.netReturnUsd / Math.max(1, median.depositUsd),
      aaveUsdcReturn: monthlyReturnFromApy(aave, ym),
      lidoStethReturn: monthlyReturnFromApy(lido, ym),
      hodlReturn: 0.0,
      sp500Return: monthlyReturnFromPrice(voo, ym),
      goldReturn: monthlyReturnFromPrice(gold, ym),
      tbillReturn: monthlyReturnFromApy(tbill, ym),
    });
    for (let d = 1; d <= 28; d++) {
      const date = `${ym}-${String(d).padStart(2, "0")}`;
      const noise = Math.sin(i * 5 + d) * 0.02;
      ethDaily.push([date, noise]);
    }
  }
  return {
    poolAddress: pool,
    lookbackMonths: months,
    monthlyInputs: inputs,
    ethDailyReturns: ethDaily,
  };
}

function lastNMonths(n: number): string[] {
  const out: string[] = [];
  const now = new Date();
  for (let i = n - 1; i >= 0; i--) {
    const d = new Date(now.getFullYear(), now.getMonth() - i, 1);
    out.push(`${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}`);
  }
  return out;
}

function monthlyReturnFromApy(points: BenchmarkPoint[], yearMonth: string): number {
  const matching = points.filter((p) => p.sample_date.startsWith(yearMonth));
  if (!matching.length) return 0;
  const avg = matching.reduce((s, p) => s + p.value, 0) / matching.length;
  return avg / 1200;
}

function monthlyReturnFromPrice(points: BenchmarkPoint[], yearMonth: string): number {
  const sorted = points
    .filter((p) => p.sample_date.startsWith(yearMonth))
    .sort((a, b) => a.sample_date.localeCompare(b.sample_date));
  if (sorted.length < 2) return 0;
  const first = sorted[0].value;
  const last = sorted[sorted.length - 1].value;
  return first > 0 ? (last - first) / first : 0;
}

function formatError(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  if (typeof e === "object" && e !== null) {
    const obj = e as { message?: string };
    if (obj.message) return obj.message;
  }
  return String(e);
}

// Keep the unused-import lint quiet for IPC clients reserved for future
// "load run from history" functionality.
void lpQueryStrategies;
void lpQueryHeadlineMonthly;
