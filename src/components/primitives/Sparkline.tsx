interface SparklineProps {
  values: number[];
  /** Tone class — controls stroke and fill colour. */
  tone?: "accent" | "up" | "down" | "warn" | "info";
  /** Optional fixed height (px). Defaults to filling the container. */
  height?: number;
  /** Render an area fill below the line. */
  filled?: boolean;
}

const TONE_TO_COLOR: Record<NonNullable<SparklineProps["tone"]>, string> = {
  accent: "var(--accent)",
  up: "var(--status-up)",
  down: "var(--status-down)",
  warn: "var(--status-warn)",
  info: "var(--secondary)",
};

const VIEW_WIDTH = 200;
const VIEW_HEIGHT = 60;
const PADDING = 3;

export function Sparkline({
  values,
  tone = "accent",
  height,
  filled = true,
}: SparklineProps) {
  if (values.length < 2) {
    return <div className="sparkline" style={height ? { height } : undefined} />;
  }

  const min = Math.min(...values);
  const max = Math.max(...values);
  const span = max - min || 1;
  const stepX = (VIEW_WIDTH - PADDING * 2) / (values.length - 1);

  const points = values.map((value, index) => {
    const x = PADDING + index * stepX;
    const normalised = (value - min) / span;
    const y = VIEW_HEIGHT - PADDING - normalised * (VIEW_HEIGHT - PADDING * 2);
    return { x, y };
  });

  const linePath = points
    .map((point, index) =>
      `${index === 0 ? "M" : "L"} ${point.x.toFixed(2)} ${point.y.toFixed(2)}`,
    )
    .join(" ");

  const fillPath = `${linePath} L ${points[points.length - 1].x.toFixed(2)} ${VIEW_HEIGHT - PADDING} L ${points[0].x.toFixed(2)} ${VIEW_HEIGHT - PADDING} Z`;

  const color = TONE_TO_COLOR[tone];

  return (
    <svg
      className="sparkline"
      viewBox={`0 0 ${VIEW_WIDTH} ${VIEW_HEIGHT}`}
      preserveAspectRatio="none"
      style={height ? { height } : undefined}
    >
      {filled ? (
        <path d={fillPath} fill={color} className="sparkline-fill" />
      ) : null}
      <path d={linePath} stroke={color} className="sparkline-line" />
    </svg>
  );
}
