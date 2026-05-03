import { Card } from "../../primitives/Card";
import { formatUsd } from "../../../lib/format";
import { median } from "../../../lib/stats";
import { shortenVenueName, venueSwatchByIndex } from "../../../lib/venues";
import type { BlockRenderProps } from "./BlockRegistry";

/**
 * Vertical price ladder: shows every venue as a coloured dot on a price axis,
 * with the median rendered as a horizontal line. Visually answers "are the
 * venues clustered or scattered, and on which side of the median?".
 */
export function PriceLadderBlock({ market, onRemove }: BlockRenderProps) {
  const { overview } = market;
  const venues = overview?.venues ?? [];

  if (venues.length === 0) {
    return (
      <Card title="Ladder" subtitle="—" onRemove={onRemove}>
        <div className="card-empty">Awaiting first sample.</div>
      </Card>
    );
  }

  const prices = venues.map((venue) => venue.priceUsd);
  const minPrice = Math.min(...prices);
  const maxPrice = Math.max(...prices);
  const span = maxPrice - minPrice;
  // Pad the axis so dots aren't pinned to the very edges
  const pad = span === 0 ? Math.max(minPrice * 0.0005, 0.01) : span * 0.4;
  const domainMin = minPrice - pad;
  const domainMax = maxPrice + pad;
  const domainSpan = domainMax - domainMin;
  const med = median(prices);

  const yFor = (value: number) => {
    return ((domainMax - value) / domainSpan) * 100;
  };

  const ticks = [domainMax, (domainMax + domainMin) / 2, domainMin];

  return (
    <Card title="Ladder" subtitle={overview?.pairLabel ?? ""} onRemove={onRemove}>
      <div className="ladder-content">
        <div className="ladder-stage">
          <svg
            viewBox="0 0 100 100"
            preserveAspectRatio="none"
            className="ladder-svg"
          >
            {ticks.map((tick, idx) => (
              <line
                key={idx}
                x1={0}
                x2={100}
                y1={yFor(tick)}
                y2={yFor(tick)}
                className="ladder-grid"
              />
            ))}
            <line
              x1={0}
              x2={100}
              y1={yFor(med)}
              y2={yFor(med)}
              className="ladder-median"
            />
          </svg>

          <div className="ladder-ticks">
            {ticks.map((tick, idx) => (
              <span
                key={idx}
                className="ladder-tick mono"
                style={{ top: `${yFor(tick)}%` }}
              >
                {formatUsd(tick)}
              </span>
            ))}
            <span className="ladder-median-tick mono" style={{ top: `${yFor(med)}%` }}>
              μ
            </span>
          </div>

          <div className="ladder-dots">
            {venues
              .map((venue, idx) => ({ venue, idx, y: yFor(venue.priceUsd) }))
              .sort((a, b) => a.y - b.y)
              .map(({ venue, idx, y }) => (
                <div
                  key={venue.dexName}
                  className={`ladder-dot ${venueSwatchByIndex(idx)}`}
                  style={{ top: `${y}%` }}
                  title={`${venue.dexName}: ${formatUsd(venue.priceUsd)}`}
                >
                  <span className="ladder-dot-mark" />
                  <span className="ladder-dot-label mono">
                    {shortenVenueName(venue.dexName)} {formatUsd(venue.priceUsd)}
                  </span>
                </div>
              ))}
          </div>
        </div>
      </div>
    </Card>
  );
}
