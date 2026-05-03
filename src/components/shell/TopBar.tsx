import { RefreshIcon } from "../primitives/Icon";
import type { RefreshIntervalMs } from "../../hooks/useMarketData";
import type { PairSummary } from "../../features/arbitrage/types";
import type { PnlMode } from "../../lib/arbitrage";

export type ConnectionStatus = "live" | "stale" | "down" | "paused";

interface TopBarProps {
  activeTabId: string;
  onSelectTab: (tabId: string) => void;
  connectionStatus: ConnectionStatus;
  connectionLabel: string;
  pairs: PairSummary[];
  pairId: string;
  onSelectPair: (pairId: string) => void;
  pnlMode: PnlMode;
  onSelectPnlMode: (mode: PnlMode) => void;
  intervalMs: RefreshIntervalMs;
  onSelectInterval: (interval: RefreshIntervalMs) => void;
  onRefresh: () => void;
}

interface TabDef {
  id: string;
  label: string;
  status: "active" | "soon";
}

const TABS: TabDef[] = [
  { id: "arbitrage", label: "Arbitrage", status: "active" },
  { id: "lp-backtester", label: "LP Backtester", status: "active" },
  { id: "wallet", label: "Wallet", status: "soon" },
  { id: "gas", label: "Gas", status: "soon" },
  { id: "risk", label: "Risk", status: "soon" },
];

const STATUS_LABEL: Record<ConnectionStatus, string> = {
  live: "Live",
  stale: "Stale",
  down: "Disconnected",
  paused: "Paused",
};

const INTERVAL_OPTIONS: Array<{ value: RefreshIntervalMs; label: string }> = [
  { value: 1000, label: "1s" },
  { value: 2000, label: "2s" },
  { value: 5000, label: "5s" },
  { value: 10_000, label: "10s" },
  { value: 0, label: "Paused" },
];

function TopBarDivider() {
  return <span className="topbar-divider" aria-hidden />;
}

export function TopBar({
  activeTabId,
  onSelectTab,
  connectionStatus,
  connectionLabel,
  pairs,
  pairId,
  onSelectPair,
  pnlMode,
  onSelectPnlMode,
  intervalMs,
  onSelectInterval,
  onRefresh,
}: TopBarProps) {
  const poolFeesActive = pnlMode === "gas-and-fees";

  return (
    <header className="topbar">
      <div className="topbar-brand">
        <span className="topbar-logo" aria-hidden />
        <span className="topbar-name">
          Aur<span className="topbar-name-accent">i</span>x
        </span>
      </div>

      <nav className="topbar-nav" aria-label="Primary navigation">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            type="button"
            className={`topbar-tab ${tab.id === activeTabId ? "is-active" : ""} ${
              tab.status === "soon" ? "is-disabled" : ""
            }`}
            onClick={() => {
              if (tab.status === "active") {
                onSelectTab(tab.id);
              }
            }}
            aria-pressed={tab.id === activeTabId}
            disabled={tab.status === "soon"}
          >
            {tab.label}
            {tab.status === "soon" ? <span className="topbar-tab-badge">Soon</span> : null}
          </button>
        ))}
      </nav>

      <div className="topbar-actions">
        <span className="topbar-status">
          <span
            className={`topbar-status-dot ${
              connectionStatus === "live"
                ? ""
                : connectionStatus === "stale"
                  ? "is-stale"
                  : connectionStatus === "paused"
                    ? "is-paused"
                    : "is-down"
            }`}
          />
          {STATUS_LABEL[connectionStatus]} · {connectionLabel}
        </span>

        <TopBarDivider />

        {pairs.length > 0 ? (
          <div
            className="topbar-segmented"
            role="group"
            aria-label="Trading pair"
          >
            {pairs.map((pair) => (
              <button
                key={pair.id}
                type="button"
                className={`topbar-segment ${pair.id === pairId ? "is-active" : ""}`}
                onClick={() => onSelectPair(pair.id)}
                aria-pressed={pair.id === pairId}
                title={pair.label}
              >
                {pair.baseSymbol}
              </button>
            ))}
          </div>
        ) : null}

        <TopBarDivider />

        <button
          type="button"
          className={`topbar-switch ${poolFeesActive ? "is-active" : ""}`}
          onClick={() => onSelectPnlMode(poolFeesActive ? "gas" : "gas-and-fees")}
          aria-pressed={poolFeesActive}
          title="Subtract pool fees in addition to gas when reporting net P/L"
        >
          <span className="topbar-switch-track" aria-hidden>
            <span className="topbar-switch-thumb" />
          </span>
          Pool fees
        </button>

        <TopBarDivider />

        <select
          className="topbar-select"
          value={String(intervalMs)}
          onChange={(event) =>
            onSelectInterval(Number(event.target.value) as RefreshIntervalMs)
          }
          aria-label="Refresh interval"
        >
          {INTERVAL_OPTIONS.map((option) => (
            <option key={option.value} value={String(option.value)}>
              {option.label}
            </option>
          ))}
        </select>

        <button
          type="button"
          className="topbar-button"
          onClick={onRefresh}
          aria-label="Refresh now"
          title="Refresh now"
        >
          <RefreshIcon className="topbar-button-icon" />
        </button>
      </div>
    </header>
  );
}
