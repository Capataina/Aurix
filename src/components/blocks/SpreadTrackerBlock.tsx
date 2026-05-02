import { Card } from "../primitives/Card";
import { Histogram } from "../primitives/Histogram";
import { RangeIndicator } from "../primitives/RangeIndicator";
import { formatPreciseUsd, formatUsd } from "../../lib/format";
import { mean, range as rangeStats } from "../../lib/stats";
import { findExtremes, shortenVenueName, venueSwatchByIndex } from "../../lib/venues";
import type { BlockRenderProps } from "./BlockRegistry";

export function SpreadTrackerBlock({ market, onRemove }: BlockRenderProps) {
  const { history, overview } = market;

  const spreads = history.map((entry) => {
    const prices = entry.venues.map((venue) => venue.priceUsd);
    return Math.max(...prices) - Math.min(...prices);
  });

  const currentSpread =
    overview && overview.venues.length > 0
      ? Math.max(...overview.venues.map((venue) => venue.priceUsd)) -
        Math.min(...overview.venues.map((venue) => venue.priceUsd))
      : null;

  const baseline = mean(spreads);
  const { min, max } = rangeStats(spreads);
  const tone =
    currentSpread === null
      ? "neutral"
      : currentSpread > baseline * 1.15
        ? "warn"
        : "info";

  const extremes = overview ? findExtremes(overview.venues) : null;

  return (
    <Card title="Spread" subtitle="high − low" onRemove={onRemove}>
      <div className="spread-content">
        <div className="spread-headline">
          <span
            className={`metric-value is-large ${tone === "warn" ? "is-warn" : ""}`}
          >
            {currentSpread === null ? "—" : formatUsd(currentSpread)}
          </span>
          {extremes ? (
            <div className="spread-route">
              <div className="spread-route-row">
                <span className="route-row-tag is-down">▲</span>
                <span className="route-row-venue">
                  <span className={`dot ${venueSwatchByIndex(extremes.richestIndex)}`} />
                  {shortenVenueName(extremes.richest.dexName)}
                </span>
                <span className="route-row-price">
                  {formatPreciseUsd(extremes.richest.priceUsd, 2)}
                </span>
              </div>
              <div className="spread-route-row">
                <span className="route-row-tag is-up">▼</span>
                <span className="route-row-venue">
                  <span className={`dot ${venueSwatchByIndex(extremes.cheapestIndex)}`} />
                  {shortenVenueName(extremes.cheapest.dexName)}
                </span>
                <span className="route-row-price">
                  {formatPreciseUsd(extremes.cheapest.priceUsd, 2)}
                </span>
              </div>
            </div>
          ) : null}
          <span className="metric-delta numeric spread-baseline">
            {spreads.length > 0 ? `μ ${formatUsd(baseline)}` : "—"}
          </span>
        </div>

        <div style={{ flex: 1, minHeight: 0 }}>
          <Histogram
            values={spreads}
            marker={currentSpread ?? undefined}
            tone={tone === "warn" ? "warn" : "info"}
            height={undefined}
          />
        </div>

        {currentSpread !== null && spreads.length > 1 ? (
          <RangeIndicator
            current={currentSpread}
            min={min}
            max={max}
            baseline={baseline}
            tone={tone === "warn" ? "warn" : "info"}
          />
        ) : null}
      </div>
    </Card>
  );
}
