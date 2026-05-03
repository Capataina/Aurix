// Recommended defaults for Tab 2 — chosen so the auto-run pipeline
// produces a meaningful first-render result without the user having to
// configure anything. Numbers tuned against the synthetic swap stream
// the synthetic_ingest command generates (sinusoidal tick walk between
// -300 and +300 over the supplied block range).

import type { RebalanceRule } from "./types";
import type { StrategyControlsState } from "../../components/blocks/StrategyControlsBlock";

/** Uniswap V3 5bps WETH/USDC pool — the canonical pair we monitor. */
export const DEFAULT_POOL = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640";

/** 1000-block synthetic window — enough for Sharpe to be non-trivial. */
export const DEFAULT_FROM_BLOCK = 1_000;
export const DEFAULT_TO_BLOCK = 2_000;

/** ±300 ticks ≈ ±3% range on price. Sits well inside the synthetic
 *  walk's [-300, +300] envelope so time-in-range stays high. */
export const DEFAULT_TICK_LOWER = -300;
export const DEFAULT_TICK_UPPER = 300;

export const DEFAULT_DEPOSIT_USD = 10_000;
export const DEFAULT_FEE_TIER_BPS = 5;
export const DEFAULT_MEV_HAIRCUT_BPS = 5;
export const DEFAULT_RULE: RebalanceRule = { kind: "static" };

export const DEFAULT_CONTROLS: StrategyControlsState = {
  poolAddress: DEFAULT_POOL,
  fromBlock: DEFAULT_FROM_BLOCK,
  toBlock: DEFAULT_TO_BLOCK,
  tickLower: DEFAULT_TICK_LOWER,
  tickUpper: DEFAULT_TICK_UPPER,
  depositUsd: DEFAULT_DEPOSIT_USD,
  feeTierBps: DEFAULT_FEE_TIER_BPS,
  mevHaircutBps: DEFAULT_MEV_HAIRCUT_BPS,
  rule: DEFAULT_RULE,
};

/** Grid axes — 12 cells = 4 widths × 3 rules × 1 deposit × 1 period.
 *  Small enough to run in <2 seconds, varied enough that the heatmap
 *  has clear structure. */
export const DEFAULT_GRID_RANGE_WIDTHS = [0.5, 1.0, 2.5, 5.0];
export const DEFAULT_GRID_RULES: RebalanceRule[] = [
  { kind: "static" },
  { kind: "schedule", every_n_blocks: 100 },
  { kind: "price_exit_threshold", central_pct: 0.5 },
];
export const DEFAULT_GRID_PERIOD_DAYS = 30;
