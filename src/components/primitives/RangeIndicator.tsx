interface RangeIndicatorProps {
  current: number;
  min: number;
  max: number;
  /** Optional baseline (e.g. mean / median) rendered as a tick. */
  baseline?: number;
  tone?: "accent" | "info" | "up" | "warn";
}

const TONE: Record<NonNullable<RangeIndicatorProps["tone"]>, string> = {
  accent: "var(--accent)",
  info: "var(--secondary)",
  up: "var(--status-up)",
  warn: "var(--status-warn)",
};

/**
 * Horizontal range bar with a "current" marker and optional baseline tick.
 * Compact one-line visual for "how does X sit between min and max?".
 */
export function RangeIndicator({
  current,
  min,
  max,
  baseline,
  tone = "accent",
}: RangeIndicatorProps) {
  const span = max - min || 1;
  const currentPct = ((current - min) / span) * 100;
  const baselinePct = baseline !== undefined ? ((baseline - min) / span) * 100 : null;
  const fill = TONE[tone];

  return (
    <div style={{ position: "relative", height: 14, width: "100%" }}>
      {/* Track */}
      <div
        style={{
          position: "absolute",
          top: 6,
          left: 0,
          right: 0,
          height: 2,
          background: "rgba(255, 255, 255, 0.06)",
          borderRadius: 1,
        }}
      />
      {/* Min/max anchors */}
      <div
        style={{
          position: "absolute",
          top: 4,
          left: 0,
          width: 2,
          height: 6,
          background: "rgba(255, 255, 255, 0.18)",
          borderRadius: 1,
        }}
      />
      <div
        style={{
          position: "absolute",
          top: 4,
          right: 0,
          width: 2,
          height: 6,
          background: "rgba(255, 255, 255, 0.18)",
          borderRadius: 1,
        }}
      />
      {/* Baseline tick */}
      {baselinePct !== null ? (
        <div
          style={{
            position: "absolute",
            top: 3,
            left: `calc(${baselinePct}% - 0.5px)`,
            width: 1,
            height: 8,
            background: "rgba(255, 255, 255, 0.32)",
          }}
        />
      ) : null}
      {/* Current marker */}
      <div
        style={{
          position: "absolute",
          top: 1,
          left: `calc(${Math.max(0, Math.min(100, currentPct))}% - 4px)`,
          width: 8,
          height: 12,
          background: fill,
          borderRadius: 2,
          boxShadow: `0 0 0 1px rgba(0,0,0,0.5), 0 0 8px ${fill}`,
          transition: "left 200ms var(--ease-out)",
        }}
      />
    </div>
  );
}
