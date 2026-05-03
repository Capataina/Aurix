import { useMemo, useState } from "react";

import { Card } from "../../primitives/Card";
import { MicroBar } from "../../primitives/MicroBar";
import { Pill } from "../../primitives/Pill";
import { formatPreciseUsd } from "../../../lib/format";
import type { StrategyResultRow } from "../../../features/lp-backtest/types";

interface StrategyHeatmapBlockProps {
  rows: StrategyResultRow[];
  onRemove?: () => void;
}

type SortKey =
  | "sharpe"
  | "sortino"
  | "deflatedSharpe"
  | "netReturnUsd"
  | "feesUsd";

const SORT_OPTIONS: Array<{ key: SortKey; label: string }> = [
  { key: "deflatedSharpe", label: "Deflated Sharpe" },
  { key: "sharpe", label: "Sharpe" },
  { key: "sortino", label: "Sortino" },
  { key: "netReturnUsd", label: "Net" },
  { key: "feesUsd", label: "Fees" },
];

function compactRule(serialised: string): string {
  try {
    const parsed = JSON.parse(serialised) as { kind: string; [k: string]: unknown };
    if (parsed.kind === "static") return "static";
    if (parsed.kind === "schedule") return `sched/${parsed.every_n_blocks}`;
    if (parsed.kind === "price_exit_threshold")
      return `exit ${(parsed.central_pct as number).toFixed(2)}`;
    if (parsed.kind === "out_of_range_duration")
      return `oor/${parsed.min_oor_blocks}`;
    return parsed.kind;
  } catch {
    return serialised;
  }
}

export function StrategyHeatmapBlock({
  rows,
  onRemove,
}: StrategyHeatmapBlockProps) {
  const [sortKey, setSortKey] = useState<SortKey>("deflatedSharpe");

  const sorted = useMemo(
    () =>
      [...rows].sort((a, b) => (b[sortKey] as number) - (a[sortKey] as number)),
    [rows, sortKey],
  );

  const sharpeBounds = useMemo(() => {
    if (rows.length === 0) return { min: 0, max: 0 };
    const values = rows.map((r) => r.sharpe);
    return { min: Math.min(...values), max: Math.max(...values) };
  }, [rows]);

  const netBounds = useMemo(() => {
    if (rows.length === 0) return { min: 0, max: 0 };
    const values = rows.map((r) => r.netReturnUsd);
    return { min: Math.min(...values), max: Math.max(...values) };
  }, [rows]);

  if (rows.length === 0) {
    return (
      <Card title="Strategy grid" subtitle="range × rule × deposit × period" onRemove={onRemove}>
        <div className="lp-card-empty">No grid results yet — auto-running.</div>
      </Card>
    );
  }

  const top = sorted.slice(0, 8);
  const best = top[0];
  const worst = sorted[sorted.length - 1];

  return (
    <Card
      title="Strategy grid"
      subtitle={`${rows.length} cells · top by ${SORT_OPTIONS.find((o) => o.key === sortKey)?.label}`}
      onRemove={onRemove}
      headerExtra={
        <select
          className="ctrl-mini-select"
          value={sortKey}
          onChange={(e) => setSortKey(e.target.value as SortKey)}
        >
          {SORT_OPTIONS.map((o) => (
            <option key={o.key} value={o.key}>
              {o.label}
            </option>
          ))}
        </select>
      }
    >
      <div className="grid-stack">
        <div className="grid-headline">
          <div className="grid-headline-cell">
            <span className="grid-headline-label">Best</span>
            <span className="grid-headline-rule mono">
              {compactRule(best.rebalanceRule)} · {best.rangeWidthPct.toFixed(1)}%
            </span>
            <Pill tone={best.sharpe > 0 ? "up" : "down"}>
              Sharpe {best.sharpe.toFixed(2)}
            </Pill>
          </div>
          <div className="grid-headline-cell">
            <span className="grid-headline-label">Worst</span>
            <span className="grid-headline-rule mono">
              {compactRule(worst.rebalanceRule)} · {worst.rangeWidthPct.toFixed(1)}%
            </span>
            <Pill tone={worst.sharpe > 0 ? "neutral" : "down"}>
              Sharpe {worst.sharpe.toFixed(2)}
            </Pill>
          </div>
        </div>

        <div className="grid-rows">
          {top.map((r, i) => (
            <div key={i} className={`grid-row ${i === 0 ? "is-best" : ""}`}>
              <span className="grid-row-rank mono">{i + 1}</span>
              <div className="grid-row-meta">
                <span className="grid-row-rule mono">
                  {compactRule(r.rebalanceRule)}
                </span>
                <span className="grid-row-range">
                  ±{(r.rangeWidthPct / 2).toFixed(2)}%
                </span>
              </div>
              <div className="grid-row-bars">
                <span className="grid-row-bar-label">Sharpe</span>
                <MicroBar
                  value={r.sharpe}
                  min={sharpeBounds.min}
                  max={sharpeBounds.max}
                  tone={r.sharpe > 0 ? "up" : "down"}
                  showMarker
                />
              </div>
              <div className="grid-row-bars">
                <span className="grid-row-bar-label">Net</span>
                <MicroBar
                  value={r.netReturnUsd}
                  min={netBounds.min}
                  max={netBounds.max}
                  tone={r.netReturnUsd > 0 ? "up" : "down"}
                  showMarker
                />
              </div>
              <div className="grid-row-numbers mono">
                <span title={`${r.feesUsd}`}>
                  fees {formatPreciseUsd(r.feesUsd, 4)}
                </span>
                <span className={r.netReturnUsd >= 0 ? "is-up" : "is-down"}>
                  net {formatPreciseUsd(r.netReturnUsd, 4)}
                </span>
                <span>{r.timeInRangePct.toFixed(0)}% in range</span>
                <span>{r.rebalanceCount} rebals</span>
              </div>
            </div>
          ))}
        </div>
      </div>
    </Card>
  );
}
