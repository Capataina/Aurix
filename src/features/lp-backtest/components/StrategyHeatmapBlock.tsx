import { useMemo, useState } from "react";

import { Card } from "../../../components/primitives/Card";
import { formatUsd } from "../../../lib/format";
import type { StrategyResultRow } from "../types";

interface StrategyHeatmapBlockProps {
  rows: StrategyResultRow[];
  onRunGrid: () => void;
  busy: boolean;
  onRemove?: () => void;
}

type SortKey =
  | "sharpe"
  | "sortino"
  | "deflatedSharpe"
  | "netReturnUsd"
  | "feesUsd"
  | "ilUsd"
  | "mgmtGasUsd"
  | "maxDrawdownPct";

const SORT_OPTIONS: Array<{ key: SortKey; label: string }> = [
  { key: "sharpe", label: "Sharpe" },
  { key: "sortino", label: "Sortino" },
  { key: "deflatedSharpe", label: "Deflated Sharpe" },
  { key: "netReturnUsd", label: "Net return" },
  { key: "feesUsd", label: "Fees" },
  { key: "ilUsd", label: "IL" },
  { key: "mgmtGasUsd", label: "Mgmt gas" },
  { key: "maxDrawdownPct", label: "Max DD" },
];

function compareDesc(a: StrategyResultRow, b: StrategyResultRow, key: SortKey): number {
  return (b[key] as number) - (a[key] as number);
}

export function StrategyHeatmapBlock({
  rows,
  onRunGrid,
  busy,
  onRemove,
}: StrategyHeatmapBlockProps) {
  const [sortKey, setSortKey] = useState<SortKey>("deflatedSharpe");

  const sorted = useMemo(() => {
    return [...rows].sort((a, b) => compareDesc(a, b, sortKey));
  }, [rows, sortKey]);

  if (!rows.length) {
    return (
      <Card
        title="Strategy grid"
        subtitle="range × rebalance × deposit × period"
        onRemove={onRemove}
      >
        <div className="lp-empty">
          No grid results yet.
          <button
            type="button"
            className="lp-button is-primary"
            onClick={onRunGrid}
            disabled={busy}
            style={{ marginLeft: 12 }}
          >
            Run grid
          </button>
        </div>
      </Card>
    );
  }

  return (
    <Card
      title="Strategy grid"
      subtitle={`${rows.length} cells · sorted by ${SORT_OPTIONS.find((o) => o.key === sortKey)?.label}`}
      onRemove={onRemove}
      headerExtra={
        <select
          className="lp-select"
          value={sortKey}
          onChange={(e) => setSortKey(e.target.value as SortKey)}
        >
          {SORT_OPTIONS.map((o) => (
            <option key={o.key} value={o.key}>
              Sort: {o.label}
            </option>
          ))}
        </select>
      }
    >
      <div className="lp-strategy-table" role="region" aria-label="Strategy grid">
        <table>
          <thead>
            <tr>
              <th>Range %</th>
              <th>Rule</th>
              <th>Deposit</th>
              <th>Period (d)</th>
              <th>Sharpe</th>
              <th>Sortino</th>
              <th>Deflated</th>
              <th>Net</th>
              <th>Fees</th>
              <th>IL</th>
              <th>Gas</th>
              <th>Max DD</th>
              <th>Time in range</th>
              <th>Rebals</th>
            </tr>
          </thead>
          <tbody>
            {sorted.slice(0, 50).map((r, i) => (
              <tr key={i}>
                <td>{r.rangeWidthPct.toFixed(1)}</td>
                <td title={r.rebalanceRule}>{compactRule(r.rebalanceRule)}</td>
                <td>{formatUsd(r.depositUsd)}</td>
                <td>{r.periodDays}</td>
                <td>{r.sharpe.toFixed(2)}</td>
                <td>{r.sortino.toFixed(2)}</td>
                <td>{r.deflatedSharpe.toFixed(2)}</td>
                <td>{formatUsd(r.netReturnUsd)}</td>
                <td>{formatUsd(r.feesUsd)}</td>
                <td>{formatUsd(r.ilUsd)}</td>
                <td>{formatUsd(r.mgmtGasUsd)}</td>
                <td>{r.maxDrawdownPct.toFixed(1)}%</td>
                <td>{r.timeInRangePct.toFixed(0)}%</td>
                <td>{r.rebalanceCount}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Card>
  );
}

function compactRule(serialised: string): string {
  try {
    const parsed = JSON.parse(serialised) as { kind: string; [k: string]: unknown };
    if (parsed.kind === "static") return "static";
    if (parsed.kind === "schedule") return `sched/${parsed.every_n_blocks}`;
    if (parsed.kind === "price_exit_threshold")
      return `exit/${(parsed.central_pct as number).toFixed(2)}`;
    if (parsed.kind === "out_of_range_duration")
      return `oor/${parsed.min_oor_blocks}`;
    return parsed.kind;
  } catch {
    return serialised;
  }
}
