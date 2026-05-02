import type { PriceSnapshot } from "../features/arbitrage/types";

import { GAS_UNITS_ESTIMATE } from "./config";
import { median } from "./stats";

/**
 * Which costs to subtract when reporting a route's "net".
 *
 * - `gas`           — net = (sellPrice − buyPrice) − gas. The cheap, optimistic
 *                      view. Useful for spotting raw venue disagreement.
 * - `gas-and-fees`  — net = sellPrice·(1−sellFee) − buyPrice/(1−buyFee) − gas.
 *                      The honest view. Pool fees are the dominant cost on
 *                      tight spreads and the reason most "obvious" arbs
 *                      aren't real.
 *
 * Both modes are derived from the same history on every render — switching
 * the mode is a pure presentation change and never invalidates state.
 */
export type PnlMode = "gas" | "gas-and-fees";

export const PNL_MODE_OPTIONS: Array<{ value: PnlMode; label: string }> = [
  { value: "gas", label: "Gas-adjusted" },
  { value: "gas-and-fees", label: "Gas + pool fees" },
];

/** The truthful default — pool fees are real costs, hiding them is misleading. */
export const DEFAULT_PNL_MODE: PnlMode = "gas-and-fees";

export interface RouteAnalysis {
  buy: PriceSnapshot;
  sell: PriceSnapshot;
  buyIndex: number;
  sellIndex: number;
  /** Sell price minus buy price (raw spread for this pair). */
  spreadUsd: number;
  /** Estimated gas cost (USD) for the round trip. */
  gasCostUsd: number;
  /**
   * USDC required to acquire 1 unit of base at the buy venue, accounting for
   * the buy-side pool fee. Equals `buy.priceUsd` exactly in `gas` mode.
   */
  buyCostUsd: number;
  /**
   * USDC received from selling 1 unit of base at the sell venue, accounting
   * for the sell-side pool fee. Equals `sell.priceUsd` exactly in `gas` mode.
   */
  sellProceedsUsd: number;
  /** Pool fee on the buy leg in USD. Zero in `gas` mode. */
  buyFeeUsd: number;
  /** Pool fee on the sell leg in USD. Zero in `gas` mode. */
  sellFeeUsd: number;
  /** Combined pool fee. Zero in `gas` mode. */
  feeCostUsd: number;
  /** Realised P/L per 1 unit of base for this route under the active mode. */
  netUsd: number;
}

export function computeGasCostUsd(
  gasPriceGwei: number,
  referencePriceUsd: number,
  gasUnits: number = GAS_UNITS_ESTIMATE,
): number {
  return (gasPriceGwei * gasUnits * referencePriceUsd) / 1_000_000_000;
}

/**
 * P/L of one buy → sell route. The fee math models the standard Uniswap-style
 * AMM convention: the pool keeps `fee` of the input token, so
 *   buy:  pay = price / (1 − feeBuy)   to receive 1 base
 *   sell: get = price · (1 − feeSell)  for sending 1 base
 */
export function computeRoute(
  buy: PriceSnapshot,
  sell: PriceSnapshot,
  buyIndex: number,
  sellIndex: number,
  gasCostUsd: number,
  mode: PnlMode,
): RouteAnalysis {
  const spreadUsd = sell.priceUsd - buy.priceUsd;

  if (mode === "gas") {
    return {
      buy,
      sell,
      buyIndex,
      sellIndex,
      spreadUsd,
      gasCostUsd,
      buyCostUsd: buy.priceUsd,
      sellProceedsUsd: sell.priceUsd,
      buyFeeUsd: 0,
      sellFeeUsd: 0,
      feeCostUsd: 0,
      netUsd: spreadUsd - gasCostUsd,
    };
  }

  const buyFeeRate = buy.feeTierBps / 10_000;
  const sellFeeRate = sell.feeTierBps / 10_000;
  const buyCostUsd = buy.priceUsd / (1 - buyFeeRate);
  const sellProceedsUsd = sell.priceUsd * (1 - sellFeeRate);
  const buyFeeUsd = buyCostUsd - buy.priceUsd;
  const sellFeeUsd = sell.priceUsd - sellProceedsUsd;

  return {
    buy,
    sell,
    buyIndex,
    sellIndex,
    spreadUsd,
    gasCostUsd,
    buyCostUsd,
    sellProceedsUsd,
    buyFeeUsd,
    sellFeeUsd,
    feeCostUsd: buyFeeUsd + sellFeeUsd,
    netUsd: sellProceedsUsd - buyCostUsd - gasCostUsd,
  };
}

/**
 * Best (highest-net) buy→sell route across all venue pairs. Iterates the full
 * NxN matrix because in `gas-and-fees` mode the optimal pair isn't always the
 * cheapest/richest — fee differences across venues can flip the ordering.
 */
export function findBestRoute(
  venues: readonly PriceSnapshot[],
  gasPriceGwei: number,
  mode: PnlMode,
): RouteAnalysis | null {
  if (venues.length < 2) return null;

  const referencePrice = median(venues.map((venue) => venue.priceUsd));
  const gasCostUsd = computeGasCostUsd(gasPriceGwei, referencePrice);

  let best: RouteAnalysis | null = null;
  for (let i = 0; i < venues.length; i += 1) {
    for (let j = 0; j < venues.length; j += 1) {
      if (i === j) continue;
      const candidate = computeRoute(venues[i], venues[j], i, j, gasCostUsd, mode);
      if (!best || candidate.netUsd > best.netUsd) {
        best = candidate;
      }
    }
  }

  return best;
}
