import { useEffect, useRef, useState } from "react";

import { Card } from "../../primitives/Card";
import {
  formatSignedPercent,
  formatUsd,
} from "../../../lib/format";
import { findBestRoute } from "../../../lib/arbitrage";
import { SLOT_COUNT } from "../../../lib/config";
import { median } from "../../../lib/stats";
import { venueSwatchByIndex, type SwatchClass } from "../../../lib/venues";
import type { MarketOverview } from "../../../features/arbitrage/types";
import type { BlockRenderProps } from "./BlockRegistry";

type ChartMode = "raw" | "deviation" | "spread" | "net";

const MODE_LABELS: Record<ChartMode, string> = {
  raw: "Raw",
  deviation: "Deviation",
  spread: "Spread",
  net: "Net P/L",
};

interface RenderSeries {
  label: string;
  swatch: SwatchClass;
  points: Array<{ x: number; y: number }>;
  dashed?: boolean;
}

function buildPath(points: Array<{ x: number; y: number }>): string {
  if (points.length === 0) return "";
  return points
    .map((point, index) =>
      `${index === 0 ? "M" : "L"} ${point.x.toFixed(2)} ${point.y.toFixed(2)}`,
    )
    .join(" ");
}

function createDomain(values: number[]) {
  const minValue = Math.min(...values);
  const maxValue = Math.max(...values);

  if (minValue === maxValue) {
    const padding = Math.max(Math.abs(minValue) * 0.002, 0.0005);
    return { min: minValue - padding, max: maxValue + padding };
  }

  const spread = maxValue - minValue;
  const padding = spread * 0.12;
  return { min: minValue - padding, max: maxValue + padding };
}

function scaleY(value: number, domain: { min: number; max: number }, top: number, bottom: number) {
  const normalised = (value - domain.min) / (domain.max - domain.min);
  return bottom - normalised * (bottom - top);
}

function xForIndex(index: number, occupiedOffset: number, left: number, width: number) {
  return left + ((occupiedOffset + index) / (SLOT_COUNT - 1)) * width;
}

interface DerivedSample {
  overview: MarketOverview;
  medianPrice: number;
  spread: number;
  /** Best-route net under the active mode (post gas, optionally post fees). */
  netUsd: number;
}

function deriveSamples(
  history: MarketOverview[],
  pnlMode: BlockRenderProps["pnlMode"],
): DerivedSample[] {
  return history.map((overview) => {
    const prices = overview.venues.map((venue) => venue.priceUsd);
    const medianPrice = median(prices);
    const spread = Math.max(...prices) - Math.min(...prices);
    const route = findBestRoute(overview.venues, overview.gasPriceGwei, pnlMode);
    return {
      overview,
      medianPrice,
      spread,
      netUsd: route?.netUsd ?? spread,
    };
  });
}

export function PriceChartBlock({ market, pnlMode, onRemove }: BlockRenderProps) {
  const [mode, setMode] = useState<ChartMode>("spread");
  const stageRef = useRef<HTMLDivElement | null>(null);
  const [size, setSize] = useState({ width: 600, height: 240 });

  useEffect(() => {
    const node = stageRef.current;
    if (!node) return;

    const updateSize = () => {
      setSize({
        width: Math.max(node.clientWidth, 200),
        height: Math.max(node.clientHeight, 120),
      });
    };

    updateSize();
    const observer = new ResizeObserver(() => updateSize());
    observer.observe(node);

    return () => {
      observer.disconnect();
    };
  }, []);

  const { history, overview } = market;

  const headerExtra = (
    <span className="metric-delta" style={{ fontSize: 11, color: "var(--text-muted)" }}>
      {history.length}/{SLOT_COUNT}
    </span>
  );

  if (history.length === 0) {
    return (
      <Card title="Price chart" subtitle="Awaiting data" headerExtra={headerExtra} onRemove={onRemove}>
        <div className="card-empty">Chart begins after the first tick.</div>
      </Card>
    );
  }

  const venueOrder = new Map(
    history[0].venues.map((venue, index) => [venue.dexName, index] as const),
  );
  const padding = { top: 12, right: 8, bottom: 22, left: 44 };
  const chartTop = padding.top;
  const chartBottom = size.height - padding.bottom;
  const plotWidth = size.width - padding.left - padding.right;
  const occupiedOffset = SLOT_COUNT - Math.max(history.length, 2);
  const samples = deriveSamples(history, pnlMode);

  let series: RenderSeries[] = [];
  let yTicks: string[] = [];
  let metricLabel = "";
  let metricValue = "";

  const venueNames = history[0].venues.map((venue) => venue.dexName);

  if (mode === "raw") {
    const allValues = history.flatMap((entry) =>
      entry.venues.map((venue) => venue.priceUsd),
    );
    const domain = createDomain(allValues);

    series = venueNames.map((venueName) => {
      const points = history.map((entry, index) => {
        const venue = entry.venues.find((candidate) => candidate.dexName === venueName);
        const y = scaleY(venue?.priceUsd ?? domain.min, domain, chartTop, chartBottom);
        return { x: xForIndex(index, occupiedOffset, padding.left, plotWidth), y };
      });

      return {
        label: venueName,
        swatch: venueSwatchByIndex(venueOrder.get(venueName) ?? 0),
        points,
      };
    });

    yTicks = [
      formatUsd(domain.max),
      formatUsd((domain.max + domain.min) / 2),
      formatUsd(domain.min),
    ];
    metricLabel = "Median";
    metricValue = formatUsd(samples[samples.length - 1].medianPrice);
  } else if (mode === "deviation") {
    const deviationByVenue = venueNames.map((venueName) => ({
      venueName,
      values: history.map((entry) => {
        const prices = entry.venues.map((venue) => venue.priceUsd);
        const midpoint = median(prices);
        const venue = entry.venues.find((candidate) => candidate.dexName === venueName);
        const value = venue?.priceUsd ?? midpoint;
        return ((value - midpoint) / midpoint) * 100;
      }),
    }));

    const domain = createDomain(deviationByVenue.flatMap((entry) => entry.values));
    series = deviationByVenue.map((entry) => ({
      label: entry.venueName,
      swatch: venueSwatchByIndex(venueOrder.get(entry.venueName) ?? 0),
      dashed: true,
      points: entry.values.map((value, index) => ({
        x: xForIndex(index, occupiedOffset, padding.left, plotWidth),
        y: scaleY(value, domain, chartTop, chartBottom),
      })),
    }));

    yTicks = [
      formatSignedPercent(domain.max, 2),
      formatSignedPercent((domain.max + domain.min) / 2, 2),
      formatSignedPercent(domain.min, 2),
    ];

    const last = deviationByVenue
      .map((entry) => ({
        venueName: entry.venueName,
        value: entry.values[entry.values.length - 1],
      }))
      .sort((left, right) => Math.abs(right.value) - Math.abs(left.value))[0];
    metricLabel = "Strongest";
    metricValue = `${last.venueName} ${formatSignedPercent(last.value)}`;
  } else if (mode === "spread") {
    const spreadValues = samples.map((entry) => entry.spread);
    const domain = createDomain(spreadValues);

    series = [
      {
        label: "Spread",
        swatch: "venue-2",
        points: spreadValues.map((value, index) => ({
          x: xForIndex(index, occupiedOffset, padding.left, plotWidth),
          y: scaleY(value, domain, chartTop, chartBottom),
        })),
      },
    ];

    yTicks = [
      formatUsd(domain.max),
      formatUsd((domain.max + domain.min) / 2),
      formatUsd(domain.min),
    ];
    metricLabel = "Now";
    metricValue = formatUsd(spreadValues[spreadValues.length - 1]);
  } else {
    const netValues = samples.map((entry) => entry.netUsd);
    const domain = createDomain(netValues);

    series = [
      {
        label: pnlMode === "gas-and-fees" ? "Net (gas + fees)" : "Net (gas)",
        swatch: "venue-4",
        points: netValues.map((value, index) => ({
          x: xForIndex(index, occupiedOffset, padding.left, plotWidth),
          y: scaleY(value, domain, chartTop, chartBottom),
        })),
      },
    ];

    yTicks = [
      formatUsd(domain.max),
      formatUsd((domain.max + domain.min) / 2),
      formatUsd(domain.min),
    ];
    metricLabel = "Net";
    metricValue = formatUsd(netValues[netValues.length - 1]);
  }

  const eventPoints =
    mode === "net"
      ? samples
          .map((entry, index) => ({ index, value: entry.netUsd }))
          .filter((entry) => entry.value > 0)
          .map((entry) => ({
            x: xForIndex(entry.index, occupiedOffset, padding.left, plotWidth),
            y: chartTop + 4,
          }))
      : [];

  const gridYs = [chartTop, (chartTop + chartBottom) / 2, chartBottom];

  return (
    <Card
      title="Price chart"
      subtitle={overview?.pairLabel ?? ""}
      headerExtra={headerExtra}
      onRemove={onRemove}
    >
      <div className="chart-card">
        <div className="chart-controls">
          {(["raw", "deviation", "spread", "net"] as ChartMode[]).map((current) => (
            <button
              key={current}
              type="button"
              className={`chart-mode ${mode === current ? "is-active" : ""}`}
              onClick={() => setMode(current)}
            >
              {MODE_LABELS[current]}
            </button>
          ))}
          <span style={{ marginLeft: "auto", display: "inline-flex", gap: 8, alignItems: "center", fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--text-muted)" }}>
            <span>{metricLabel}</span>
            <span style={{ color: "var(--text-secondary)" }}>{metricValue}</span>
          </span>
        </div>

        <div className="chart-stage" ref={stageRef}>
          <svg viewBox={`0 0 ${size.width} ${size.height}`} preserveAspectRatio="none">
            {gridYs.map((y, index) => (
              <g key={y}>
                <line
                  className="chart-grid-line"
                  x1={padding.left}
                  y1={y}
                  x2={size.width - padding.right}
                  y2={y}
                />
                <text className="chart-grid-label" x={4} y={y + 3}>
                  {yTicks[index]}
                </text>
              </g>
            ))}

            {series.map((entry) => {
              const lastPoint = entry.points[entry.points.length - 1];
              return (
                <g key={entry.label}>
                  <path
                    className={`chart-line chart-${entry.swatch} ${entry.dashed ? "chart-line-dashed" : ""}`}
                    d={buildPath(entry.points)}
                  />
                  <circle
                    className={`chart-point-ring chart-${entry.swatch}`}
                    cx={lastPoint.x}
                    cy={lastPoint.y}
                    r={5}
                  />
                  <circle
                    className={`chart-point chart-${entry.swatch}`}
                    cx={lastPoint.x}
                    cy={lastPoint.y}
                    r={2.5}
                  />
                </g>
              );
            })}

            {eventPoints.map((point, index) => (
              <circle
                key={index}
                className="chart-event-point"
                cx={point.x}
                cy={point.y}
                r={2.5}
              />
            ))}
          </svg>
        </div>

        <div className="chart-footer">
          <div className="chart-legend">
            {series.map((entry) => (
              <span className="chart-legend-item" key={entry.label}>
                <span className={`chart-legend-swatch ${entry.swatch}`} />
                {entry.label}
              </span>
            ))}
            {eventPoints.length > 0 ? (
              <span className="chart-legend-item">
                <span className="chart-legend-swatch is-event" />
                Profitable
              </span>
            ) : null}
          </div>
          <span>newest right →</span>
        </div>
      </div>
    </Card>
  );
}
