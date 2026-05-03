import { useEffect, useMemo, useRef, useState } from "react";

import { Card } from "../../primitives/Card";
import { formatUsd } from "../../../lib/format";
import { findBestRoute } from "../../../lib/arbitrage";
import { median } from "../../../lib/stats";
import { venueSwatchByIndex, type SwatchClass } from "../../../lib/venues";
import type { MarketOverview } from "../../../features/arbitrage/types";
import type { BlockRenderProps } from "./BlockRegistry";

type ChartMode = "raw" | "spread" | "net";

const MODE_LABELS: Record<ChartMode, string> = {
  raw: "Raw",
  spread: "Spread",
  net: "Net P/L",
};

interface RenderSeries {
  label: string;
  swatch: SwatchClass;
  points: Array<{ x: number; y: number }>;
  /** Whether to render a filled gradient area beneath the line. Reserved
   *  for the "primary" series in modes with a single line. */
  fill: boolean;
  dashed?: boolean;
}

/** Index of significant Y-deltas in pixel space — mark each as a step. */
function detectStepTransitions(
  points: Array<{ x: number; y: number }>,
  thresholdPx = 1.5,
): Array<{ x: number; y: number }> {
  const out: Array<{ x: number; y: number }> = [];
  for (let i = 1; i < points.length; i++) {
    const prev = points[i - 1];
    const cur = points[i];
    if (Math.abs(prev.y - cur.y) > thresholdPx) {
      out.push(cur);
    }
  }
  return out;
}

/**
 * Catmull-Rom-to-Bezier path interpolation. Produces a smooth curve that
 * passes through every point without overshooting — perfect for chart
 * data that has both flat plateaus and sharp steps. Tension 0.4 is a
 * compromise: enough smoothing to soften the eye-jarring corners without
 * blurring the actual step transitions into mush.
 */
function buildSmoothPath(
  points: Array<{ x: number; y: number }>,
  tension = 0.4,
): string {
  if (points.length === 0) return "";
  if (points.length === 1) {
    return `M ${points[0].x.toFixed(2)} ${points[0].y.toFixed(2)}`;
  }

  let path = `M ${points[0].x.toFixed(2)} ${points[0].y.toFixed(2)}`;
  for (let i = 0; i < points.length - 1; i++) {
    const p0 = i === 0 ? points[0] : points[i - 1];
    const p1 = points[i];
    const p2 = points[i + 1];
    const p3 = i + 2 < points.length ? points[i + 2] : p2;

    const c1x = p1.x + ((p2.x - p0.x) * tension) / 6;
    const c1y = p1.y + ((p2.y - p0.y) * tension) / 6;
    const c2x = p2.x - ((p3.x - p1.x) * tension) / 6;
    const c2y = p2.y - ((p3.y - p1.y) * tension) / 6;

    path += ` C ${c1x.toFixed(2)} ${c1y.toFixed(2)}, ${c2x.toFixed(2)} ${c2y.toFixed(2)}, ${p2.x.toFixed(2)} ${p2.y.toFixed(2)}`;
  }
  return path;
}

function buildFillPath(
  linePath: string,
  points: Array<{ x: number; y: number }>,
  baselineY: number,
): string {
  if (points.length === 0) return "";
  const lastX = points[points.length - 1].x.toFixed(2);
  const firstX = points[0].x.toFixed(2);
  return `${linePath} L ${lastX} ${baselineY.toFixed(2)} L ${firstX} ${baselineY.toFixed(2)} Z`;
}

function createDomain(values: number[]) {
  if (values.length === 0) return { min: 0, max: 1 };
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

/** Build x-coordinate for a sample, stretching all available samples
 *  across the plot width so the chart never leaves dead space on the
 *  left. As samples accumulate the older points pack tighter. */
function xForIndex(
  index: number,
  totalSamples: number,
  left: number,
  width: number,
): number {
  if (totalSamples <= 1) return left + width;
  return left + (index / (totalSamples - 1)) * width;
}

/** Time labels for the x-axis. Renders relative ticks ("-Ns") at evenly
 *  spaced positions so the user can read how much window the chart is
 *  showing. Pulled from the actual sample timestamps. */
function buildTimeTicks(
  samples: DerivedSample[],
  left: number,
  width: number,
): Array<{ x: number; label: string }> {
  if (samples.length < 2) return [];
  const newestMs = samples[samples.length - 1].overview.fetchedAtUnixMs;
  const oldestMs = samples[0].overview.fetchedAtUnixMs;
  const spanMs = Math.max(1, newestMs - oldestMs);
  const stops = [0, 0.25, 0.5, 0.75, 1.0];
  return stops.map((stop) => {
    const x = left + stop * width;
    const tMs = oldestMs + stop * spanMs;
    const ageSec = Math.round((newestMs - tMs) / 1000);
    const label = ageSec === 0 ? "now" : `−${ageSec}s`;
    return { x, label };
  });
}

export function PriceChartBlock({ market, pnlMode, onRemove }: BlockRenderProps) {
  const [mode, setMode] = useState<ChartMode>("spread");
  const stageRef = useRef<HTMLDivElement | null>(null);
  const [size, setSize] = useState({ width: 600, height: 240 });

  // Stable gradient id so multiple chart instances don't collide.
  const gradientId = useMemo(
    () => `chart-fill-${Math.random().toString(36).slice(2, 9)}`,
    [],
  );

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
      {history.length} samples
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
  const padding = { top: 14, right: 12, bottom: 26, left: 56 };
  const chartTop = padding.top;
  const chartBottom = size.height - padding.bottom;
  const plotWidth = size.width - padding.left - padding.right;
  const samples = deriveSamples(history, pnlMode);
  const totalSamples = history.length;

  let series: RenderSeries[] = [];
  let yTicks: string[] = [];
  let metricLabel = "";
  let metricValue = "";
  /** Y-coordinate of the visual baseline for fill. Only meaningful when
   *  a series sets `fill: true` — currently only Net P/L mode does, and
   *  only when the y-domain straddles zero (so the fill anchors at the
   *  zero line and visualises profitable vs loss regions). */
  let baselineY = chartBottom;
  let showZeroLine = false;
  let zeroY = 0;

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
        return { x: xForIndex(index, totalSamples, padding.left, plotWidth), y };
      });

      return {
        label: venueName,
        swatch: venueSwatchByIndex(venueOrder.get(venueName) ?? 0),
        points,
        // Multi-line view — fills muddy the read across overlapping
        // venue traces. Lines alone are honest about the data.
        fill: false,
      };
    });

    yTicks = [
      formatUsd(domain.max),
      formatUsd((domain.max + domain.min) / 2),
      formatUsd(domain.min),
    ];
    metricLabel = "Median";
    metricValue = formatUsd(samples[samples.length - 1].medianPrice);
  } else if (mode === "spread") {
    const spreadValues = samples.map((entry) => entry.spread);
    const domain = createDomain(spreadValues);

    series = [
      {
        label: "Spread",
        swatch: "venue-2",
        // Spread is a scalar with no meaningful zero baseline visible
        // (it's always >= 0, but the y-axis zooms tight to the data
        // so zero is off-screen). A fill would just be decorative.
        // Leaving the line bare keeps the read honest.
        fill: false,
        points: spreadValues.map((value, index) => ({
          x: xForIndex(index, totalSamples, padding.left, plotWidth),
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

    showZeroLine = domain.min < 0 && domain.max > 0;
    zeroY = scaleY(0, domain, chartTop, chartBottom);
    series = [
      {
        label: pnlMode === "gas-and-fees" ? "Net (gas + fees)" : "Net (gas)",
        swatch: "venue-4",
        // Fill only when zero is on-screen — then the area above zero
        // reads as "profitable region" and below as "loss region", which
        // is informationally meaningful. Otherwise drop the fill since
        // it would just be decorative.
        fill: showZeroLine,
        points: netValues.map((value, index) => ({
          x: xForIndex(index, totalSamples, padding.left, plotWidth),
          y: scaleY(value, domain, chartTop, chartBottom),
        })),
      },
    ];

    yTicks = [
      formatUsd(domain.max),
      formatUsd((domain.max + domain.min) / 2),
      formatUsd(domain.min),
    ];
    baselineY = showZeroLine ? zeroY : chartBottom;
    metricLabel = "Net";
    metricValue = formatUsd(netValues[netValues.length - 1]);
  }

  const eventPoints =
    mode === "net"
      ? samples
          .map((entry, index) => ({ index, value: entry.netUsd }))
          .filter((entry) => entry.value > 0)
          .map((entry) => ({
            x: xForIndex(entry.index, totalSamples, padding.left, plotWidth),
            y: chartTop + 4,
          }))
      : [];

  const gridYs = [chartTop, (chartTop + chartBottom) / 2, chartBottom];
  const timeTicks = buildTimeTicks(samples, padding.left, plotWidth);

  return (
    <Card
      title="Price chart"
      subtitle={overview?.pairLabel ?? ""}
      headerExtra={headerExtra}
      onRemove={onRemove}
    >
      <div className="chart-card">
        <div className="chart-controls">
          {(["raw", "spread", "net"] as ChartMode[]).map((current) => (
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
            <defs>
              {series.map((entry) => (
                <linearGradient
                  key={`grad-${entry.label}`}
                  id={`${gradientId}-${entry.swatch}`}
                  x1="0"
                  y1="0"
                  x2="0"
                  y2="1"
                >
                  <stop offset="0%" className={`chart-grad-from chart-${entry.swatch}`} />
                  <stop offset="100%" className={`chart-grad-to chart-${entry.swatch}`} />
                </linearGradient>
              ))}
            </defs>

            {/* horizontal y-grid */}
            {gridYs.map((y, index) => (
              <g key={`grid-${y}`}>
                <line
                  className="chart-grid-line"
                  x1={padding.left}
                  y1={y}
                  x2={size.width - padding.right}
                  y2={y}
                />
                <text className="chart-grid-label" x={padding.left - 8} y={y + 3} textAnchor="end">
                  {yTicks[index]}
                </text>
              </g>
            ))}

            {/* vertical x-grid */}
            {timeTicks.map((tick) => (
              <line
                key={`xgrid-${tick.x}`}
                className="chart-grid-line chart-grid-vertical"
                x1={tick.x}
                y1={chartTop}
                x2={tick.x}
                y2={chartBottom}
              />
            ))}

            {/* zero baseline (where meaningful) */}
            {showZeroLine ? (
              <g>
                <line
                  className="chart-zero-line"
                  x1={padding.left}
                  y1={zeroY}
                  x2={size.width - padding.right}
                  y2={zeroY}
                />
                <text
                  className="chart-zero-label"
                  x={padding.left - 8}
                  y={zeroY + 3}
                  textAnchor="end"
                >
                  0
                </text>
              </g>
            ) : null}

            {/* gradient fills (only for primary series) */}
            {series.map((entry) => {
              if (!entry.fill || entry.points.length === 0) return null;
              const linePath = buildSmoothPath(entry.points);
              const fillPath = buildFillPath(linePath, entry.points, baselineY);
              return (
                <path
                  key={`fill-${entry.label}`}
                  className={`chart-fill chart-${entry.swatch}`}
                  d={fillPath}
                  fill={`url(#${gradientId}-${entry.swatch})`}
                />
              );
            })}

            {/* lines */}
            {series.map((entry) => {
              if (entry.points.length === 0) return null;
              const linePath = buildSmoothPath(entry.points);
              const lastPoint = entry.points[entry.points.length - 1];
              const transitions = detectStepTransitions(entry.points);
              return (
                <g key={`line-${entry.label}`}>
                  <path
                    className={`chart-line chart-${entry.swatch} ${entry.dashed ? "chart-line-dashed" : ""}`}
                    d={linePath}
                  />
                  {transitions.map((p, i) => (
                    <circle
                      key={`step-${entry.label}-${i}`}
                      className={`chart-step-mark chart-${entry.swatch}`}
                      cx={p.x}
                      cy={p.y}
                      r={2}
                    />
                  ))}
                  <circle
                    className={`chart-point-ring chart-${entry.swatch}`}
                    cx={lastPoint.x}
                    cy={lastPoint.y}
                    r={6}
                  />
                  <circle
                    className={`chart-point chart-${entry.swatch}`}
                    cx={lastPoint.x}
                    cy={lastPoint.y}
                    r={3}
                  />
                </g>
              );
            })}

            {/* profitable-tick markers (net mode only) */}
            {eventPoints.map((point, index) => (
              <circle
                key={`evt-${index}`}
                className="chart-event-point"
                cx={point.x}
                cy={point.y}
                r={2.5}
              />
            ))}

            {/* x-axis time labels along the bottom */}
            {timeTicks.map((tick) => (
              <text
                key={`xlabel-${tick.x}`}
                className="chart-grid-label"
                x={tick.x}
                y={chartBottom + 14}
                textAnchor="middle"
              >
                {tick.label}
              </text>
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
            {showZeroLine ? (
              <span className="chart-legend-item">
                <span className="chart-legend-swatch is-zero" />
                Zero baseline
              </span>
            ) : null}
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
