import { invoke } from "@tauri-apps/api/core";

import type {
  BacktestResponse,
  BenchmarkPoint,
  GridConfig,
  HeadlineConfig,
  HeadlineMonthlyRow,
  HeadlineOutput,
  IngestionReport,
  PositionConfig,
  RebalanceRule,
  StrategyResultRow,
} from "./types";

/** Live archive ingestion via Alchemy (KEY_REQUIRED). */
export function runLpIngestion(
  poolAddress: string,
  fromBlock: number,
  toBlock: number,
): Promise<IngestionReport> {
  return invoke<IngestionReport>("run_lp_ingestion", {
    poolAddress,
    fromBlock,
    toBlock,
  });
}

/** Synthetic ingestion — sinusoidal swap stream for demo without keys. */
export function runLpSyntheticIngest(
  poolAddress: string,
  fromBlock: number,
  toBlock: number,
): Promise<IngestionReport> {
  return invoke<IngestionReport>("run_lp_synthetic_ingest", {
    poolAddress,
    fromBlock,
    toBlock,
  });
}

export function runLpBacktest(
  config: PositionConfig,
  rule: RebalanceRule,
): Promise<BacktestResponse> {
  return invoke<BacktestResponse>("run_lp_backtest", { config, rule });
}

export function runLpGrid(config: GridConfig): Promise<StrategyResultRow[]> {
  return invoke<StrategyResultRow[]>("run_lp_grid", { config });
}

export function runLpHeadline(
  config: HeadlineConfig,
): Promise<HeadlineOutput> {
  return invoke<HeadlineOutput>("run_lp_headline", { config });
}

export function lpFetchBenchmarkSeries(
  seriesKey: string,
): Promise<BenchmarkPoint[]> {
  return invoke<BenchmarkPoint[]>("lp_fetch_benchmark_series", { seriesKey });
}

export function lpQueryBenchmarkRange(
  seriesKey: string,
  startDate: string,
  endDate: string,
): Promise<BenchmarkPoint[]> {
  return invoke<BenchmarkPoint[]>("lp_query_benchmark_range", {
    seriesKey,
    startDate,
    endDate,
  });
}

export function lpQueryStrategies(
  gridId: string,
): Promise<StrategyResultRow[]> {
  return invoke<StrategyResultRow[]>("lp_query_strategies", { gridId });
}

export function lpQueryHeadlineMonthly(
  configHash: string,
): Promise<HeadlineMonthlyRow[]> {
  return invoke<HeadlineMonthlyRow[]>("lp_query_headline_monthly", {
    configHash,
  });
}
