import type { MarketOverview } from "../types";

export type ChartMode = "raw" | "deviation" | "spread" | "gas";

interface MarketChartProps {
  history: MarketOverview[];
  activeLabel: string;
  chartMode: ChartMode;
  onSelectMode: (mode: ChartMode) => void;
  showEvents: boolean;
  onToggleEvents: () => void;
}

interface SeriesMeta {
  accentClassName: string;
  legendClassName: string;
}

interface RenderSeries {
  label: string;
  accentClassName: string;
  legendClassName: string;
  points: Array<{ x: number; y: number }>;
  dashed?: boolean;
}

const SLOT_COUNT = 100;
const GAS_UNITS_ESTIMATE = 220_000;

const SERIES_META: Record<string, SeriesMeta> = {
  "Uniswap V3 5bps": {
    accentClassName: "chart-line-sky",
    legendClassName: "legend-swatch-sky",
  },
  "Uniswap V3 30bps": {
    accentClassName: "chart-line-lilac",
    legendClassName: "legend-swatch-lilac",
  },
  "Uniswap V2": {
    accentClassName: "chart-line-peach",
    legendClassName: "legend-swatch-peach",
  },
  SushiSwap: {
    accentClassName: "chart-line-mint",
    legendClassName: "legend-swatch-mint",
  },
};

const MODE_LABELS: Record<ChartMode, string> = {
  raw: "Raw Prices",
  deviation: "Deviation",
  spread: "Spread",
  gas: "Gas-Adjusted",
};

function formatUsd(value: number): string {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    maximumFractionDigits: 2,
  }).format(value);
}

function formatSignedPercent(value: number): string {
  return `${value >= 0 ? "+" : ""}${value.toFixed(3)}%`;
}

function buildPath(points: Array<{ x: number; y: number }>): string {
  if (points.length === 0) {
    return "";
  }

  return points
    .map((point, index) =>
      `${index === 0 ? "M" : "L"} ${point.x.toFixed(2)} ${point.y.toFixed(2)}`,
    )
    .join(" ");
}

function median(values: number[]): number {
  const sortedValues = [...values].sort((left, right) => left - right);
  const midpoint = Math.floor(sortedValues.length / 2);

  if (sortedValues.length % 2 === 0) {
    return (sortedValues[midpoint - 1] + sortedValues[midpoint]) / 2;
  }

  return sortedValues[midpoint];
}

function createDomain(values: number[]) {
  const minValue = Math.min(...values);
  const maxValue = Math.max(...values);

  if (minValue === maxValue) {
    const padding = Math.max(Math.abs(minValue) * 0.002, 0.0005);
    return {
      min: minValue - padding,
      max: maxValue + padding,
    };
  }

  const spread = maxValue - minValue;
  const padding = spread * 0.125;
  return {
    min: minValue - padding,
    max: maxValue + padding,
  };
}

function scaleY(value: number, domain: { min: number; max: number }, top: number, bottom: number) {
  const normalised = (value - domain.min) / (domain.max - domain.min);
  return bottom - normalised * (bottom - top);
}

function xForIndex(index: number, occupiedOffset: number, left: number, width: number) {
  return left + ((occupiedOffset + index) / (SLOT_COUNT - 1)) * width;
}

/**
 * Renders a single comparative chart mode at a time so the surface stays readable.
 */
export function MarketChart({
  history,
  activeLabel,
  chartMode,
  onSelectMode,
  showEvents,
  onToggleEvents,
}: MarketChartProps) {
  const width = 960;
  const height = 320;
  const padding = { top: 20, right: 18, bottom: 28, left: 18 };

  if (history.length === 0) {
    return (
      <section className="chart-panel">
        <div className="chart-header">
          <div className="chart-copy">
            <span className="eyebrow">Analytics canvas</span>
            <h2 className="section-title">Waiting for market data</h2>
            <p>The chart will begin rendering once the first live sample arrives.</p>
          </div>
        </div>
      </section>
    );
  }

  const venueNames = history[0].venues.map((venue) => venue.dexName);
  const chartTop = padding.top + (height - padding.top - padding.bottom) * 0.1;
  const chartBottom = padding.top + (height - padding.top - padding.bottom) * 0.9;
  const plotWidth = width - padding.left - padding.right;
  const occupiedOffset = SLOT_COUNT - Math.max(history.length, 2);

  const perSample = history.map((overview) => {
    const prices = overview.venues.map((venue) => venue.priceUsd);
    const medianPrice = median(prices);
    const spread = Math.max(...prices) - Math.min(...prices);
    const gasCostUsd =
      (overview.gasPriceGwei * GAS_UNITS_ESTIMATE * medianPrice) / 1_000_000_000;

    return {
      overview,
      medianPrice,
      spread,
      gasAdjusted: spread - gasCostUsd,
    };
  });

  let series: RenderSeries[] = [];
  let yTicks: string[] = [];
  let metricLabel = "";
  let metricValue = "";

  if (chartMode === "raw") {
    const allValues = history.flatMap((overview) =>
      overview.venues.map((venue) => venue.priceUsd),
    );
    const domain = createDomain(allValues);

    series = venueNames.map((venueName) => {
      const points = history.map((overview, index) => {
        const venue = overview.venues.find((candidate) => candidate.dexName === venueName);
        const y = scaleY(venue?.priceUsd ?? domain.min, domain, chartTop, chartBottom);

        return {
          x: xForIndex(index, occupiedOffset, padding.left, plotWidth),
          y,
        };
      });

      return {
        label: venueName,
        accentClassName: SERIES_META[venueName].accentClassName,
        legendClassName: SERIES_META[venueName].legendClassName,
        points,
      };
    });

    yTicks = [formatUsd(domain.max), formatUsd((domain.max + domain.min) / 2), formatUsd(domain.min)];
    metricLabel = "Current median";
    metricValue = formatUsd(perSample[perSample.length - 1].medianPrice);
  }

  if (chartMode === "deviation") {
    const deviationByVenue = venueNames.map((venueName) => ({
      venueName,
      values: history.map((overview) => {
        const prices = overview.venues.map((venue) => venue.priceUsd);
        const midpoint = median(prices);
        const venue = overview.venues.find((candidate) => candidate.dexName === venueName);
        const value = venue?.priceUsd ?? midpoint;
        return ((value - midpoint) / midpoint) * 100;
      }),
    }));

    const domain = createDomain(deviationByVenue.flatMap((entry) => entry.values));

    series = deviationByVenue.map((entry) => ({
      label: `${entry.venueName} deviation`,
      accentClassName: SERIES_META[entry.venueName].accentClassName,
      legendClassName: SERIES_META[entry.venueName].legendClassName,
      dashed: true,
      points: entry.values.map((value, index) => ({
        x: xForIndex(index, occupiedOffset, padding.left, plotWidth),
        y: scaleY(value, domain, chartTop, chartBottom),
      })),
    }));

    yTicks = [
      formatSignedPercent(domain.max),
      formatSignedPercent((domain.max + domain.min) / 2),
      formatSignedPercent(domain.min),
    ];

    const lastDeviation = series
      .map((entry) => {
        const match = deviationByVenue.find(
          (candidate) => `${candidate.venueName} deviation` === entry.label,
        );
        return {
          label: entry.label.replace(" deviation", ""),
          value: match ? match.values[match.values.length - 1] : 0,
        };
      })
      .sort((left, right) => Math.abs(right.value) - Math.abs(left.value))[0];
    metricLabel = "Strongest deviation";
    metricValue = `${lastDeviation.label} ${formatSignedPercent(lastDeviation.value)}`;
  }

  if (chartMode === "spread") {
    const spreadValues = perSample.map((entry) => entry.spread);
    const domain = createDomain(spreadValues);

    series = [
      {
        label: "Venue spread",
        accentClassName: "chart-line-mint",
        legendClassName: "legend-swatch-mint",
        points: spreadValues.map((value, index) => ({
          x: xForIndex(index, occupiedOffset, padding.left, plotWidth),
          y: scaleY(value, domain, chartTop, chartBottom),
        })),
      },
    ];

    yTicks = [formatUsd(domain.max), formatUsd((domain.max + domain.min) / 2), formatUsd(domain.min)];
    metricLabel = "Current spread";
    metricValue = formatUsd(spreadValues[spreadValues.length - 1]);
  }

  if (chartMode === "gas") {
    const gasAdjustedValues = perSample.map((entry) => entry.gasAdjusted);
    const domain = createDomain(gasAdjustedValues);

    series = [
      {
        label: "Gas-adjusted estimate",
        accentClassName: "chart-line-peach",
        legendClassName: "legend-swatch-peach",
        points: gasAdjustedValues.map((value, index) => ({
          x: xForIndex(index, occupiedOffset, padding.left, plotWidth),
          y: scaleY(value, domain, chartTop, chartBottom),
        })),
      },
    ];

    yTicks = [formatUsd(domain.max), formatUsd((domain.max + domain.min) / 2), formatUsd(domain.min)];
    metricLabel = "Current net estimate";
    metricValue = formatUsd(gasAdjustedValues[gasAdjustedValues.length - 1]);
  }

  const eventPoints =
    showEvents && chartMode === "gas"
      ? perSample
          .map((entry, index) => ({ index, value: entry.gasAdjusted }))
          .filter((entry) => entry.value > 0)
          .map((entry) => ({
            x: xForIndex(entry.index, occupiedOffset, padding.left, plotWidth),
            y: chartTop + 8,
          }))
      : [];

  return (
    <section className="chart-panel">
      <div className="chart-header">
        <div className="chart-copy">
          <span className="eyebrow">Analytics canvas</span>
          <h2 className="section-title">{activeLabel}</h2>
          <p>{MODE_LABELS[chartMode]} mode with a fixed 100-sample rolling window.</p>
        </div>

        <div className="chart-metrics">
          <div className="metric-chip">
            <span className="metric-label">Mode</span>
            <span className="metric-value">{MODE_LABELS[chartMode]}</span>
          </div>
          <div className="metric-chip">
            <span className="metric-label">{metricLabel}</span>
            <span className="metric-value">{metricValue}</span>
          </div>
          <div className="metric-chip">
            <span className="metric-label">Gas</span>
            <span className="metric-value">
              {history[history.length - 1].gasPriceGwei.toFixed(2)} gwei
            </span>
          </div>
        </div>
      </div>

      <div className="chart-control-row">
        {(["raw", "deviation", "spread", "gas"] as ChartMode[]).map((mode) => (
          <button
            className={`chart-toggle ${chartMode === mode ? "chart-toggle-active" : ""}`}
            key={mode}
            onClick={() => onSelectMode(mode)}
            type="button"
          >
            {MODE_LABELS[mode]}
          </button>
        ))}
        <button
          className={`chart-toggle ${showEvents ? "chart-toggle-active" : ""}`}
          onClick={onToggleEvents}
          type="button"
        >
          Event markers
        </button>
      </div>

      <div className="chart-stage">
        <svg viewBox={`0 0 ${width} ${height}`} aria-label="Live market analytics chart">
          {[chartTop, (chartTop + chartBottom) / 2, chartBottom].map((y, index) => (
            <g key={y}>
              <line
                className="chart-grid-line"
                x1={padding.left}
                y1={y}
                x2={width - padding.right}
                y2={y}
              />
              <text className="chart-grid-label" x={padding.left} y={y - 6}>
                {yTicks[index]}
              </text>
            </g>
          ))}

          {[0, 25, 50, 75, 99].map((slot) => {
            const x = padding.left + (slot / (SLOT_COUNT - 1)) * plotWidth;
            return (
              <line
                className="chart-grid-line"
                key={slot}
                x1={x}
                y1={padding.top}
                x2={x}
                y2={height - padding.bottom}
              />
            );
          })}

          {series.map((entry) => {
            const path = buildPath(entry.points);
            const lastPoint = entry.points[entry.points.length - 1];
            return (
              <g key={entry.label}>
                <path
                  className={`chart-line ${entry.accentClassName} ${entry.dashed ? "chart-line-dashed" : ""}`}
                  d={path}
                />
                <circle
                  className={`chart-point-ring ${entry.accentClassName}`}
                  cx={lastPoint.x}
                  cy={lastPoint.y}
                  r="6"
                />
                <circle
                  className={`chart-point ${entry.accentClassName}`}
                  cx={lastPoint.x}
                  cy={lastPoint.y}
                  r="3"
                />
              </g>
            );
          })}

          {eventPoints.map((point, index) => (
            <circle
              className="chart-event-point"
              cx={point.x}
              cy={point.y}
              key={`${point.x}-${index}`}
              r="3"
            />
          ))}
        </svg>
      </div>

      <div className="chart-footer">
        <div className="chart-legend">
          {series.map((entry) => (
            <span className="legend-item" key={entry.label}>
              <span className={`legend-swatch ${entry.legendClassName}`} />
              {entry.label}
            </span>
          ))}
          {eventPoints.length > 0 ? (
            <span className="legend-item">
              <span className="legend-swatch legend-swatch-event" />
              Profitable events
            </span>
          ) : null}
        </div>

        <span>{history.length} / {SLOT_COUNT} samples | newest at right edge</span>
      </div>
    </section>
  );
}
