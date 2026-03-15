import { invoke } from "@tauri-apps/api/core";

import type { MarketOverview } from "./types";

/**
 * Requests the current multi-venue market overview from the Tauri backend.
 */
export function fetchMarketOverview(): Promise<MarketOverview> {
  return invoke<MarketOverview>("fetch_market_overview");
}
