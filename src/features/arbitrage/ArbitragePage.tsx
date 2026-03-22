import { useEffect, useRef, useState } from "react";

import { fetchMarketOverview } from "./api";
import { InsightsPanel } from "./components/InsightsPanel";
import { MarketChart, type ChartMode } from "./components/MarketChart";
import { PriceCard } from "./components/PriceCard";
import { deriveInsightsView } from "./insights";
import type { MarketOverview, PriceSnapshot } from "./types";

const VENUES = [
  {
    name: "Uniswap V3 5bps",
    state: "Active",
    accent: "dex-accent-sky",
    summary: "Concentrated-liquidity anchor venue for the main market line.",
  },
  {
    name: "Uniswap V3 30bps",
    state: "Active",
    accent: "dex-accent-lilac",
    summary: "Higher-fee concentrated venue for richer same-DEX spread comparisons.",
  },
  {
    name: "Uniswap V2",
    state: "Active",
    accent: "dex-accent-peach",
    summary: "Reserve-ratio comparison lane sourced from the classic pool model.",
  },
  {
    name: "SushiSwap",
    state: "Active",
    accent: "dex-accent-mint",
    summary: "Second reserve-based venue for visible cross-market divergence.",
  },
];

const HISTORY_LIMIT = 100;
const REFRESH_INTERVAL_MS = 1_000;
const GAS_UNITS_ESTIMATE = 220_000;

function formatUsd(value: number): string {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    maximumFractionDigits: 2,
  }).format(value);
}

function median(values: number[]): number {
  const sortedValues = [...values].sort((left, right) => left - right);
  const midpoint = Math.floor(sortedValues.length / 2);

  if (sortedValues.length % 2 === 0) {
    return (sortedValues[midpoint - 1] + sortedValues[midpoint]) / 2;
  }

  return sortedValues[midpoint];
}

/**
 * Hosts the first arbitrage analytics screen and coordinates live refreshes.
 */
export function ArbitragePage() {
  const [snapshot, setSnapshot] = useState<PriceSnapshot | null>(null);
  const [overview, setOverview] = useState<MarketOverview | null>(null);
  const [history, setHistory] = useState<MarketOverview[]>([]);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [chartMode, setChartMode] = useState<ChartMode>("spread");
  const [showEvents, setShowEvents] = useState(true);
  const requestInFlight = useRef(false);
  const insights = history.length > 0 ? deriveInsightsView(history) : null;

  async function loadSnapshot() {
    if (requestInFlight.current) {
      return;
    }

    requestInFlight.current = true;
    setLoading(true);
    setErrorMessage(null);

    try {
      const nextOverview = await fetchMarketOverview();
      setOverview(nextOverview);
      setSnapshot(nextOverview.venues[0] ?? null);
      setHistory((currentHistory) => {
        const nextHistory = [...currentHistory, nextOverview];
        return nextHistory.slice(-HISTORY_LIMIT);
      });
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to read market state.";
      setErrorMessage(message);
    } finally {
      requestInFlight.current = false;
      setLoading(false);
    }
  }

  useEffect(() => {
    void loadSnapshot();

    const intervalId = window.setInterval(() => {
      void loadSnapshot();
    }, REFRESH_INTERVAL_MS);

    return () => {
      window.clearInterval(intervalId);
    };
  }, []);

  return (
    <main className="app-shell">
      <header className="top-bar">
        <div>
          <span className="eyebrow">Local-first DeFi analytics</span>
          <p className="top-bar-copy">
            Watch markets as time-series first, then drill into venue detail
            and exact pricing when you need it.
          </p>
        </div>

        <div className="mode-pills">
          <span className="status-pill status-live">Ethereum mainnet</span>
          <span className="status-pill status-neutral">WETH / USDC</span>
          <span className="status-pill status-neutral">1s cadence</span>
        </div>
      </header>

      <section className="panel hero-card">
        <PriceCard
          snapshot={snapshot}
          gasPriceGwei={overview?.gasPriceGwei ?? null}
          loading={loading}
          errorMessage={errorMessage}
          onRefresh={loadSnapshot}
        />
        <MarketChart
          history={history}
          activeLabel={overview?.pairLabel ?? "WETH / USDC"}
          chartMode={chartMode}
          onSelectMode={setChartMode}
          showEvents={showEvents}
          onToggleEvents={() => setShowEvents((current) => !current)}
        />
      </section>

      {insights ? <InsightsPanel insights={insights} /> : null}

      <section className="dashboard-grid">
        <section className="panel feature-panel">
          <div className="section-header">
            <span className="eyebrow">Venue Surface</span>
            <h2 className="section-title">Venue lanes</h2>
            <p>
              Each venue keeps its own lane, accent, and state so the next feeds
              can join the surface without cluttering it.
            </p>
          </div>

          <div className="exchange-list">
            {VENUES.map((exchange) => (
              <article className="exchange-card" key={exchange.name}>
                <div className={`exchange-accent ${exchange.accent}`} />
                <div className="venue-content">
                  <div className="exchange-header">
                    <h3>{exchange.name}</h3>
                    <span className="status-pill status-neutral">
                      {exchange.state}
                    </span>
                  </div>
                  <p>{exchange.summary}</p>
                  <div className="venue-meta">
                    <span className="status-pill status-neutral">
                      {overview
                        ? formatUsd(
                            overview.venues.find(
                              (venue) => venue.dexName === exchange.name,
                            )?.priceUsd ?? 0,
                          )
                        : "Waiting"}
                    </span>
                    <span className="status-pill status-neutral">Live venue</span>
                  </div>
                </div>
              </article>
            ))}
          </div>
        </section>

        <section className="panel detail-panel">
          <div className="section-header">
            <span className="eyebrow">Market Detail</span>
            <h2 className="section-title">Current snapshot</h2>
          </div>

          <dl className="detail-list">
            <div>
              <dt>Chain</dt>
              <dd>{snapshot?.chain ?? "Ethereum Mainnet"}</dd>
            </div>
            <div>
              <dt>Pool</dt>
              <dd>{snapshot?.poolAddress ?? "Waiting for live read"}</dd>
            </div>
            <div>
              <dt>Venue spread</dt>
              <dd>
                {overview
                  ? (() => {
                      const prices = overview.venues.map((venue) => venue.priceUsd);
                      const spread = Math.max(...prices) - Math.min(...prices);
                      return formatUsd(spread);
                    })()
                  : "Waiting for live read"}
              </dd>
            </div>
            <div>
              <dt>Active venues</dt>
              <dd>
                {overview ? overview.venues.map((venue) => venue.dexName).join(", ") : "3"}
              </dd>
            </div>
            <div>
              <dt>Median price</dt>
              <dd>
                {overview
                  ? formatUsd(median(overview.venues.map((venue) => venue.priceUsd)))
                  : "Waiting for live read"}
              </dd>
            </div>
            <div>
              <dt>Net spread est.</dt>
              <dd>
                {overview
                  ? (() => {
                      const prices = overview.venues.map((venue) => venue.priceUsd);
                      const medianPrice = median(prices);
                      const spread = Math.max(...prices) - Math.min(...prices);
                      const gasCostUsd =
                        (overview.gasPriceGwei *
                          GAS_UNITS_ESTIMATE *
                          medianPrice) /
                        1_000_000_000;
                      return formatUsd(spread - gasCostUsd);
                    })()
                  : "Waiting for live read"}
              </dd>
            </div>
            <div>
              <dt>Last source</dt>
              <dd>{snapshot?.priceSourceLabel ?? "slot0() spot price"}</dd>
            </div>
          </dl>
        </section>
      </section>
    </main>
  );
}
