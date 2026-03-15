import type { PriceSnapshot } from "../types";

interface PriceCardProps {
  snapshot: PriceSnapshot | null;
  gasPriceGwei: number | null;
  loading: boolean;
  errorMessage: string | null;
  onRefresh: () => void;
}

function formatUsd(value: number): string {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    maximumFractionDigits: 2,
  }).format(value);
}

function formatTimestamp(timestampMs: number): string {
  return new Intl.DateTimeFormat("en-GB", {
    dateStyle: "medium",
    timeStyle: "medium",
  }).format(timestampMs);
}

/**
 * Displays the first live market price returned by the backend.
 */
export function PriceCard({
  snapshot,
  gasPriceGwei,
  loading,
  errorMessage,
  onRefresh,
}: PriceCardProps) {
  const priceLabel = snapshot ? formatUsd(snapshot.priceUsd) : "Awaiting live read";
  const exactPriceLabel = snapshot ? snapshot.priceUsd.toFixed(6) : "Waiting";
  const updatedLabel = snapshot
    ? `Updated ${formatTimestamp(snapshot.fetchedAtUnixMs)}`
    : "No market snapshot captured yet";

  return (
    <section className="hero-sidebar">
      <div className="hero-heading">
        <span className="eyebrow">Primary market</span>
        <h1 className="hero-title">Aurix</h1>
        <p className="hero-summary">
          Three live Ethereum venue reads with compact detail and a shared
          visual monitoring surface.
        </p>
      </div>
      <div className="price-stage">
        <div className="price-meta">
          <span className="status-pill status-live">
            {loading ? "Refreshing" : "Live"}
          </span>
          <span className="status-pill status-neutral">
            {snapshot?.dexName ?? "Uniswap V3"}
          </span>
          <span className="status-pill status-neutral">
            {snapshot?.pairLabel ?? "WETH / USDC"}
          </span>
        </div>

        <div className="price-value">{priceLabel}</div>
        <p className="price-caption">Precise: {exactPriceLabel}</p>
        <p className="price-caption">{updatedLabel}</p>
        <p className="price-caption">
          Gas: {gasPriceGwei !== null ? `${gasPriceGwei.toFixed(2)} gwei` : "Waiting"}
        </p>

        {errorMessage ? <p className="error-banner">{errorMessage}</p> : null}

        <button className="refresh-button" onClick={onRefresh} type="button">
          Refresh market
        </button>
      </div>
    </section>
  );
}
