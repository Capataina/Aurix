import { PanelRightIcon, RefreshIcon } from "../primitives/Icon";
import type { RefreshIntervalMs } from "../../hooks/useMarketData";
import type { PairSummary } from "../../features/arbitrage/types";

export type ConnectionStatus = "live" | "stale" | "down" | "paused";

interface TopBarProps {
  activeTabId: string;
  onSelectTab: (tabId: string) => void;
  connectionStatus: ConnectionStatus;
  connectionLabel: string;
  pairs: PairSummary[];
  pairId: string;
  onSelectPair: (pairId: string) => void;
  intervalMs: RefreshIntervalMs;
  onSelectInterval: (interval: RefreshIntervalMs) => void;
  onRefresh: () => void;
  onToggleDrawer: () => void;
  drawerOpen: boolean;
}

interface TabDef {
  id: string;
  label: string;
  status: "active" | "soon";
}

const TABS: TabDef[] = [
  { id: "arbitrage", label: "Arbitrage", status: "active" },
  { id: "lp-backtester", label: "LP Backtester", status: "soon" },
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

export function TopBar({
  activeTabId,
  onSelectTab,
  connectionStatus,
  connectionLabel,
  pairs,
  pairId,
  onSelectPair,
  intervalMs,
  onSelectInterval,
  onRefresh,
  onToggleDrawer,
  drawerOpen,
}: TopBarProps) {
  const activePair = pairs.find((entry) => entry.id === pairId);
  const fallbackLabel = activePair?.label ?? pairId;

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
        <div className="topbar-meta">
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
        </div>

        <select
          className="topbar-select"
          value={pairId}
          onChange={(event) => onSelectPair(event.target.value)}
          aria-label="Trading pair"
        >
          {pairs.length === 0 ? (
            <option value={pairId}>{fallbackLabel}</option>
          ) : (
            pairs.map((pair) => (
              <option key={pair.id} value={pair.id}>
                {pair.label}
              </option>
            ))
          )}
        </select>

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

        <button
          type="button"
          className={`topbar-button ${drawerOpen ? "is-primary" : ""}`}
          onClick={onToggleDrawer}
          aria-label="Toggle block library"
          aria-expanded={drawerOpen}
        >
          <PanelRightIcon className="topbar-button-icon" />
          Blocks
        </button>
      </div>
    </header>
  );
}
