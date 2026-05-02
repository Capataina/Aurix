interface MicroBarProps {
  /** Current value, scaled into `[min, max]`. */
  value: number;
  min: number;
  max: number;
  tone?: "accent" | "up" | "down" | "warn" | "info" | "neutral";
  /** Pixel height of the bar; defaults to 4. */
  height?: number;
  /** When true, render a 1-px notch at `value`. */
  showMarker?: boolean;
}

const TONE_FILL: Record<NonNullable<MicroBarProps["tone"]>, string> = {
  accent: "var(--accent)",
  up: "var(--status-up)",
  down: "var(--status-down)",
  warn: "var(--status-warn)",
  info: "var(--secondary)",
  neutral: "var(--text-muted)",
};

/**
 * Single-value horizontal bar. The filled portion represents `value`'s
 * position in the range `[min, max]` (clamped). Renders as a track with a
 * filled region and an optional vertical marker.
 */
export function MicroBar({
  value,
  min,
  max,
  tone = "accent",
  height = 4,
  showMarker = false,
}: MicroBarProps) {
  const span = max - min;
  const ratio = span <= 0 ? 0 : (value - min) / span;
  const clamped = Math.max(0, Math.min(1, ratio));
  const fill = TONE_FILL[tone];

  return (
    <div
      style={{
        position: "relative",
        height,
        width: "100%",
        background: "rgba(255, 255, 255, 0.05)",
        borderRadius: height / 2,
        overflow: "hidden",
      }}
    >
      <div
        style={{
          position: "absolute",
          left: 0,
          top: 0,
          bottom: 0,
          width: `${clamped * 100}%`,
          background: fill,
          borderRadius: height / 2,
          transition: "width 200ms var(--ease-out)",
        }}
      />
      {showMarker ? (
        <div
          style={{
            position: "absolute",
            left: `calc(${clamped * 100}% - 1px)`,
            top: -2,
            bottom: -2,
            width: 2,
            background: fill,
            boxShadow: `0 0 0 2px rgba(0,0,0,0.4)`,
            borderRadius: 1,
          }}
        />
      ) : null}
    </div>
  );
}
