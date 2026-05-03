import { useEffect, useRef, useState } from "react";

import { fetchMarketOverview } from "../features/arbitrage/api";
import type { MarketOverview, PriceSnapshot } from "../features/arbitrage/types";
import { HISTORY_LIMIT } from "../lib/config";

export type RefreshIntervalMs = 1000 | 2000 | 5000 | 10_000 | 0;

export interface MarketState {
  /** Most recent market overview (entire venue set + gas). */
  overview: MarketOverview | null;
  /** Hero snapshot — selected venue from the current overview. Defaults to
   *  `overview.venues[0]` when the user-selected hero dex name is null or
   *  not present in the active pair. */
  heroSnapshot: PriceSnapshot | null;
  /** Rolling history of overviews, oldest first, newest at the end. */
  history: MarketOverview[];
  /** True while a fetch is mid-flight. */
  loading: boolean;
  /** Error message from the most recent fetch, or null. */
  errorMessage: string | null;
  /** Trigger an immediate fetch outside the interval. */
  refresh: () => void;
}

interface UseMarketDataOptions {
  /** Rolling buffer cap. Defaults to `HISTORY_LIMIT` (100). */
  historyLimit?: number;
  /** Preferred hero venue by `dexName`. When the active pair doesn't
   *  include that venue, falls back to `venues[0]`. */
  heroVenueDexName?: string | null;
}

/**
 * Centralised market-data poller. One fetch loop runs regardless of how many
 * cards consume the data — they share state via this hook in the parent and
 * receive it as props.
 *
 * Switching `pairId` clears history and triggers an immediate fetch for the
 * new pair so the chart and analytics start clean. `intervalMs === 0` pauses
 * polling; `refresh()` always fetches once.
 */
export function useMarketData(
  intervalMs: RefreshIntervalMs,
  pairId: string,
  options: UseMarketDataOptions = {},
): MarketState {
  const limit = options.historyLimit ?? HISTORY_LIMIT;
  const heroVenueDexName = options.heroVenueDexName ?? null;
  const [overview, setOverview] = useState<MarketOverview | null>(null);
  const [history, setHistory] = useState<MarketOverview[]>([]);
  const [loading, setLoading] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const requestInFlight = useRef(false);
  const activePairRef = useRef(pairId);

  const loadSnapshot = async (forPairId: string) => {
    if (requestInFlight.current) {
      return;
    }

    requestInFlight.current = true;
    setLoading(true);

    try {
      const next = await fetchMarketOverview(forPairId);
      // Drop responses that arrived after the user switched pairs.
      if (activePairRef.current !== forPairId) {
        return;
      }
      setOverview(next);
      setErrorMessage(null);
      setHistory((prev) => {
        const merged = [...prev, next];
        return merged.length > limit ? merged.slice(-limit) : merged;
      });
    } catch (error) {
      if (activePairRef.current !== forPairId) {
        return;
      }
      const message =
        error instanceof Error ? error.message : "Failed to read market state.";
      setErrorMessage(message);
    } finally {
      requestInFlight.current = false;
      setLoading(false);
    }
  };

  useEffect(() => {
    activePairRef.current = pairId;
    setOverview(null);
    setHistory([]);
    setErrorMessage(null);

    void loadSnapshot(pairId);

    if (intervalMs === 0) {
      return;
    }

    const id = window.setInterval(() => {
      void loadSnapshot(pairId);
    }, intervalMs);

    return () => {
      window.clearInterval(id);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [intervalMs, pairId]);

  const resolvedHero =
    overview === null
      ? null
      : heroVenueDexName !== null
        ? (overview.venues.find((v) => v.dexName === heroVenueDexName) ??
          overview.venues[0] ??
          null)
        : (overview.venues[0] ?? null);

  return {
    overview,
    heroSnapshot: resolvedHero,
    history,
    loading,
    errorMessage,
    refresh: () => {
      void loadSnapshot(pairId);
    },
  };
}

/** Discrete history-buffer presets the settings menu offers. */
export const HISTORY_LIMIT_OPTIONS = [50, 100, 200, 500] as const;
export type HistoryLimit = (typeof HISTORY_LIMIT_OPTIONS)[number];
