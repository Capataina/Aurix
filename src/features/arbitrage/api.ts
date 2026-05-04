import { loggedInvoke } from "../../lib/telemetry";

import type { MarketOverview, PairSummary } from "./types";

/**
 * Requests the multi-venue market overview for the supplied pair id.
 * The backend resolves the pair from its catalog and concurrently reads
 * every registered venue.
 */
export function fetchMarketOverview(pairId: string): Promise<MarketOverview> {
  return loggedInvoke<MarketOverview>("fetch_market_overview", { pairId });
}

/**
 * Returns the catalog of pairs the backend can read prices for.
 * Called once at app startup to populate the pair selector.
 */
export function listPairs(): Promise<PairSummary[]> {
  return loggedInvoke<PairSummary[]>("list_pairs");
}
