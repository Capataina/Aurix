import { Card } from "../../../components/primitives/Card";
import { formatUsd, formatSignedPercent } from "../../../lib/format";
import type { EquityCurvePoint, PositionRunSummary } from "../types";

interface EquityCurveBlockProps {
  summary: PositionRunSummary | null;
  curve: EquityCurvePoint[];
  onRemove?: () => void;
}

const HEIGHT = 220;
const PADDING = 24;

interface Series {
  label: string;
  values: number[];
  colour: string;
}

function buildSeries(curve: EquityCurvePoint[]): Series[] {
  if (!curve.length) return [];
  return [
    {
      label: "Position",
      values: curve.map((p) => p.position_value_usd),
      colour: "var(--accent)",
    },
    {
      label: "Hold-only",
      values: curve.map((p) => p.hold_only_value_usd),
      colour: "var(--text-muted)",
    },
    {
      label: "Fees",
      values: curve.map((p) => p.fees_accumulated_usd),
      colour: "var(--accent-soft)",
    },
  ];
}

function pathFor(values: number[], minY: number, maxY: number, w: number): string {
  if (!values.length) return "";
  const span = Math.max(1e-12, maxY - minY);
  const stepX = w / Math.max(1, values.length - 1);
  let d = "";
  values.forEach((v, i) => {
    const x = i * stepX;
    const y = HEIGHT - PADDING - ((v - minY) / span) * (HEIGHT - PADDING * 2);
    d += i === 0 ? `M${x.toFixed(1)} ${y.toFixed(1)}` : ` L${x.toFixed(1)} ${y.toFixed(1)}`;
  });
  return d;
}

export function EquityCurveBlock({
  summary,
  curve,
  onRemove,
}: EquityCurveBlockProps) {
  const subtitle = summary
    ? `${summary.rebalance_count} rebalances · ${summary.time_in_range_pct.toFixed(1)}% in range`
    : "Run a backtest to populate the equity curve";

  if (!curve.length || !summary) {
    return (
      <Card title="Equity curve" subtitle={subtitle} onRemove={onRemove}>
        <div className="lp-empty">No equity curve yet. Configure a position and click Run backtest.</div>
      </Card>
    );
  }

  const allValues = [
    ...curve.map((p) => p.position_value_usd),
    ...curve.map((p) => p.hold_only_value_usd),
  ];
  const minY = Math.min(...allValues);
  const maxY = Math.max(...allValues);
  const width = 720;
  const series = buildSeries(curve);
  const totalReturnPct = (summary.net_pnl_usd / summary.hold_only_value_usd) * 100;

  return (
    <Card title="Equity curve" subtitle={subtitle} onRemove={onRemove}>
      <div className="lp-equity">
        <div className="lp-equity-stats">
          <div className="lp-equity-stat">
            <span className="lp-equity-stat-label">Final value</span>
            <span className="lp-equity-stat-value">
              {formatUsd(summary.final_value_usd)}
            </span>
          </div>
          <div className="lp-equity-stat">
            <span className="lp-equity-stat-label">Net P&amp;L</span>
            <span className="lp-equity-stat-value">
              {formatSignedPercent(totalReturnPct)}
            </span>
          </div>
          <div className="lp-equity-stat">
            <span className="lp-equity-stat-label">Fees</span>
            <span className="lp-equity-stat-value">
              {formatUsd(summary.total_fees_usd)}
            </span>
          </div>
          <div className="lp-equity-stat">
            <span className="lp-equity-stat-label">IL</span>
            <span className="lp-equity-stat-value">
              {formatUsd(summary.total_il_usd)}
            </span>
          </div>
          <div className="lp-equity-stat">
            <span className="lp-equity-stat-label">Mgmt gas</span>
            <span className="lp-equity-stat-value">
              {formatUsd(summary.total_mgmt_gas_usd)}
            </span>
          </div>
          <div className="lp-equity-stat">
            <span className="lp-equity-stat-label">Sharpe</span>
            <span className="lp-equity-stat-value">
              {summary.sharpe.toFixed(2)}
            </span>
          </div>
          <div className="lp-equity-stat">
            <span className="lp-equity-stat-label">Sortino</span>
            <span className="lp-equity-stat-value">
              {summary.sortino.toFixed(2)}
            </span>
          </div>
          <div className="lp-equity-stat">
            <span className="lp-equity-stat-label">Max DD</span>
            <span className="lp-equity-stat-value">
              {summary.max_drawdown_pct.toFixed(1)}%
            </span>
          </div>
        </div>
        <svg
          className="lp-equity-chart"
          viewBox={`0 0 ${width} ${HEIGHT}`}
          preserveAspectRatio="none"
          role="img"
          aria-label="Equity curve chart"
        >
          {series.map((s) => (
            <path
              key={s.label}
              d={pathFor(s.values, minY, maxY, width)}
              fill="none"
              stroke={s.colour}
              strokeWidth={1.5}
            />
          ))}
        </svg>
        <div className="lp-equity-legend">
          {series.map((s) => (
            <span key={s.label} className="lp-equity-legend-item">
              <span className="lp-equity-legend-swatch" style={{ background: s.colour }} />
              {s.label}
            </span>
          ))}
        </div>
      </div>
    </Card>
  );
}
