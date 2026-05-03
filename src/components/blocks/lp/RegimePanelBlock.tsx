import { Card } from "../../primitives/Card";
import { Pill } from "../../primitives/Pill";
import type { HeadlineMonthlyRow } from "../../../features/lp-backtest/types";

interface RegimePanelBlockProps {
  rows: HeadlineMonthlyRow[];
  onRemove?: () => void;
}

const REGIME_COLOUR: Record<string, string> = {
  low: "var(--secondary)",
  medium: "var(--status-warn)",
  high: "var(--status-up)",
};

function fmtPct(v: number): string {
  return `${v >= 0 ? "+" : ""}${(v * 100).toFixed(2)}%`;
}

function tone(value: number): "up" | "down" | "neutral" {
  if (value > 0.001) return "up";
  if (value < -0.001) return "down";
  return "neutral";
}

export function RegimePanelBlock({ rows, onRemove }: RegimePanelBlockProps) {
  if (!rows.length) {
    return (
      <Card title="Regime panel" subtitle="LP vs lending by vol regime" onRemove={onRemove}>
        <div className="lp-card-empty">Headline analysis hasn't run yet.</div>
      </Card>
    );
  }
  const sorted = [...rows].sort((a, b) =>
    a.yearMonth < b.yearMonth ? -1 : a.yearMonth > b.yearMonth ? 1 : 0,
  );
  const maxAbsSpread = Math.max(
    1e-9,
    ...sorted.map((r) =>
      Math.abs(r.bestLpReturn - Math.max(r.aaveUsdcReturn, r.lidoStethReturn)),
    ),
  );

  return (
    <Card
      title="Regime panel"
      subtitle={`${rows.length} months · LP vs lending`}
      onRemove={onRemove}
    >
      <div className="regime-stack">
        <div className="regime-bar-row">
          {sorted.map((r) => {
            const lending = Math.max(r.aaveUsdcReturn, r.lidoStethReturn);
            const spread = r.bestLpReturn - lending;
            const widthPct = Math.min(100, (Math.abs(spread) / maxAbsSpread) * 100);
            const isPositive = spread >= 0;
            return (
              <div
                key={r.yearMonth}
                className="regime-bar-cell"
                title={`${r.yearMonth}\nregime=${r.volRegime}\nLP-vs-lending=${fmtPct(spread)}`}
              >
                <span
                  className="regime-bar-month-tag mono"
                  style={{ background: REGIME_COLOUR[r.volRegime] ?? "var(--text-muted)" }}
                >
                  {r.yearMonth.slice(2)}
                </span>
                <div className="regime-bar-stage">
                  <div className="regime-bar-track" />
                  <div className="regime-bar-axis" />
                  <div
                    className={`regime-bar-fill ${isPositive ? "is-positive" : "is-negative"}`}
                    style={{
                      width: `${widthPct / 2}%`,
                      [isPositive ? "left" : "right"]: "50%",
                    }}
                  />
                </div>
                <span className={`regime-bar-value mono is-${tone(spread)}`}>
                  {fmtPct(spread)}
                </span>
              </div>
            );
          })}
        </div>

        <div className="regime-legend">
          <RegimeLegend label="Low vol" colour={REGIME_COLOUR.low} />
          <RegimeLegend label="Mid vol" colour={REGIME_COLOUR.medium} />
          <RegimeLegend label="High vol" colour={REGIME_COLOUR.high} />
          <span className="regime-legend-spacer" />
          <Pill tone="up">positive = LP wins</Pill>
          <Pill tone="down">negative = lending wins</Pill>
        </div>
      </div>
    </Card>
  );
}

function RegimeLegend({ label, colour }: { label: string; colour: string }) {
  return (
    <span className="regime-legend-item">
      <span className="regime-legend-swatch" style={{ background: colour }} />
      {label}
    </span>
  );
}
