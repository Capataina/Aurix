import { Card } from "../../primitives/Card";
import { Dial } from "../../primitives/Dial";
import { Sparkline } from "../../primitives/Sparkline";
import { HISTORY_LIMIT } from "../../../lib/config";
import { formatPercent, formatUsd } from "../../../lib/format";
import { median, range as rangeStats, standardDeviation } from "../../../lib/stats";
import type { BlockRenderProps } from "./BlockRegistry";

export function QuickStatsBlock({ market, onRemove }: BlockRenderProps) {
  const { overview, history } = market;

  const prices = overview?.venues.map((venue) => venue.priceUsd) ?? [];
  const venueMedian = prices.length > 0 ? median(prices) : null;
  const venueRange = prices.length > 0 ? rangeStats(prices) : null;
  const venueStdPct =
    prices.length > 1 && venueMedian
      ? (standardDeviation(prices) / venueMedian) * 100
      : null;

  const heroSeries = history.map((entry) => entry.venues[0]?.priceUsd ?? 0);
  const samplesPct = history.length / HISTORY_LIMIT;

  return (
    <Card title="Stats" subtitle="now" onRemove={onRemove}>
      <div className="stats-quad">
        <div className="stats-cell">
          <span className="stats-label">Median</span>
          <span className="stats-value mono">
            {venueMedian === null ? "—" : formatUsd(venueMedian)}
          </span>
          <div className="stats-spark">
            <Sparkline values={heroSeries} tone="accent" filled height={20} />
          </div>
        </div>

        <div className="stats-cell">
          <span className="stats-label">Range</span>
          <span className="stats-value mono">
            {venueRange === null ? "—" : formatUsd(venueRange.spread)}
          </span>
          <div
            style={{
              height: 4,
              borderRadius: 2,
              background: "rgba(255,255,255,0.05)",
              overflow: "hidden",
              marginTop: 4,
            }}
          >
            <div
              style={{
                width: `${Math.min(
                  100,
                  venueRange && venueMedian
                    ? (venueRange.spread / venueMedian) * 100 * 50
                    : 0,
                )}%`,
                height: "100%",
                background: "var(--secondary)",
              }}
            />
          </div>
        </div>

        <div className="stats-cell">
          <span className="stats-label">Std %</span>
          <span className="stats-value mono">
            {venueStdPct === null ? "—" : formatPercent(venueStdPct, 3)}
          </span>
          <div
            style={{
              height: 4,
              borderRadius: 2,
              background: "rgba(255,255,255,0.05)",
              overflow: "hidden",
              marginTop: 4,
            }}
          >
            <div
              style={{
                width: `${Math.min(100, (venueStdPct ?? 0) * 80)}%`,
                height: "100%",
                background: "var(--status-warn)",
              }}
            />
          </div>
        </div>

        <div className="stats-cell stats-cell-dial">
          <Dial
            value={samplesPct}
            label={`${history.length}`}
            sublabel={`/${HISTORY_LIMIT}`}
            tone="info"
            size={64}
          />
        </div>
      </div>
    </Card>
  );
}
