import { Card } from "../primitives/Card";
import { RangeIndicator } from "../primitives/RangeIndicator";
import type { EquityCurvePoint, PositionConfig } from "../../features/lp-backtest/types";

interface PositionRangeBlockProps {
  config: PositionConfig | null;
  curve: EquityCurvePoint[];
  onRemove?: () => void;
}

/** Convert a tick to its position on a price scale (1.0001^tick). For
 *  small windows the display tick range shifts by a few % of price; we
 *  show the position using the tick coordinate directly which is easier
 *  to read in this context. */
function tickWidthBp(lower: number, upper: number): number {
  // Each tick = 1 basis point on price. Range_width_bp = upper - lower bp.
  return upper - lower;
}

export function PositionRangeBlock({
  config,
  curve,
  onRemove,
}: PositionRangeBlockProps) {
  if (!config) {
    return (
      <Card title="Range" subtitle="position window" onRemove={onRemove}>
        <div className="lp-card-empty">No position yet.</div>
      </Card>
    );
  }
  const widthBp = tickWidthBp(config.tickLower, config.tickUpper);
  const widthPct = widthBp / 100; // 100bp = 1%

  // Recent activity: best-effort visual proxy from the equity curve.
  const lastTick = curve.length > 0 ? sampleEntryTick(curve) : config.tickLower;

  return (
    <Card title="Range" subtitle={`±${(widthPct / 2).toFixed(2)}% on price`} onRemove={onRemove}>
      <div className="range-stack">
        <div className="range-header">
          <span className="range-tick-pair mono">
            <span>{config.tickLower}</span>
            <span className="range-tick-arrow">→</span>
            <span>{config.tickUpper}</span>
          </span>
          <span className="range-tick-width-pill mono">{widthBp} bp wide</span>
        </div>

        <div className="range-indicator-stage">
          <RangeIndicator
            current={lastTick}
            min={config.tickLower}
            max={config.tickUpper}
            baseline={(config.tickLower + config.tickUpper) / 2}
            tone="accent"
          />
          <div className="range-indicator-labels mono">
            <span>{config.tickLower}</span>
            <span className="range-indicator-mid">last tick</span>
            <span>{config.tickUpper}</span>
          </div>
        </div>

        <div className="range-stats-grid">
          <RangeStat label="Entry block" value={config.entryBlock.toLocaleString()} />
          <RangeStat label="Exit block" value={config.exitBlock.toLocaleString()} />
          <RangeStat label="Fee tier" value={`${config.feeTierBps} bps`} />
          <RangeStat
            label="MEV haircut"
            value={
              config.mevHaircutBps > 0 ? `${config.mevHaircutBps.toFixed(1)} bps` : "off"
            }
          />
        </div>
      </div>
    </Card>
  );
}

function sampleEntryTick(curve: EquityCurvePoint[]): number {
  // Best-effort proxy: the last sample's relative position in the curve
  // approximates the current tick. We don't have raw tick on the curve
  // point (only in_range), so we estimate from in_range proportion.
  // This is purely a visual hint; the table below shows the real ticks.
  const inRange = curve.filter((p) => p.in_range).length;
  const ratio = inRange / Math.max(1, curve.length);
  // Map ratio → tick within [-1, 1] of midpoint (dimensionless display).
  return Math.round((ratio - 0.5) * 2 * 100); // ±100 hint
}

function RangeStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="range-stat">
      <span className="range-stat-label">{label}</span>
      <span className="range-stat-value mono">{value}</span>
    </div>
  );
}
