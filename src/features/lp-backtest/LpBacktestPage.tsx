import { useCallback, useEffect, useState } from "react";

import {
  lpFetchBenchmarkSeries,
  lpGetChainHead,
  lpQueryHeadlineMonthly,
  lpQueryStrategies,
  runLpBacktest,
  runLpGrid,
  runLpHeadline,
  runLpIngestion,
  runLpSyntheticIngest,
} from "./api";
import { BenchmarkCacheBlock } from "../../components/blocks/lp/BenchmarkCacheBlock";
import { EquityCurveBlock } from "../../components/blocks/lp/EquityCurveBlock";
import { HeadlineVerdictBlock } from "../../components/blocks/lp/HeadlineVerdictBlock";
import { KeyMetricsBlock } from "../../components/blocks/lp/KeyMetricsBlock";
import { PositionPnlBlock } from "../../components/blocks/lp/PositionPnlBlock";
import { PositionRangeBlock } from "../../components/blocks/lp/PositionRangeBlock";
import { RegimePanelBlock } from "../../components/blocks/lp/RegimePanelBlock";
import {
  StrategyControlsBlock,
  type StrategyControlsState,
} from "../../components/blocks/lp/StrategyControlsBlock";
import { StrategyHeatmapBlock } from "../../components/blocks/lp/StrategyHeatmapBlock";
import {
  DEFAULT_CONTROLS,
  DEFAULT_GRID_PERIOD_DAYS,
  DEFAULT_GRID_RANGE_WIDTHS,
  DEFAULT_GRID_RULES,
} from "./defaults";
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

export function LpBacktestPage() {
  const [controls, setControls] = useState<StrategyControlsState>(DEFAULT_CONTROLS);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("");
  const [summary, setSummary] = useState<PositionRunSummary | null>(null);
  const [curve, setCurve] = useState<EquityCurvePoint[]>([]);
  const [strategies, setStrategies] = useState<StrategyResultRow[]>([]);
  const [headline, setHeadline] = useState<HeadlineRunSummary | null>(null);
  const [headlineMonthly, setHeadlineMonthly] = useState<HeadlineMonthlyRow[]>([]);
  const [benchmarks, setBenchmarks] = useState<BenchmarkSeriesMap>({});

  const positionConfigOf = useCallback(
    (state: StrategyControlsState): PositionConfig => ({
      poolAddress: state.poolAddress,
      tickLower: state.tickLower,
      tickUpper: state.tickUpper,
      depositToken0: humanToRaw(state.depositUsd / 2 / 3000, 18),
      depositToken1: humanToRaw(state.depositUsd / 2, 6),
      entryBlock: state.fromBlock,
      exitBlock: state.toBlock,
      feeTierBps: state.feeTierBps,
      token0Decimals: 18,
      token1Decimals: 6,
      mevHaircutBps: state.mevHaircutBps,
    }),
    [],
  );

  const handleRunBacktest = useCallback(
    async (silent = false) => {
      if (!silent) setBusy(true);
      setStatus("Running backtest…");
      try {
        const response = await runLpBacktest(positionConfigOf(controls), controls.rule);
        setSummary(response.summary);
        setCurve(response.equityCurve);
        setStatus("Backtest complete");
      } catch (e) {
        setStatus(`Backtest failed: ${formatError(e)}`);
      } finally {
        if (!silent) setBusy(false);
      }
    },
    [controls, positionConfigOf],
  );

  const handleSyntheticIngest = useCallback(
    async (silent = false) => {
      if (!silent) setBusy(true);
      setStatus("Generating synthetic swaps…");
      try {
        const report = await runLpSyntheticIngest(
          controls.poolAddress,
          controls.fromBlock,
          controls.toBlock,
        );
        setStatus(`Ingested ${report.swapRowsPersisted} swaps over ${controls.toBlock - controls.fromBlock + 1} blocks`);
      } catch (e) {
        setStatus(`Synthetic ingest failed: ${formatError(e)}`);
      } finally {
        if (!silent) setBusy(false);
      }
    },
    [controls.poolAddress, controls.fromBlock, controls.toBlock],
  );

  const handleLiveIngest = useCallback(async () => {
    setBusy(true);
    setStatus("Live ingest via Alchemy…");
    try {
      const report = await runLpIngestion(
        controls.poolAddress,
        controls.fromBlock,
        controls.toBlock,
      );
      setStatus(`Live ingest: ${report.swapRowsPersisted} swaps`);
    } catch (e) {
      const msg = formatError(e);
      if (msg.toLowerCase().includes("api key")) {
        setStatus("Live ingest needs MAINNET_RPC_URL or ALCHEMY_API_KEY in .env");
      } else {
        setStatus(`Live ingest failed: ${msg}`);
      }
    } finally {
      setBusy(false);
    }
  }, [controls.poolAddress, controls.fromBlock, controls.toBlock]);

  const handleRunGrid = useCallback(
    async (silent = false): Promise<StrategyResultRow[]> => {
      if (!silent) setBusy(true);
      setStatus("Running strategy grid…");
      try {
        const config: GridConfig = {
          grid_id: `auto_${Date.now()}`,
          pool_address: controls.poolAddress,
          range_widths_pct: DEFAULT_GRID_RANGE_WIDTHS,
          rebalance_rules: DEFAULT_GRID_RULES,
          deposits_usd: [controls.depositUsd],
          periods_days: [DEFAULT_GRID_PERIOD_DAYS],
          fee_tier_bps: controls.feeTierBps,
          token0_decimals: 18,
          token1_decimals: 6,
          mev_haircut_bps: controls.mevHaircutBps,
          period_end_block: controls.toBlock,
          blocks_per_day: Math.max(
            1,
            Math.floor((controls.toBlock - controls.fromBlock + 1) / DEFAULT_GRID_PERIOD_DAYS),
          ),
        };
        const rows = await runLpGrid(config);
        setStrategies(rows);
        setStatus(`Grid: ${rows.length} cells`);
        return rows;
      } catch (e) {
        setStatus(`Grid failed: ${formatError(e)}`);
        return [];
      } finally {
        if (!silent) setBusy(false);
      }
    },
    [controls],
  );

  const handleSynthesiseHeadline = useCallback(
    async (gridRows: StrategyResultRow[] | null = null, silent = false) => {
      if (!silent) setBusy(true);
      setStatus("Synthesising headline…");
      try {
        const rows = gridRows ?? strategies;
        if (!rows.length) {
          setStatus("Grid empty — run grid first");
          return;
        }
        const config = synthesiseHeadlineConfig(controls.poolAddress, rows);
        if (!config) {
          setStatus("Could not synthesise — grid empty");
          return;
        }
        const out = await runLpHeadline(config);
        setHeadline(out.summary);
        setHeadlineMonthly(out.monthly);
        setStatus("Headline synthesised");
      } catch (e) {
        setStatus(`Headline failed: ${formatError(e)}`);
      } finally {
        if (!silent) setBusy(false);
      }
    },
    [controls.poolAddress, strategies],
  );

  const handleFetchBenchmarks = useCallback(async () => {
    setBusy(true);
    setStatus("Fetching benchmark series…");
    try {
      const series = ["aave_v3_usdc_supply_apy", "lido_steth_apy", "fred_dgs3mo", "stooq_voo"];
      const results = await Promise.allSettled(series.map((s) => lpFetchBenchmarkSeries(s)));
      const next: BenchmarkSeriesMap = {};
      let ok = 0;
      results.forEach((r, i) => {
        if (r.status === "fulfilled") {
          next[series[i]] = r.value;
          ok += 1;
        } else {
          next[series[i]] = [];
        }
      });
      setBenchmarks(next);
      setStatus(`Benchmarks fetched: ${ok}/${series.length} series`);
    } catch (e) {
      setStatus(`Benchmark fetch failed: ${formatError(e)}`);
    } finally {
      setBusy(false);
    }
  }, []);

  // Auto-run on mount: chain-head fetch → synthetic ingest → backtest →
  // grid → headline. The pipeline is fully idempotent (storage uses
  // INSERT OR IGNORE for swap rows + config_hash for runs/headline) so
  // we DON'T guard against StrictMode's double-mount. The previous
  // `initialised.current` ref guard caused a real bug: StrictMode would
  // unmount mid-flight, set the cleanup's `cancelled = true`, and the
  // remount would skip the effect entirely (initialised was already
  // true) — leaving the dashboard stuck in busy=true forever.
  //
  // Every state setter is gated on a per-closure `mounted` flag, so the
  // cancelled run's stale state updates simply no-op (idempotent at the
  // React level) and the second StrictMode run drives state to
  // completion. In production (no double-mount) it just runs once.
  useEffect(() => {
    let mounted = true;
    void runPipeline();

    async function runPipeline() {
      try {
        // eslint-disable-next-line no-console
        console.log("[lp] auto-run: starting");
        if (mounted) setBusy(true);
        if (mounted) setStatus("Resolving recent block window…");

        // Default the block window to "the last 1000 blocks of the
        // chain". The synthetic ingest then generates fake swaps over
        // that range — block numbers feel current and the same window
        // is reusable for live ingest later. Falls back to the static
        // DEFAULT_CONTROLS window when no Alchemy key is configured.
        let toBlock = DEFAULT_CONTROLS.toBlock;
        let fromBlock = DEFAULT_CONTROLS.fromBlock;
        try {
          const head = await lpGetChainHead();
          // eslint-disable-next-line no-console
          console.log("[lp] auto-run: chain head =", head);
          if (typeof head === "number" && head > 1000) {
            toBlock = head;
            fromBlock = head - 1000;
          }
        } catch (e) {
          // eslint-disable-next-line no-console
          console.log(
            "[lp] auto-run: chain head fetch failed (using fallback) —",
            e,
          );
        }
        if (!mounted) return;

        const resolvedControls = {
          ...DEFAULT_CONTROLS,
          fromBlock,
          toBlock,
        };
        if (mounted) setControls(resolvedControls);

        if (mounted) setStatus("Running synthetic ingest…");
        const ingestReport = await runLpSyntheticIngest(
          resolvedControls.poolAddress,
          resolvedControls.fromBlock,
          resolvedControls.toBlock,
        );
        // eslint-disable-next-line no-console
        console.log("[lp] auto-run: synthetic ingest →", ingestReport);
        if (!mounted) return;

        if (mounted) setStatus("Running backtest…");
        const cfg = positionConfigOf(resolvedControls);
        const response = await runLpBacktest(cfg, resolvedControls.rule);
        // eslint-disable-next-line no-console
        console.log(
          "[lp] auto-run: backtest →",
          response.summary,
          response.equityCurve.length,
          "samples",
        );
        if (!mounted) return;
        setSummary(response.summary);
        setCurve(response.equityCurve);

        if (mounted) setStatus("Running strategy grid…");
        const gridConfig: GridConfig = {
          grid_id: `auto_${Date.now()}`,
          pool_address: resolvedControls.poolAddress,
          range_widths_pct: DEFAULT_GRID_RANGE_WIDTHS,
          rebalance_rules: DEFAULT_GRID_RULES,
          deposits_usd: [resolvedControls.depositUsd],
          periods_days: [DEFAULT_GRID_PERIOD_DAYS],
          fee_tier_bps: resolvedControls.feeTierBps,
          token0_decimals: 18,
          token1_decimals: 6,
          mev_haircut_bps: resolvedControls.mevHaircutBps,
          period_end_block: resolvedControls.toBlock,
          blocks_per_day: Math.max(
            1,
            Math.floor(
              (resolvedControls.toBlock - resolvedControls.fromBlock + 1) /
                DEFAULT_GRID_PERIOD_DAYS,
            ),
          ),
        };
        const gridRows = await runLpGrid(gridConfig);
        // eslint-disable-next-line no-console
        console.log("[lp] auto-run: grid →", gridRows.length, "cells");
        if (!mounted) return;
        setStrategies(gridRows);

        if (mounted) setStatus("Synthesising headline…");
        const headlineConfig = synthesiseHeadlineConfig(
          resolvedControls.poolAddress,
          gridRows,
        );
        if (headlineConfig) {
          const out = await runLpHeadline(headlineConfig);
          // eslint-disable-next-line no-console
          console.log("[lp] auto-run: headline →", out.summary);
          if (!mounted) return;
          setHeadline(out.summary);
          setHeadlineMonthly(out.monthly);
        }

        if (mounted) {
          setStatus("Auto-run complete");
          // eslint-disable-next-line no-console
          console.log("[lp] auto-run: COMPLETE");
        }
      } catch (e) {
        // eslint-disable-next-line no-console
        console.error("[lp] auto-run: FAILED —", e);
        if (mounted) setStatus(`Auto-run failed: ${formatError(e)}`);
      } finally {
        if (mounted) setBusy(false);
      }
    }

    return () => {
      mounted = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const positionConfig = positionConfigOf(controls);

  return (
    <div className="dashboard-page">
      <div className="dashboard-grid">
        {/* Row 1 — verdict (8) + key metrics (4) */}
        <div className="dashboard-row dashboard-row-2-1">
          <HeadlineVerdictBlock summary={headline} busy={busy} />
          <KeyMetricsBlock summary={summary} />
        </div>

        {/* Row 2 — equity curve full width */}
        <div className="dashboard-row dashboard-row-1">
          <EquityCurveBlock summary={summary} curve={curve} />
        </div>

        {/* Row 3 — pnl + range full split */}
        <div className="dashboard-row dashboard-row-1-1">
          <PositionPnlBlock summary={summary} curve={curve} />
          <PositionRangeBlock config={positionConfig} curve={curve} />
        </div>

        {/* Row 4 — controls (4) + strategy grid (8) */}
        <div className="dashboard-row dashboard-row-1-2">
          <StrategyControlsBlock
            state={controls}
            onChange={setControls}
            onRunBacktest={() => handleRunBacktest()}
            onRunSyntheticIngest={() => handleSyntheticIngest()}
            onRunLiveIngest={handleLiveIngest}
            onRunGrid={() => handleRunGrid()}
            onSynthesiseHeadline={() => handleSynthesiseHeadline()}
            busy={busy}
            status={status}
          />
          <StrategyHeatmapBlock rows={strategies} />
        </div>

        {/* Row 5 — regime panel (8) + benchmarks (4) */}
        <div className="dashboard-row dashboard-row-2-1">
          <RegimePanelBlock rows={headlineMonthly} />
          <BenchmarkCacheBlock
            series={benchmarks}
            onFetch={handleFetchBenchmarks}
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
): HeadlineConfig | null {
  if (!strategies.length) return null;
  const months = 6;
  const inputs: HeadlineMonthlyInput[] = [];
  const ethDaily: Array<[string, number]> = [];
  for (let i = 0; i < months; i++) {
    const ym = `2024-${String(i + 1).padStart(2, "0")}`;
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
      aaveUsdcReturn: 0.005,
      lidoStethReturn: 0.0035,
      hodlReturn: 0.0,
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

function formatError(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  if (typeof e === "object" && e !== null) {
    const obj = e as { message?: string };
    if (obj.message) return obj.message;
  }
  return String(e);
}

// Mark referenced helpers as used
void lpQueryStrategies;
void lpQueryHeadlineMonthly;
