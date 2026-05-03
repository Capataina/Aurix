import { Card } from "../../primitives/Card";
import { Heatmap } from "../../primitives/Heatmap";
import { computeGasCostUsd, computeRoute } from "../../../lib/arbitrage";
import { formatSignedUsd } from "../../../lib/format";
import { median } from "../../../lib/stats";
import { shortenVenueName } from "../../../lib/venues";
import type { BlockRenderProps } from "./BlockRegistry";

/**
 * NxN gas-(and optionally fee-)adjusted arbitrage matrix. Every cell is the
 * net P/L of buying at the row venue and selling at the column venue, using
 * the active pnlMode. Diagonal is NaN. Best cell highlighted in the footer.
 */
export function ArbitrageMatrixBlock({ market, pnlMode, onRemove }: BlockRenderProps) {
  const { overview } = market;
  const venues = overview?.venues ?? [];

  if (venues.length === 0 || !overview) {
    return (
      <Card title="Arb matrix" subtitle="—" onRemove={onRemove}>
        <div className="card-empty">Awaiting first sample.</div>
      </Card>
    );
  }

  const referencePrice = median(venues.map((venue) => venue.priceUsd));
  const gasCostUsd = computeGasCostUsd(overview.gasPriceGwei, referencePrice);

  const labels = venues.map((venue) => shortenVenueName(venue.dexName));
  const data: number[][] = venues.map((buy, i) =>
    venues.map((sell, j) => {
      if (i === j) return Number.NaN;
      return computeRoute(buy, sell, i, j, gasCostUsd, pnlMode).netUsd;
    }),
  );

  const positive = data
    .flat()
    .filter((value) => Number.isFinite(value) && value > 0);
  const best = positive.length > 0 ? Math.max(...positive) : 0;

  const subtitleParts: string[] = [`gas ${overview.gasPriceGwei.toFixed(2)}g`];
  if (pnlMode === "gas-and-fees") subtitleParts.push("+ fees");
  const subtitle = subtitleParts.join(" ");

  return (
    <Card title="Arb matrix" subtitle={subtitle} onRemove={onRemove}>
      <div className="arb-matrix-content">
        <div className="arb-matrix-stage">
          <Heatmap
            data={data}
            symmetricRange={Math.max(Math.abs(best) * 1.2, 0.01)}
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
