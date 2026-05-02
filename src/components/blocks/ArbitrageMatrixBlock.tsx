import { Card } from "../primitives/Card";
import { Heatmap } from "../primitives/Heatmap";
import { GAS_UNITS_ESTIMATE } from "../../lib/config";
import { formatSignedUsd } from "../../lib/format";
import { median } from "../../lib/stats";
import { shortenVenueName } from "../../lib/venues";
import type { BlockRenderProps } from "./BlockRegistry";

/**
 * NxN gas-adjusted arbitrage matrix. Cell (buy = i, sell = j) shows
 * `price[j] - price[i] - gas_cost`. Diagonal is NaN. Positive cells = green,
 * negative = red. Gives a one-glance view of which buy→sell route is the
 * best opportunity right now.
 */
export function ArbitrageMatrixBlock({ market, onRemove }: BlockRenderProps) {
  const { overview } = market;
  const venues = overview?.venues ?? [];

  if (venues.length === 0 || !overview) {
    return (
      <Card title="Arb matrix" subtitle="—" onRemove={onRemove}>
        <div className="card-empty">Awaiting first sample.</div>
      </Card>
    );
  }

  const prices = venues.map((venue) => venue.priceUsd);
  const medianPrice = median(prices);
  const gasCostUsd =
    (overview.gasPriceGwei * GAS_UNITS_ESTIMATE * medianPrice) / 1_000_000_000;

  const labels = venues.map((venue) => shortenVenueName(venue.dexName));
  const data: number[][] = venues.map((buy, i) =>
    venues.map((sell, j) => {
      if (i === j) return Number.NaN;
      return sell.priceUsd - buy.priceUsd - gasCostUsd;
    }),
  );

  const positive = data.flat().filter((value) => Number.isFinite(value) && value > 0);
  const best = positive.length > 0 ? Math.max(...positive) : 0;

  return (
    <Card
      title="Arb matrix"
      subtitle={`gas −${overview.gasPriceGwei.toFixed(0)}g`}
      onRemove={onRemove}
    >
      <div className="arb-matrix-content">
        <div className="arb-matrix-stage">
          <Heatmap
            data={data}
            symmetricRange={Math.max(best * 1.2, 0.01)}
            rowLabels={labels.map((label) => `↘ ${label}`)}
            colLabels={labels}
            cellGap={2}
          />
        </div>
        <div className="arb-matrix-footer">
          <span className="mono" style={{ color: "var(--text-muted)" }}>
            buy ↓ sell →
          </span>
          <span
            className="mono"
            style={{
              color: best > 0 ? "var(--status-up)" : "var(--text-muted)",
            }}
          >
            best {best > 0 ? formatSignedUsd(best) : "—"}
          </span>
        </div>
      </div>
    </Card>
  );
}
