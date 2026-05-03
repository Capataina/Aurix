import { useCallback, useState } from "react";

import {
  lpFetchBenchmarkSeries,
  lpQueryBenchmarkRange,
  lpQueryHeadlineMonthly,
  lpQueryStrategies,
  runLpBacktest,
  runLpGrid,
  runLpHeadline,
  runLpIngestion,
  runLpSyntheticIngest,
} from "./api";
import { EquityCurveBlock } from "./components/EquityCurveBlock";
import { HeadlineVerdictBlock } from "./components/HeadlineVerdictBlock";
import { RegimePanelBlock } from "./components/RegimePanelBlock";
import {
  StrategyControlsBlock,
  type StrategyControlsState,
} from "./components/StrategyControlsBlock";
import { StrategyHeatmapBlock } from "./components/StrategyHeatmapBlock";
import type {
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

const DEFAULT_POOL = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640";

const DEFAULT_CONTROLS: StrategyControlsState = {
  poolAddress: DEFAULT_POOL,
  fromBlock: 1000,
  toBlock: 1500,
  tickLower: -300,
  tickUpper: 300,
  depositUsd: 10_000,
  feeTierBps: 5,
  mevHaircutBps: 5,
  rule: { kind: "static" },
};

interface LpBacktestPageProps {
  drawerOpen: boolean;
  onCloseDrawer: () => void;
}

export function LpBacktestPage({ drawerOpen, onCloseDrawer }: LpBacktestPageProps) {
  const [controls, setControls] = useState<StrategyControlsState>(DEFAULT_CONTROLS);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("");
  const [summary, setSummary] = useState<PositionRunSummary | null>(null);
  const [curve, setCurve] = useState<EquityCurvePoint[]>([]);
  const [strategies, setStrategies] = useState<StrategyResultRow[]>([]);
  const [headline, setHeadline] = useState<HeadlineRunSummary | null>(null);
  const [headlineMonthly, setHeadlineMonthly] = useState<HeadlineMonthlyRow[]>([]);

  const positionConfig = useCallback(
    (): PositionConfig => ({
      poolAddress: controls.poolAddress,
      tickLower: controls.tickLower,
      tickUpper: controls.tickUpper,
      depositToken0: humanToRaw(controls.depositUsd / 2 / 3000, 18),
      depositToken1: humanToRaw(controls.depositUsd / 2, 6),
      entryBlock: controls.fromBlock,
      exitBlock: controls.toBlock,
      feeTierBps: controls.feeTierBps,
      token0Decimals: 18,
      token1Decimals: 6,
      mevHaircutBps: controls.mevHaircutBps,
    }),
    [controls],
  );

  const handleRunBacktest = useCallback(async () => {
    setBusy(true);
    setStatus("Running backtest…");
    try {
      const response = await runLpBacktest(positionConfig(), controls.rule);
      setSummary(response.summary);
      setCurve(response.equityCurve);
      setStatus("Backtest complete");
    } catch (e) {
      console.error(e);
      setStatus(`Backtest failed: ${formatError(e)}`);
    } finally {
      setBusy(false);
    }
  }, [controls.rule, positionConfig]);

  const handleSyntheticIngest = useCallback(async () => {
    setBusy(true);
    setStatus("Generating synthetic swaps…");
    try {
      const report = await runLpSyntheticIngest(
        controls.poolAddress,
        controls.fromBlock,
        controls.toBlock,
      );
      setStatus(
        `Synthetic ingest: ${report.swapRowsPersisted} swaps over ${
          controls.toBlock - controls.fromBlock + 1
        } blocks`,
      );
    } catch (e) {
      setStatus(`Synthetic ingest failed: ${formatError(e)}`);
    } finally {
      setBusy(false);
    }
  }, [controls.poolAddress, controls.fromBlock, controls.toBlock]);

  const handleLiveIngest = useCallback(async () => {
    setBusy(true);
    setStatus("Live ingest via Alchemy…");
    try {
      const report = await runLpIngestion(
        controls.poolAddress,
        controls.fromBlock,
        controls.toBlock,
      );
      setStatus(
        `Live ingest: ${report.swapRowsPersisted} swaps + ${report.poolEventRowsPersisted} pool events`,
      );
    } catch (e) {
      const msg = formatError(e);
      if (msg.includes("api key required")) {
        setStatus(
          "Live ingest needs MAINNET_RPC_URL or ALCHEMY_API_KEY. Use Synthetic ingest instead.",
        );
      } else {
        setStatus(`Live ingest failed: ${msg}`);
      }
    } finally {
      setBusy(false);
    }
  }, [controls.poolAddress, controls.fromBlock, controls.toBlock]);

  const handleRunGrid = useCallback(async () => {
    setBusy(true);
    setStatus("Running grid (range × rule × deposit × period)…");
    try {
      const config: GridConfig = {
        grid_id: `grid_${Date.now()}`,
        pool_address: controls.poolAddress,
        range_widths_pct: [0.5, 1.0, 2.5, 5.0],
        rebalance_rules: [
          { kind: "static" },
          { kind: "schedule", every_n_blocks: 100 },
          { kind: "price_exit_threshold", central_pct: 0.5 },
          { kind: "out_of_range_duration", min_oor_blocks: 50 },
        ],
        deposits_usd: [controls.depositUsd],
        periods_days: [30],
        fee_tier_bps: controls.feeTierBps,
        token0_decimals: 18,
        token1_decimals: 6,
        mev_haircut_bps: controls.mevHaircutBps,
        period_end_block: controls.toBlock,
        blocks_per_day: Math.max(1, Math.floor((controls.toBlock - controls.fromBlock + 1) / 30)),
      };
      const rows = await runLpGrid(config);
      setStrategies(rows);
      setStatus(`Grid complete: ${rows.length} cells. Storing.`);
    } catch (e) {
      setStatus(`Grid failed: ${formatError(e)}`);
    } finally {
      setBusy(false);
    }
  }, [
    controls.poolAddress,
    controls.fromBlock,
    controls.toBlock,
    controls.depositUsd,
    controls.feeTierBps,
    controls.mevHaircutBps,
  ]);

  const handleSynthesiseHeadline = useCallback(async () => {
    setBusy(true);
    setStatus("Synthesising headline (M2.8)…");
    try {
      const config = synthesiseHeadlineConfig(controls.poolAddress, strategies);
      if (!config) {
        setStatus("Run the grid first — headline needs strategy results.");
        return;
      }
      const out = await runLpHeadline(config);
      setHeadline(out.summary);
      setHeadlineMonthly(out.monthly);
      setStatus("Headline synthesised");
    } catch (e) {
      setStatus(`Headline failed: ${formatError(e)}`);
    } finally {
      setBusy(false);
    }
  }, [controls.poolAddress, strategies]);

  const handleFetchBenchmarks = useCallback(async () => {
    setBusy(true);
    setStatus("Fetching benchmark series (DefiLlama + FRED + Stooq)…");
    try {
      const series = ["aave_v3_usdc_supply_apy", "lido_steth_apy", "fred_dgs3mo"];
      const results = await Promise.all(series.map((s) => lpFetchBenchmarkSeries(s)));
      const total = results.reduce((sum, arr) => sum + arr.length, 0);
      setStatus(`Benchmarks fetched: ${total} points across ${series.length} series.`);
    } catch (e) {
      setStatus(`Benchmark fetch failed: ${formatError(e)}`);
    } finally {
      setBusy(false);
    }
  }, []);

  return (
    <div className={`lp-page ${drawerOpen ? "is-drawer-open" : ""}`}>
      <div className="lp-grid">
        <div className="lp-row">
          <StrategyControlsBlock
            state={controls}
            onChange={setControls}
            onRunBacktest={handleRunBacktest}
            onRunSyntheticIngest={handleSyntheticIngest}
            onRunLiveIngest={handleLiveIngest}
            busy={busy}
            status={status}
          />
        </div>
        <div className="lp-row">
          <EquityCurveBlock summary={summary} curve={curve} />
        </div>
        <div className="lp-row">
          <HeadlineVerdictBlock
            summary={headline}
            onRunHeadline={handleSynthesiseHeadline}
            busy={busy}
          />
        </div>
        <div className="lp-row">
          <StrategyHeatmapBlock
            rows={strategies}
            onRunGrid={handleRunGrid}
            busy={busy}
          />
        </div>
        <div className="lp-row">
          <RegimePanelBlock rows={headlineMonthly} />
        </div>
        <div className="lp-row">
          <div className="lp-utility">
            <button
              type="button"
              className="lp-button"
              onClick={handleFetchBenchmarks}
              disabled={busy}
              title="Fetches DefiLlama Aave/Lido APYs and FRED 3-month T-bill rate."
            >
              Fetch benchmarks
            </button>
            <button
              type="button"
              className="lp-button"
              onClick={async () => {
                if (!summary) return;
                const stored = await lpQueryStrategies(`grid_${summary.completed_at_unix_ms}`);
                setStrategies(stored);
                setStatus(`Loaded ${stored.length} cached strategy rows`);
              }}
              disabled={busy}
            >
              Reload last grid
            </button>
            <button
              type="button"
              className="lp-button"
              onClick={async () => {
                if (!headline) return;
                const monthly = await lpQueryHeadlineMonthly(headline.config_hash);
                setHeadlineMonthly(monthly);
                setStatus(`Loaded ${monthly.length} cached headline rows`);
              }}
              disabled={busy}
            >
              Reload headline
            </button>
            <button
              type="button"
              className="lp-button"
              onClick={async () => {
                const aave = await lpQueryBenchmarkRange(
                  "aave_v3_usdc_supply_apy",
                  "2024-01-01",
                  "2025-12-31",
                );
                setStatus(`Aave APY rows in cache: ${aave.length}`);
              }}
              disabled={busy}
            >
              Inspect benchmark cache
            </button>
            {drawerOpen ? (
              <button
                type="button"
                className="lp-button"
                onClick={onCloseDrawer}
              >
                Close drawer
              </button>
            ) : null}
          </div>
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
  // Bucket strategies into 6 synthetic months by index. Real headline
  // expects per-month best LP from a 24-month grid; this is a demo
  // synthesis using whatever cells we have.
  const months = 6;
  const inputs: HeadlineMonthlyInput[] = [];
  const ethDaily: Array<[string, number]> = [];
  for (let i = 0; i < months; i++) {
    const ym = `2024-${String(i + 1).padStart(2, "0")}`;
    const slice = strategies.slice(
      Math.floor((i / months) * strategies.length),
      Math.floor(((i + 1) / months) * strategies.length),
    );
    const best = slice.length ? slice.reduce((a, b) => (a.sharpe > b.sharpe ? a : b)) : strategies[0];
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
    const obj = e as { message?: string; keyRequired?: string | null };
    if (obj.message) return obj.message;
  }
  return String(e);
}
