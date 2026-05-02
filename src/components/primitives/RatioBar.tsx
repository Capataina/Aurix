interface RatioBarSegment {
  value: number;
  tone: "up" | "down" | "warn" | "accent" | "info" | "neutral";
  label?: string;
}

interface RatioBarProps {
  segments: RatioBarSegment[];
  height?: number;
}

const TONE_FILL: Record<RatioBarSegment["tone"], string> = {
  up: "var(--status-up)",
  down: "var(--status-down)",
  warn: "var(--status-warn)",
  accent: "var(--accent)",
  info: "var(--secondary)",
  neutral: "rgba(255, 255, 255, 0.18)",
};

/**
 * Stacked horizontal bar — proportions like "12% positive / 88% negative".
 * Gaps between segments are 1px for visual separation; segments below 1%
 * collapse to a single hairline so the bar always has visible structure.
 */
export function RatioBar({ segments, height = 6 }: RatioBarProps) {
  const total = segments.reduce((sum, s) => sum + s.value, 0);
  if (total <= 0) {
    return (
      <div
        style={{
          height,
          width: "100%",
          background: "rgba(255, 255, 255, 0.05)",
          borderRadius: height / 2,
        }}
      />
    );
  }

  return (
    <div
      style={{
        display: "flex",
        gap: 1,
        height,
        width: "100%",
        background: "rgba(255, 255, 255, 0.04)",
        borderRadius: height / 2,
        overflow: "hidden",
      }}
    >
      {segments.map((segment, idx) => {
        const percent = (segment.value / total) * 100;
        return (
          <div
            key={idx}
            title={segment.label}
            style={{
              flex: `${percent} 1 0`,
              background: TONE_FILL[segment.tone],
              transition: "flex 200ms var(--ease-out)",
            }}
          />
        );
      })}
    </div>
  );
}
