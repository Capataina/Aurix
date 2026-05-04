// TypeScript mirrors of the Rust IPC payloads. Names match the Rust
// structs after `#[serde(rename_all = "camelCase")]`. Keep this file
// in lockstep with the Rust side — any rename on either side without a
// matching update produces silent `undefined` reads at runtime.

export interface PositionConfig {
  poolAddress: string;
  tickLower: number;
  tickUpper: number;
  depositToken0: string;
  depositToken1: string;
  entryBlock: number;
  exitBlock: number;
  feeTierBps: number;
  token0Decimals: number;
  token1Decimals: number;
  mevHaircutBps: number;
  /** Per-token USD prices (DefiLlama). Optional — when both are
   *  supplied, the engine values the position via per-token feeds
   *  instead of assuming token1 is USD-pegged. */
  token0UsdPrice?: number | null;
  token1UsdPrice?: number | null;
}

export type RebalanceRule =
  | { kind: "static" }
  | { kind: "schedule"; every_n_blocks: number }
  | { kind: "price_exit_threshold"; central_pct: number }
  | { kind: "out_of_range_duration"; min_oor_blocks: number };

export interface PositionRunSummary {
  config_hash: string;
  pool_address: string;
  tick_lower: number;
  tick_upper: number;
  deposit_token0: string;
  deposit_token1: string;
  entry_block: number;
  exit_block: number;
  rebalance_rule: string;
  mev_haircut_bps: number;
  total_fees_usd: number;
  total_il_usd: number;
  total_lvr_usd: number;
  total_mgmt_gas_usd: number;
  final_value_usd: number;
  hold_only_value_usd: number;
  net_pnl_usd: number;
  time_in_range_pct: number;
  rebalance_count: number;
  max_drawdown_pct: number;
  sharpe: number;
  sortino: number;
  calmar: number;
  completed_at_unix_ms: number;
}

export interface EquityCurvePoint {
  sample_idx: number;
  block_number: number;
  block_timestamp: number;
  position_value_usd: number;
  fees_accumulated_usd: number;
  il_usd: number;
  lvr_usd: number;
  mgmt_gas_paid_usd: number;
  hold_only_value_usd: number;
  net_pnl_usd: number;
  in_range: boolean;
}

export interface BacktestResponse {
  summary: PositionRunSummary;
  equityCurve: EquityCurvePoint[];
}

export interface IngestionReport {
  poolAddress: string;
  fromBlock: number;
  toBlock: number;
  swapRowsPersisted: number;
  poolEventRowsPersisted: number;
  gasRowsPersisted: number;
}

export interface GridConfig {
  grid_id: string;
  pool_address: string;
  range_widths_pct: number[];
  rebalance_rules: RebalanceRule[];
  deposits_usd: number[];
  periods_days: number[];
  fee_tier_bps: number;
  token0_decimals: number;
  token1_decimals: number;
  mev_haircut_bps: number;
  period_end_block: number;
  blocks_per_day: number;
}

export interface StrategyResultRow {
  gridId: string;
  poolAddress: string;
  rangeWidthPct: number;
  rebalanceRule: string;
  depositUsd: number;
  periodDays: number;
  periodStartUnixMs: number;
  periodEndUnixMs: number;
  feesUsd: number;
  ilUsd: number;
  lvrUsd: number;
  mgmtGasUsd: number;
  netReturnUsd: number;
  netReturnVsHold: number;
  timeInRangePct: number;
  rebalanceCount: number;
  maxDrawdownPct: number;
  sharpe: number;
  sortino: number;
  calmar: number;
  deflatedSharpe: number;
  completedAtUnixMs: number;
}

export interface HeadlineMonthlyInput {
  yearMonth: string;
  bestLpReturn: number;
  naiveLpReturn: number;
  medianLpReturn: number;
  aaveUsdcReturn: number;
  lidoStethReturn: number;
  hodlReturn: number;
  sp500Return: number;
  goldReturn: number;
  tbillReturn: number;
}

export interface FirstSwapInfo {
  blockNumber: number;
  blockTimestamp: number;
  tick: number;
  sqrtPriceX96: string;
  humanPrice: number;
}

export interface PoolMetadata {
  poolAddress: string;
  token0Address: string;
  token0Symbol: string;
  token0Decimals: number;
  token1Address: string;
  token1Symbol: string;
  token1Decimals: number;
  feeTierBps: number;
  isToken1UsdPegged: boolean;
}

export interface HeadlineConfig {
  poolAddress: string;
  lookbackMonths: number;
  monthlyInputs: HeadlineMonthlyInput[];
  ethDailyReturns: Array<[string, number]>;
}

export interface HeadlineRunSummary {
  configHash: string;
  poolAddress: string;
  lookbackMonths: number;
  regimeMethod: string;
  monthsLpBeatLending: number;
  monthsLpBeatSp500: number;
  monthsLpBeatGold: number;
  monthsLpBeatTbill: number;
  monthsTotal: number;
  medianLowVolSpread: number | null;
  medianMedVolSpread: number | null;
  medianHighVolSpread: number | null;
  medianSp500Spread: number | null;
  medianGoldSpread: number | null;
  medianTbillSpread: number | null;
  verdictText: string;
  completedAtUnixMs: number;
}

export interface HeadlineMonthlyRow {
  yearMonth: string;
  volRegime: string;
  bestLpReturn: number;
  naiveLpReturn: number;
  medianLpReturn: number;
  aaveUsdcReturn: number;
  lidoStethReturn: number;
  hodlReturn: number;
  sp500Return: number;
  goldReturn: number;
  tbillReturn: number;
  ethVol30d: number;
}

export interface HeadlineOutput {
  summary: HeadlineRunSummary;
  monthly: HeadlineMonthlyRow[];
}

export interface BenchmarkPoint {
  series_key: string;
  sample_date: string;
  value: number;
  source: string;
  fetched_at_unix_ms: number;
}

export interface CommandError {
  message: string;
  keyRequired: string | null;
}
