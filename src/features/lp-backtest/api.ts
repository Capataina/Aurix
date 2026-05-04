import { loggedInvoke as invoke } from "../../lib/telemetry";

import type {
  BacktestResponse,
  BenchmarkPoint,
  FirstSwapInfo,
  GridConfig,
  HeadlineConfig,
  HeadlineMonthlyRow,
  HeadlineOutput,
  IngestionReport,
  PoolMetadata,
  PositionConfig,
  RebalanceRule,
  StrategyResultRow,
} from "./types";

/**
 * Returns the latest finalized block on Ethereum mainnet via Alchemy.
 * Used to default the LP backtester's block window to a "last N blocks"
 * rolling range. Throws `KeyRequired` when no key is configured —
 * caller falls back to a static default in that case.
 */
export function lpGetChainHead(chainId?: string): Promise<number> {
  return invoke<number>("lp_get_chain_head", { chainId });
}

/** Live archive ingestion. Tries subgraph → user-Alchemy → public RPC.
 *  Errors when none can serve the range. */
export function runLpIngestion(
  poolAddress: string,
  fromBlock: number,
  toBlock: number,
  chainId?: string,
  protocol?: string,
): Promise<IngestionReport> {
  return invoke<IngestionReport>("run_lp_ingestion", {
    poolAddress,
    fromBlock,
    toBlock,
    chainId,
    protocol,
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

/**
 * Fetches pool metadata via the Uniswap V3 subgraph: token0/token1
 * addresses + symbols + decimals + fee tier. Returns a flag for
 * whether token1 is USD-pegged so the frontend can default the
 * deposit-split logic without hardcoded decimals.
 */
export function lpPoolMetadata(
  poolAddress: string,
  chainId?: string,
  protocol?: string,
): Promise<PoolMetadata> {
  return invoke<PoolMetadata>("lp_pool_metadata", {
    poolAddress,
    chainId,
    protocol,
  });
}

/** USD spot prices for the supplied token addresses on a chain.
 *  Backed by DefiLlama's free coins API — no auth required. Used to
 *  value non-USD-quote pool positions (WBTC/ETH, LDO/ETH, etc.). */
export function lpTokenUsdPrices(
  addresses: string[],
  chainId?: string,
): Promise<{ prices: Record<string, number> }> {
  return invoke<{ prices: Record<string, number> }>("lp_token_usd_prices", {
    addresses,
    chainId,
  });
}

/**
 * After ingesting swaps, query the first swap chronologically inside
 * `[fromBlock, toBlock]` to derive realised entry tick + price. The
 * frontend uses these to set the position's tickLower/tickUpper and to
 * split the deposit by the actual market price — no /3000 hardcode.
 *
 * Returns null when the range has no swaps (empty pool / wrong range).
 */
export function lpQueryFirstSwapPrice(
  poolAddress: string,
  fromBlock: number,
  toBlock: number,
  token0Decimals: number,
  token1Decimals: number,
): Promise<FirstSwapInfo | null> {
  return invoke<FirstSwapInfo | null>("lp_query_first_swap_price", {
    poolAddress,
    fromBlock,
    toBlock,
    token0Decimals,
    token1Decimals,
  });
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
