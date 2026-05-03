import { Card } from "../../../components/primitives/Card";
import { Pill } from "../../../components/primitives/Pill";
import type { HeadlineMonthlyRow } from "../types";

interface RegimePanelBlockProps {
  rows: HeadlineMonthlyRow[];
  onRemove?: () => void;
}

const REGIME_TONE: Record<string, "up" | "neutral" | "down"> = {
  low: "neutral",
  medium: "neutral",
  high: "up",
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
        <div className="lp-empty">No headline run yet.</div>
      </Card>
    );
  }
  const sorted = [...rows].sort((a, b) =>
    a.yearMonth < b.yearMonth ? -1 : a.yearMonth > b.yearMonth ? 1 : 0,
  );
  return (
    <Card
      title="Regime panel"
      subtitle={`${rows.length} months · LP vs Aave/Lido by vol regime`}
      onRemove={onRemove}
    >
      <div className="lp-regime-table">
        <table>
          <thead>
            <tr>
              <th>Month</th>
              <th>Regime</th>
              <th>ETH 30d vol</th>
              <th>Best LP</th>
              <th>Median LP</th>
              <th>Aave</th>
              <th>Lido</th>
              <th>HODL</th>
              <th>LP - lending</th>
            </tr>
          </thead>
          <tbody>
            {sorted.map((r) => {
              const lending = Math.max(r.aaveUsdcReturn, r.lidoStethReturn);
              const spread = r.bestLpReturn - lending;
              return (
                <tr key={r.yearMonth}>
                  <td>{r.yearMonth}</td>
                  <td>
                    <Pill tone={REGIME_TONE[r.volRegime] ?? "neutral"}>{r.volRegime}</Pill>
                  </td>
                  <td>{(r.ethVol30d * 100).toFixed(2)}%</td>
                  <td>{fmtPct(r.bestLpReturn)}</td>
                  <td>{fmtPct(r.medianLpReturn)}</td>
                  <td>{fmtPct(r.aaveUsdcReturn)}</td>
                  <td>{fmtPct(r.lidoStethReturn)}</td>
                  <td>{fmtPct(r.hodlReturn)}</td>
                  <td>
                    <Pill tone={tone(spread)}>{fmtPct(spread)}</Pill>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </Card>
  );
}
