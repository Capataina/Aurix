/**
 * Normalised market snapshot surfaced by the Rust backend.
 */
export interface PriceSnapshot {
  chain: string;
  dexName: string;
  pairLabel: string;
  priceUsd: number;
  poolAddress: string;
  feeTierBps: number;
  priceSourceLabel: string;
  fetchedAtUnixMs: number;
}

/**
 * Aggregated market state for all active venues in the current sampling tick.
 */
export interface MarketOverview {
  chain: string;
  /** Stable pair identifier matching the backend catalog. */
  pairId: string;
  pairLabel: string;
  fetchedAtUnixMs: number;
  gasPriceGwei: number;
  venues: PriceSnapshot[];
}

/**
 * Summary of one registered pair, returned by the backend `list_pairs` command.
 */
export interface PairSummary {
  id: string;
  label: string;
  baseSymbol: string;
  quoteSymbol: string;
  venueCount: number;
}
