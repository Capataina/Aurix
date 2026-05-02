/**
 * Frontend constants that pair with the backend `RuntimeConfig`.
 *
 * These values mirror the backend defaults today; if you change either side
 * the other should follow until/unless we wire `runtime_config` from the
 * Tauri command and consume it at startup.
 */

/** Estimated gas units for an arbitrage swap, used by gas-adjusted analytics. */
export const GAS_UNITS_ESTIMATE = 220_000;

/** Rolling-window length kept by `useMarketData`. */
export const HISTORY_LIMIT = 100;

/** Number of x-axis slots the price chart spaces samples across. */
export const SLOT_COUNT = 100;

/** Default pair id loaded on first launch (must exist in the backend pair catalog). */
export const DEFAULT_PAIR_ID = "weth-usdc";
