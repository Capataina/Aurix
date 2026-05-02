import { Card } from "../primitives/Card";
import { DeviationBar } from "../primitives/DeviationBar";
import { formatPreciseUsd, formatSignedPercent } from "../../lib/format";
import { median } from "../../lib/stats";
import { shortenVenueName, venueSwatchByIndex } from "../../lib/venues";
import type { BlockRenderProps } from "./BlockRegistry";

export function VenueGridBlock({ market, onRemove }: BlockRenderProps) {
  const { overview } = market;
  const venues = overview?.venues ?? [];

  const prices = venues.map((venue) => venue.priceUsd);
  const medianPrice = prices.length > 0 ? median(prices) : 0;
  const cheapest = prices.length > 0 ? Math.min(...prices) : null;
  const richest = prices.length > 0 ? Math.max(...prices) : null;
  const maxDeviationPct =
    medianPrice > 0
      ? Math.max(
          ...venues.map((v) => Math.abs((v.priceUsd - medianPrice) / medianPrice) * 100),
          0.001,
        )
      : 0.001;

  return (
    <Card
      title="Venues"
      subtitle={overview?.pairLabel ?? "—"}
      onRemove={onRemove}
      bodyClassName="is-grid"
    >
      <div className="venue-grid">
        {venues.length === 0 ? (
          <div className="card-empty" style={{ gridColumn: "1 / -1" }}>
            Awaiting first sample.
          </div>
        ) : (
          venues.map((venue, index) => {
            const deviationPct = ((venue.priceUsd - medianPrice) / medianPrice) * 100;
            const isCheapest = cheapest !== null && venue.priceUsd === cheapest;
            const isRichest = richest !== null && venue.priceUsd === richest;

            const stateClass = isCheapest
              ? "is-cheapest"
              : isRichest
                ? "is-richest"
                : "";

            return (
              <div
                className={`venue-tile ${venueSwatchByIndex(index)} ${stateClass}`}
                key={venue.dexName}
              >
                <div className="venue-tile-row">
                  <span className="venue-tile-name">{shortenVenueName(venue.dexName)}</span>
                  {isCheapest ? <span className="venue-tile-flag is-up">▼ low</span> : null}
                  {isRichest ? <span className="venue-tile-flag is-down">▲ high</span> : null}
                </div>
                <div className="venue-tile-priceline">
                  <span className="venue-tile-price">{formatPreciseUsd(venue.priceUsd, 2)}</span>
                  <span className="venue-tile-tag">{formatSignedPercent(deviationPct, 3)}</span>
                </div>
                <DeviationBar
                  value={deviationPct}
                  range={maxDeviationPct}
                  height={3}
                />
              </div>
            );
          })
        )}
      </div>
    </Card>
  );
}
