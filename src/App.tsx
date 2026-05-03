import { useEffect, useState } from "react";

import { ArbitragePage } from "./features/arbitrage/ArbitragePage";
import { LpBacktestPage } from "./features/lp-backtest/LpBacktestPage";
import { TopBar, type ConnectionStatus } from "./components/shell/TopBar";
import { useMarketData, type RefreshIntervalMs } from "./hooks/useMarketData";
import { usePersistedState } from "./hooks/usePersistedState";
import { listPairs } from "./features/arbitrage/api";
import type { PairSummary } from "./features/arbitrage/types";
import { DEFAULT_PAIR_ID } from "./lib/config";
import { DEFAULT_PNL_MODE, type PnlMode } from "./lib/arbitrage";

const STALE_AFTER_MS = 8_000;

function deriveConnectionStatus(
  intervalMs: RefreshIntervalMs,
  errorMessage: string | null,
  lastTickMs: number | null,
  nowMs: number,
): ConnectionStatus {
  if (intervalMs === 0) return "paused";
  if (errorMessage) return "down";
  if (lastTickMs === null) return "stale";
  if (nowMs - lastTickMs > STALE_AFTER_MS) return "stale";
  return "live";
}

export default function App() {
  const [activeTabId, setActiveTabId] = useState("arbitrage");
  const [intervalMs, setIntervalMs] = usePersistedState<RefreshIntervalMs>(
    "aurix:refresh-interval",
    1000,
  );
  const [pairId, setPairId] = usePersistedState<string>(
    "aurix:pair-id",
    DEFAULT_PAIR_ID,
  );
  const [pnlMode, setPnlMode] = usePersistedState<PnlMode>(
    "aurix:pnl-mode",
    DEFAULT_PNL_MODE,
  );
  const [pairs, setPairs] = useState<PairSummary[]>([]);

  useEffect(() => {
    let cancelled = false;
    listPairs()
      .then((catalog) => {
        if (!cancelled) {
          setPairs(catalog);
          // Defensive: if persisted pairId is no longer in the catalog, fall back.
          if (catalog.length > 0 && !catalog.some((entry) => entry.id === pairId)) {
            setPairId(catalog[0].id);
          }
        }
      })
      .catch(() => {
        /* swallowed — pair selector will render with current id alone */
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const market = useMarketData(intervalMs, pairId);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const id = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, []);

  const lastTickMs = market.overview?.fetchedAtUnixMs ?? null;
  const connectionStatus = deriveConnectionStatus(
    intervalMs,
    market.errorMessage,
    lastTickMs,
    now,
  );

  const connectionLabel = market.overview?.chain ?? "Ethereum mainnet";

  return (
    <div className="app-shell">
      <TopBar
        activeTabId={activeTabId}
        onSelectTab={setActiveTabId}
        connectionStatus={connectionStatus}
        connectionLabel={connectionLabel}
        pairs={pairs}
        pairId={pairId}
        onSelectPair={setPairId}
        pnlMode={pnlMode}
        onSelectPnlMode={setPnlMode}
        intervalMs={intervalMs}
        onSelectInterval={setIntervalMs}
        onRefresh={market.refresh}
      />

      <main className="app-main">
        {activeTabId === "arbitrage" ? (
          <ArbitragePage market={market} pnlMode={pnlMode} />
        ) : activeTabId === "lp-backtester" ? (
          <LpBacktestPage />
        ) : null}
      </main>
    </div>
  );
}
