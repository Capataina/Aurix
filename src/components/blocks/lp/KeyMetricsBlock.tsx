import { Card } from "../../primitives/Card";
import { Dial } from "../../primitives/Dial";
import type { PositionRunSummary } from "../../../features/lp-backtest/types";

interface KeyMetricsBlockProps {
  summary: PositionRunSummary | null;
  onRemove?: () => void;
}

/** Map a Sharpe value (typical range -3 to 3) to a 0-1 dial fill,
 *  centred on Sharpe = 0 → 0.5. */
function sharpeToFill(s: number): number {
  return Math.max(0, Math.min(1, (s + 3) / 6));
}

function ddToFill(ddPct: number): number {
  // 0% → full dial, 50%+ → empty dial. Inverted so "less DD = more fill".
  return Math.max(0, Math.min(1, 1 - ddPct / 50));
}

function timeInRangeToFill(pct: number): number {
  return Math.max(0, Math.min(1, pct / 100));
}

export function KeyMetricsBlock({ summary, onRemove }: KeyMetricsBlockProps) {
  return (
    <Card title="Risk-adjusted" subtitle="annualised" onRemove={onRemove}>
      <div className="metrics-row">
        <Dial
          value={summary ? sharpeToFill(summary.sharpe) : 0}
          label={summary ? summary.sharpe.toFixed(2) : "—"}
          sublabel="Sharpe"
          tone={summary && summary.sharpe >= 1 ? "up" : summary && summary.sharpe < 0 ? "down" : "info"}
          size={84}
        />
        <Dial
          value={summary ? sharpeToFill(summary.sortino) : 0}
          label={summary ? summary.sortino.toFixed(2) : "—"}
          sublabel="Sortino"
          tone={summary && summary.sortino >= 1 ? "up" : summary && summary.sortino < 0 ? "down" : "info"}
          size={84}
        />
        <Dial
          value={summary ? ddToFill(summary.max_drawdown_pct) : 0}
          label={summary ? `${summary.max_drawdown_pct.toFixed(1)}%` : "—"}
          sublabel="Max DD"
          tone={
            summary && summary.max_drawdown_pct < 5
              ? "up"
              : summary && summary.max_drawdown_pct > 20
                ? "down"
                : "warn"
          }
          size={84}
        />
        <Dial
          value={summary ? timeInRangeToFill(summary.time_in_range_pct) : 0}
          label={summary ? `${summary.time_in_range_pct.toFixed(0)}%` : "—"}
          sublabel="In range"
          tone={summary && summary.time_in_range_pct > 80 ? "up" : "warn"}
          size={84}
        />
      </div>
    </Card>
  );
}
