import { Card } from "../primitives/Card";
import { Pill } from "../primitives/Pill";
import { GAS_UNITS_ESTIMATE } from "../../lib/config";
import {
  formatGweiSmart,
  formatPreciseUsd,
  formatSignedUsd,
  formatUsd,
} from "../../lib/format";
import { median } from "../../lib/stats";
import { findExtremes, shortenVenueName, venueSwatchByIndex } from "../../lib/venues";
import type { BlockRenderProps } from "./BlockRegistry";

/**
 * The single most important card if you're trying to read what the dashboard
 * is telling you: shows the explicit best arbitrage route as
 *   BUY at <cheapest venue, with price>
 *   SELL at <richest venue, with price>
 * plus a clean breakdown of spread − gas = net, and an at-a-glance verdict.
 */
export function ArbRouteBlock({ market, onRemove }: BlockRenderProps) {
  const { overview } = market;

  if (!overview || overview.venues.length === 0) {
    return (
      <Card title="Best route" subtitle="—" onRemove={onRemove}>
        <div className="card-empty">Awaiting first sample.</div>
      </Card>
    );
  }

  const extremes = findExtremes(overview.venues);
  if (!extremes) {
    return (
      <Card title="Best route" subtitle="—" onRemove={onRemove}>
        <div className="card-empty">Awaiting first sample.</div>
      </Card>
    );
  }

  const spread = extremes.richest.priceUsd - extremes.cheapest.priceUsd;
  const medianPrice = median(overview.venues.map((venue) => venue.priceUsd));
  const gasCostUsd =
    (overview.gasPriceGwei * GAS_UNITS_ESTIMATE * medianPrice) / 1_000_000_000;
  const net = spread - gasCostUsd;

  const verdictTone = net > 0 ? "up" : "down";
  const verdictLabel = net > 0 ? "PROFITABLE" : "UNPROFITABLE";

  return (
    <Card
      title="Best route"
      subtitle={`${shortenVenueName(extremes.cheapest.dexName)} → ${shortenVenueName(
        extremes.richest.dexName,
      )}`}
      onRemove={onRemove}
      headerExtra={
        <Pill tone={verdictTone} showDot pulse={net > 0}>
          {verdictLabel}
        </Pill>
      }
    >
      <div className="arb-route-content">
        <div className="arb-route-leg">
          <span className="arb-route-leg-tag is-up">BUY</span>
          <span className="arb-route-venue">
            <span className={`dot ${venueSwatchByIndex(extremes.cheapestIndex)}`} />
            {shortenVenueName(extremes.cheapest.dexName)}
          </span>
          <span className="arb-route-price mono">
            {formatPreciseUsd(extremes.cheapest.priceUsd, 2)}
          </span>
        </div>

        <div className="arb-route-leg">
          <span className="arb-route-leg-tag is-down">SELL</span>
          <span className="arb-route-venue">
            <span className={`dot ${venueSwatchByIndex(extremes.richestIndex)}`} />
            {shortenVenueName(extremes.richest.dexName)}
          </span>
          <span className="arb-route-price mono">
            {formatPreciseUsd(extremes.richest.priceUsd, 2)}
          </span>
        </div>

        <div className="route-divider" />

        <div className="arb-route-math">
          <div className="arb-route-math-row">
            <span className="arb-route-math-label">spread</span>
            <span className="arb-route-math-value mono">{formatUsd(spread)}</span>
          </div>
          <div className="arb-route-math-row">
            <span className="arb-route-math-label">
              gas <span className="mono" style={{ fontSize: 10, color: "var(--text-muted)" }}>
                ({formatGweiSmart(overview.gasPriceGwei)} × {GAS_UNITS_ESTIMATE.toLocaleString()})
              </span>
            </span>
            <span className="arb-route-math-value mono is-down">
              −{formatUsd(gasCostUsd)}
            </span>
          </div>
          <div className="arb-route-math-row arb-route-math-net">
            <span className="arb-route-math-label">net</span>
            <span
              className={`arb-route-math-value mono ${
                net > 0 ? "is-up" : "is-down"
              }`}
            >
              {formatSignedUsd(net)}
            </span>
          </div>
        </div>
      </div>
    </Card>
  );
}
