import { useMemo } from "react";

import { Card } from "../primitives/Card";
import { Pill } from "../primitives/Pill";
import { formatSignedPercent, formatPreciseUsd } from "../../lib/format";
import type { EquityCurvePoint, PositionRunSummary } from "../../features/lp-backtest/types";

interface EquityCurveBlockProps {
  summary: PositionRunSummary | null;
  curve: EquityCurvePoint[];
  onRemove?: () => void;
}

const VIEW_WIDTH = 1000;
const VIEW_HEIGHT = 240;
const PAD_TOP = 12;
const PAD_BOTTOM = 18;
const PAD_LEFT = 0;
const PAD_RIGHT = 0;

interface SeriesSpec {
  key: keyof EquityCurvePoint;
  label: string;
  colour: string;
  filled: boolean;
}

const SERIES: SeriesSpec[] = [
  {
    key: "position_value_usd",
    label: "Position",
    colour: "var(--accent)",
    filled: true,
  },
  {
    key: "hold_only_value_usd",
    label: "Hold-only",
    colour: "var(--text-muted)",
    filled: false,
  },
];

export function EquityCurveBlock({
  summary,
  curve,
  onRemove,
}: EquityCurveBlockProps) {
  const stats = useMemo(() => {
    if (!summary || curve.length === 0) return null;
    const totalReturnPct =
      summary.hold_only_value_usd === 0
        ? 0
        : ((summary.final_value_usd - summary.hold_only_value_usd) /
            summary.hold_only_value_usd) *
          100;
    return {
      totalReturnPct,
      finalUsd: summary.final_value_usd,
      holdUsd: summary.hold_only_value_usd,
      feesUsd: summary.total_fees_usd,
      ilUsd: summary.total_il_usd,
      mgmtUsd: summary.total_mgmt_gas_usd,
    };
  }, [summary, curve.length]);

  const paths = useMemo(() => {
    if (curve.length < 2) return null;
    const allValues = curve.flatMap((p) => [
      p.position_value_usd,
      p.hold_only_value_usd,
    ]);
    const min = Math.min(...allValues);
    const max = Math.max(...allValues);
    const span = Math.max(1e-9, max - min);
    const w = VIEW_WIDTH - PAD_LEFT - PAD_RIGHT;
    const h = VIEW_HEIGHT - PAD_TOP - PAD_BOTTOM;
    const stepX = w / (curve.length - 1);

    const buildPath = (values: number[]): { line: string; area: string } => {
      let line = "";
      values.forEach((v, i) => {
        const x = PAD_LEFT + i * stepX;
        const y = PAD_TOP + h - ((v - min) / span) * h;
        line += i === 0 ? `M${x.toFixed(2)} ${y.toFixed(2)}` : ` L${x.toFixed(2)} ${y.toFixed(2)}`;
      });
      const lastX = PAD_LEFT + (values.length - 1) * stepX;
      const baseY = PAD_TOP + h;
      const area = `${line} L${lastX.toFixed(2)} ${baseY.toFixed(2)} L${PAD_LEFT.toFixed(2)} ${baseY.toFixed(2)} Z`;
      return { line, area };
    };

    return {
      series: SERIES.map((spec) => ({
        spec,
        ...buildPath(curve.map((p) => p[spec.key] as number)),
      })),
      // Out-of-range overlay: vertical band wherever in_range is false.
      oorBands: buildOorBands(curve, stepX, w, PAD_TOP, h),
    };
  }, [curve]);

  const tone =
    stats === null
      ? "neutral"
      : stats.totalReturnPct > 0
        ? "up"
        : stats.totalReturnPct < 0
          ? "down"
          : "neutral";

  return (
    <Card
      title="Equity curve"
      subtitle={
        summary
          ? `${curve.length} samples · ${summary.rebalance_count} rebalances`
          : "auto-run pending"
      }
      onRemove={onRemove}
      headerExtra={
        stats ? (
          <Pill tone={tone}>
            {formatSignedPercent(stats.totalReturnPct, 2)}
          </Pill>
        ) : null
      }
    >
      <div className="equity-block">
        <div className="equity-stats-row">
          <EquityStat
            label="Final"
            value={stats ? formatPreciseUsd(stats.finalUsd, 4) : "—"}
            mono
          />
          <EquityStat
            label="Hold"
            value={stats ? formatPreciseUsd(stats.holdUsd, 4) : "—"}
            mono
          />
          <EquityStat
            label="Fees"
            value={stats ? formatPreciseUsd(stats.feesUsd, 4) : "—"}
            mono
            tone="up"
          />
          <EquityStat
            label="IL"
            value={stats ? formatPreciseUsd(stats.ilUsd, 4) : "—"}
            mono
            tone={stats && stats.ilUsd < 0 ? "down" : "neutral"}
          />
          <EquityStat
            label="Mgmt gas"
            value={stats ? formatPreciseUsd(stats.mgmtUsd, 4) : "—"}
            mono
            tone="warn"
          />
        </div>

        <div className="equity-chart-stage">
          {paths === null ? (
            <div className="lp-card-empty">No equity curve data yet.</div>
          ) : (
            <svg
              className="equity-chart"
              viewBox={`0 0 ${VIEW_WIDTH} ${VIEW_HEIGHT}`}
              preserveAspectRatio="none"
              role="img"
              aria-label="Equity curve chart"
            >
              {/* OOR bands behind everything */}
              {paths.oorBands.map((band, idx) => (
                <rect
                  key={`oor-${idx}`}
                  x={band.x}
                  y={band.y}
                  width={band.width}
                  height={band.height}
                  fill="var(--status-down)"
                  opacity={0.04}
                />
              ))}
              {/* Filled areas */}
              {paths.series.map((s) =>
                s.spec.filled ? (
                  <path
                    key={`fill-${s.spec.key}`}
                    d={s.area}
                    fill={s.spec.colour}
                    opacity={0.12}
                  />
                ) : null,
              )}
              {/* Lines */}
              {paths.series.map((s) => (
                <path
                  key={`line-${s.spec.key}`}
                  d={s.line}
                  fill="none"
                  stroke={s.spec.colour}
                  strokeWidth={s.spec.filled ? 1.6 : 1.2}
                  strokeOpacity={s.spec.filled ? 1 : 0.85}
                  strokeDasharray={s.spec.filled ? undefined : "3 3"}
                />
              ))}
            </svg>
          )}
        </div>

        <div className="equity-legend-row">
          {SERIES.map((s) => (
            <span key={s.key} className="equity-legend-item">
              <span
                className="equity-legend-swatch"
                style={{ background: s.colour, opacity: s.filled ? 1 : 0.7 }}
              />
              {s.label}
            </span>
          ))}
          <span className="equity-legend-item is-trail">
            <span className="equity-legend-swatch is-oor" />
            Out-of-range bands
          </span>
        </div>
      </div>
    </Card>
  );
}

function EquityStat({
  label,
  value,
  mono = false,
  tone,
}: {
  label: string;
  value: string;
  mono?: boolean;
  tone?: "up" | "down" | "warn" | "neutral";
}) {
  const valueClass =
    `equity-stat-value` + (mono ? " mono" : "") + (tone ? ` is-${tone}` : "");
  return (
    <div className="equity-stat">
      <span className="equity-stat-label">{label}</span>
      <span className={valueClass}>{value}</span>
    </div>
  );
}

interface OorBand {
  x: number;
  y: number;
  width: number;
  height: number;
}

function buildOorBands(
  curve: EquityCurvePoint[],
  stepX: number,
  w: number,
  padTop: number,
  h: number,
): OorBand[] {
  const bands: OorBand[] = [];
  let bandStart: number | null = null;
  curve.forEach((p, i) => {
    if (!p.in_range && bandStart === null) {
      bandStart = i;
    } else if (p.in_range && bandStart !== null) {
      const x0 = PAD_LEFT + bandStart * stepX;
      const x1 = PAD_LEFT + i * stepX;
      bands.push({ x: x0, y: padTop, width: x1 - x0, height: h });
      bandStart = null;
    }
  });
  if (bandStart !== null) {
    const x0 = PAD_LEFT + bandStart * stepX;
    const x1 = PAD_LEFT + w;
    bands.push({ x: x0, y: padTop, width: x1 - x0, height: h });
  }
  return bands;
}
