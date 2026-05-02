/**
 * Per-venue presentation helpers.
 *
 * Venue identity, decimals, and address are owned by the backend pair catalog
 * (`src-tauri/src/config/`). The frontend's job is purely presentational:
 * map venue index → swatch class, and shorten a backend `dexName` for tight
 * UI surfaces (venue tiles, chart legends).
 */

export type SwatchClass =
  | "venue-1"
  | "venue-2"
  | "venue-3"
  | "venue-4"
  | "venue-5"
  | "venue-6";

const SWATCH_ORDER: SwatchClass[] = [
  "venue-1",
  "venue-2",
  "venue-3",
  "venue-4",
  "venue-5",
  "venue-6",
];

/**
 * Returns the CSS swatch class for the venue at `index` in the current
 * pair's venue list. Cycles after 6 venues.
 */
export function venueSwatchByIndex(index: number): SwatchClass {
  return SWATCH_ORDER[index % SWATCH_ORDER.length];
}

/**
 * Compact label used for venue tiles and chart-legend swatches. Falls back to
 * the first 10 chars of the input when no specific pattern matches.
 */
/**
 * Locate the cheapest and richest entries by `priceUsd`. Returns the entries
 * themselves plus their indices (for venue colour resolution). Returns null
 * for an empty input.
 */
export function findExtremes<T extends { priceUsd: number }>(
  venues: T[],
): { cheapest: T; richest: T; cheapestIndex: number; richestIndex: number } | null {
  if (venues.length === 0) return null;
  let cheapestIdx = 0;
  let richestIdx = 0;
  for (let i = 1; i < venues.length; i += 1) {
    if (venues[i].priceUsd < venues[cheapestIdx].priceUsd) cheapestIdx = i;
    if (venues[i].priceUsd > venues[richestIdx].priceUsd) richestIdx = i;
  }
  return {
    cheapest: venues[cheapestIdx],
    richest: venues[richestIdx],
    cheapestIndex: cheapestIdx,
    richestIndex: richestIdx,
  };
}

export function shortenVenueName(dexName: string): string {
  const lower = dexName.toLowerCase();

  if (lower.includes("v3") && lower.includes("5bps")) return "V3-5";
  if (lower.includes("v3") && lower.includes("30bps")) return "V3-30";
  if (lower.includes("v3") && lower.includes("100bps")) return "V3-100";
  if (lower.includes("uniswap v2")) return "V2";
  if (lower.includes("sushi")) return "Sushi";
  if (lower.includes("curve")) return "Curve";
  if (lower.includes("balancer")) return "Bal";
  if (lower.includes("pancake")) return "Cake";

  return dexName.length > 10 ? dexName.slice(0, 10) : dexName;
}
