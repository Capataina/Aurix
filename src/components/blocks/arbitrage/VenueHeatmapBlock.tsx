import { Card } from "../../primitives/Card";
import { Heatmap } from "../../primitives/Heatmap";
import { median } from "../../../lib/stats";
import { shortenVenueName } from "../../../lib/venues";
import type { BlockRenderProps } from "./BlockRegistry";

const MAX_COLS = 60;

/**
 * Per-venue per-tick deviation heatmap. Rows are venues, columns are recent
 * sampling ticks (oldest left), cells coloured by deviation from the
 * cross-venue median for that tick. Reveals temporal patterns — sustained
 * over- or under-pricing, regime shifts, transient spikes.
 */
export function VenueHeatmapBlock({ market, onRemove }: BlockRenderProps) {
  const { history } = market;

  if (history.length < 2) {
    return (
      <Card title="Heatmap" subtitle="venue × tick" onRemove={onRemove}>
        <div className="card-empty">Awaiting at least 2 samples.</div>
      </Card>
    );
  }

  const venueNames = history[0].venues.map((venue) => venue.dexName);
  const trimmed = history.slice(-MAX_COLS);

  const data: number[][] = venueNames.map((venueName) =>
    trimmed.map((entry) => {
      const prices = entry.venues.map((venue) => venue.priceUsd);
      const med = median(prices);
      const venue = entry.venues.find((candidate) => candidate.dexName === venueName);
      return venue ? ((venue.priceUsd - med) / med) * 100 : 0;
    }),
  );

  const flat = data.flat();
  const range =
    flat.length > 0
      ? Math.max(...flat.map((value) => Math.abs(value)), 0.01)
      : 0.01;

  return (
    <Card title="Heatmap" subtitle={`${trimmed.length} ticks`} onRemove={onRemove}>
      <div className="heatmap-content">
        <Heatmap
          data={data}
          symmetricRange={range}
          rowLabels={venueNames.map((name) => shortenVenueName(name))}
          cellGap={1}
        />
      </div>
    </Card>
  );
}
