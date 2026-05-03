import { Card } from "../primitives/Card";
import { RatioBar } from "../primitives/RatioBar";
import { Sparkline } from "../primitives/Sparkline";
import { formatPreciseUsd } from "../../lib/format";
import type { EquityCurvePoint, PositionRunSummary } from "../../features/lp-backtest/types";

interface PositionPnlBlockProps {
  summary: PositionRunSummary | null;
  curve: EquityCurvePoint[];
  onRemove?: () => void;
}

export function PositionPnlBlock({
  summary,
  curve,
  onRemove,
}: PositionPnlBlockProps) {
  const fees = summary?.total_fees_usd ?? 0;
  const ilAbs = Math.abs(summary?.total_il_usd ?? 0);
  const lvr = summary?.total_lvr_usd ?? 0;
  const gas = summary?.total_mgmt_gas_usd ?? 0;

  const feesSeries = curve.map((p) => p.fees_accumulated_usd);
  const ilSeries = curve.map((p) => p.il_usd);
  const lvrSeries = curve.map((p) => p.lvr_usd);
  const gasSeries = curve.map((p) => p.mgmt_gas_paid_usd);

  return (
    <Card title="P&L sources" subtitle="cumulative" onRemove={onRemove}>
      <div className="pnl-stack">
        <div className="pnl-quad">
          <PnlCell
            label="Fees"
            value={summary ? formatPreciseUsd(fees, 4) : "—"}
            tone="up"
            series={feesSeries}
          />
          <PnlCell
            label="IL"
            value={summary ? formatPreciseUsd(summary.total_il_usd, 4) : "—"}
            tone={(summary?.total_il_usd ?? 0) < 0 ? "down" : "neutral"}
            series={ilSeries}
          />
          <PnlCell
            label="LVR"
            value={summary ? formatPreciseUsd(lvr, 4) : "—"}
            tone="warn"
            series={lvrSeries}
          />
          <PnlCell
            label="Mgmt gas"
            value={summary ? formatPreciseUsd(gas, 4) : "—"}
            tone="info"
            series={gasSeries}
          />
        </div>

        <div className="pnl-ratio-row">
          <span className="pnl-ratio-label">Composition</span>
          <RatioBar
            segments={[
              { value: fees, tone: "up", label: "Fees" },
              { value: ilAbs, tone: "down", label: "IL" },
              { value: lvr, tone: "warn", label: "LVR" },
              { value: gas, tone: "info", label: "Mgmt gas" },
            ]}
            height={6}
          />
        </div>
      </div>
    </Card>
  );
}

function PnlCell({
  label,
  value,
  tone,
  series,
}: {
  label: string;
  value: string;
  tone: "up" | "down" | "warn" | "info" | "neutral";
  series: number[];
}) {
  const sparkTone = tone === "neutral" ? "info" : tone;
  return (
    <div className="pnl-cell">
      <span className="pnl-cell-label">{label}</span>
      <span className={`pnl-cell-value mono is-${tone}`}>{value}</span>
      <div className="pnl-cell-spark">
        {series.length > 1 ? (
          <Sparkline values={series} tone={sparkTone} filled height={20} />
        ) : (
          <div style={{ height: 20 }} />
        )}
      </div>
    </div>
  );
}
