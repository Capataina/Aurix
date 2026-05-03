import { Card } from "../../primitives/Card";
import { DeviationBar } from "../../primitives/DeviationBar";
import { Sparkline } from "../../primitives/Sparkline";
import { formatSignedPercent } from "../../../lib/format";
import type { BlockRenderProps } from "./BlockRegistry";

const SHORT_WINDOW = 10;

/**
 * Short-window price momentum — percent change over the last N ticks. A big
 * arrow + signed value + tiny sparkline make direction and magnitude
 * readable in well under a second.
 */
export function MomentumBlock({ market, onRemove }: BlockRenderProps) {
  const { history } = market;

  if (history.length < 2) {
    return (
      <Card title="Momentum" subtitle={`${SHORT_WINDOW}-tick`} onRemove={onRemove}>
        <div className="card-empty">Awaiting samples.</div>
      </Card>
    );
  }

  const heroSeries = history.map((entry) => entry.venues[0]?.priceUsd ?? 0);
  const recent = heroSeries.slice(-SHORT_WINDOW);
  const start = recent[0];
  const end = recent[recent.length - 1];
  const changePct = start === 0 ? 0 : ((end - start) / start) * 100;

  // Range scaling: largest |Δ%| over the rolling window — keeps the bar
  // calibrated even when the asset is in a quiet regime.
  const allChanges = heroSeries.slice(0, -1).map((price, idx) => {
    const next = heroSeries[idx + 1];
    return price === 0 ? 0 : ((next - price) / price) * 100;
  });
  const rollingExtreme = Math.max(...allChanges.map((v) => Math.abs(v)), 0.001);

  const tone = changePct > 0 ? "up" : changePct < 0 ? "down" : "neutral";

  return (
    <Card title="Momentum" subtitle={`${SHORT_WINDOW}-tick`} onRemove={onRemove}>
      <div className="momentum-content">
        <div className="momentum-headline">
          <span
            className="momentum-arrow"
            style={{
              color:
                tone === "up"
                  ? "var(--status-up)"
                  : tone === "down"
                    ? "var(--status-down)"
                    : "var(--text-muted)",
            }}
          >
            {tone === "up" ? "▲" : tone === "down" ? "▼" : "▬"}
          </span>
          <span
            className={`metric-value is-large ${
              tone === "up" ? "is-up" : tone === "down" ? "is-down" : "is-muted"
            }`}
          >
            {formatSignedPercent(changePct, 3)}
          </span>
        </div>

        <DeviationBar value={changePct} range={rollingExtreme} height={6} />

        <div style={{ flex: 1, minHeight: 32 }}>
          <Sparkline values={recent} tone={tone === "down" ? "down" : "up"} filled />
        </div>
      </div>
    </Card>
  );
}
