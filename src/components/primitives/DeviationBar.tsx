interface DeviationBarProps {
  /** Signed deviation in the same units as `range` (e.g. percent or absolute). */
  value: number;
  /** Symmetric extent around zero. The bar always centres on 0. */
  range: number;
  /** Pixel height. Default 4. */
  height?: number;
  /** Override the colour rule (positive=up, negative=down by default). */
  toneOverride?: "accent" | "up" | "down" | "warn" | "info";
}

const TONE_FILL: Record<NonNullable<DeviationBarProps["toneOverride"]>, string> = {
  accent: "var(--accent)",
  up: "var(--status-up)",
  down: "var(--status-down)",
  warn: "var(--status-warn)",
  info: "var(--secondary)",
};

/**
 * Centred bar showing a signed deviation from zero. Useful for "% deviation
 * from median" or "P/L vs flat". The fill grows right for positive values,
 * left for negative; colour reflects sign unless overridden.
 */
export function DeviationBar({
  value,
  range,
  height = 4,
  toneOverride,
}: DeviationBarProps) {
  if (range <= 0) {
    range = Math.max(Math.abs(value), 0.0001);
  }

  const ratio = Math.max(-1, Math.min(1, value / range));
  const widthPercent = Math.abs(ratio) * 50; // half the bar at most
  const tone = toneOverride ?? (value >= 0 ? "up" : "down");
  const fill = TONE_FILL[tone];

  return (
    <div
      style={{
        position: "relative",
        height,
        width: "100%",
        background: "rgba(255, 255, 255, 0.04)",
        borderRadius: height / 2,
        overflow: "hidden",
      }}
    >
      {/* Centre line */}
      <div
        style={{
          position: "absolute",
          left: "calc(50% - 0.5px)",
          top: 0,
          bottom: 0,
          width: 1,
          background: "rgba(255, 255, 255, 0.15)",
        }}
      />
      {/* Filled portion */}
      <div
        style={{
          position: "absolute",
          top: 0,
          bottom: 0,
          left: ratio < 0 ? `${50 - widthPercent}%` : "50%",
          width: `${widthPercent}%`,
          background: fill,
          borderRadius: height / 2,
          transition: "left 200ms var(--ease-out), width 200ms var(--ease-out)",
        }}
      />
    </div>
  );
}
