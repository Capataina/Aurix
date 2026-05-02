interface HistogramProps {
  values: number[];
  /** Number of bins; default 12. */
  bins?: number;
  /** Highlight the bin containing this value with the accent tone. */
  marker?: number;
  tone?: "accent" | "info" | "up" | "warn";
  height?: number;
}

const TONE_BAR: Record<NonNullable<HistogramProps["tone"]>, string> = {
  accent: "var(--accent)",
  info: "var(--secondary)",
  up: "var(--status-up)",
  warn: "var(--status-warn)",
};

const VIEW_HEIGHT = 100;

/**
 * Compact distribution histogram. Renders the value distribution as vertical
 * bars; if `marker` is supplied, the bin containing it is highlighted in the
 * accent colour and the other bins are dimmed.
 *
 * Used to answer "where in the recent distribution is the current value?".
 */
export function Histogram({
  values,
  bins = 12,
  marker,
  tone = "info",
  height = 60,
}: HistogramProps) {
  if (values.length === 0) {
    return <div style={{ height }} />;
  }

  const min = Math.min(...values);
  const max = Math.max(...values);
  const span = max - min || 1;
  const binSize = span / bins;

  const counts = new Array(bins).fill(0) as number[];
  for (const value of values) {
    const idx = Math.min(bins - 1, Math.floor((value - min) / binSize));
    counts[idx] += 1;
  }
  const peak = Math.max(...counts);

  const markerBin = marker !== undefined
    ? Math.min(bins - 1, Math.max(0, Math.floor((marker - min) / binSize)))
    : -1;

  const accentColor = TONE_BAR[tone];
  const dimColor = "rgba(255, 255, 255, 0.15)";

  const barWidth = 100 / bins;
  const gap = 0.5;

  return (
    <svg
      viewBox={`0 0 100 ${VIEW_HEIGHT}`}
      preserveAspectRatio="none"
      style={{ display: "block", width: "100%", height }}
    >
      {counts.map((count, idx) => {
        const h = peak === 0 ? 0 : (count / peak) * (VIEW_HEIGHT - 6);
        const isMarker = idx === markerBin;
        const x = idx * barWidth + gap / 2;
        const y = VIEW_HEIGHT - h - 1;

        return (
          <rect
            key={idx}
            x={x}
            y={y}
            width={barWidth - gap}
            height={Math.max(h, 1.2)}
            fill={isMarker ? accentColor : dimColor}
            rx={0.5}
          />
        );
      })}
    </svg>
  );
}
