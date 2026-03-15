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
  pairLabel: string;
  fetchedAtUnixMs: number;
  gasPriceGwei: number;
  venues: PriceSnapshot[];
}
