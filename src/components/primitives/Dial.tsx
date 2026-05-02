interface DialProps {
  /** Value in `[0, 1]`. Clamped. */
  value: number;
  /** Label shown in the middle. */
  label?: string;
  /** Optional secondary line. */
  sublabel?: string;
  tone?: "accent" | "up" | "down" | "warn" | "info";
  size?: number;
}

const TONE_STROKE: Record<NonNullable<DialProps["tone"]>, string> = {
  accent: "var(--accent)",
  up: "var(--status-up)",
  down: "var(--status-down)",
  warn: "var(--status-warn)",
  info: "var(--secondary)",
};

/**
 * 270° radial gauge. Useful for "fill" metrics like sample-window completion,
 * positive-tick ratio, or normalised gas pressure.
 */
export function Dial({
  value,
  label,
  sublabel,
  tone = "accent",
  size = 80,
}: DialProps) {
  const clamped = Math.max(0, Math.min(1, value));
  const radius = 32;
  const circumference = 2 * Math.PI * radius;
  const arcLength = circumference * 0.75; // 270°
  const fillLength = arcLength * clamped;
  const stroke = TONE_STROKE[tone];

  return (
    <div
      style={{
        position: "relative",
        width: size,
        height: size,
        flexShrink: 0,
      }}
    >
      <svg viewBox="0 0 80 80" style={{ display: "block", width: size, height: size }}>
        <g transform="rotate(135 40 40)">
          <circle
            cx={40}
            cy={40}
            r={radius}
            fill="none"
            stroke="rgba(255, 255, 255, 0.06)"
            strokeWidth={4}
            strokeDasharray={`${arcLength} ${circumference}`}
            strokeLinecap="round"
          />
          <circle
            cx={40}
            cy={40}
            r={radius}
            fill="none"
            stroke={stroke}
            strokeWidth={4}
            strokeDasharray={`${fillLength} ${circumference}`}
            strokeLinecap="round"
            style={{ transition: "stroke-dasharray 280ms var(--ease-out)" }}
          />
        </g>
      </svg>
      <div
        style={{
          position: "absolute",
          inset: 0,
          display: "grid",
          placeItems: "center",
          textAlign: "center",
        }}
      >
        <div>
          {label !== undefined ? (
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: size <= 60 ? 12 : 15,
                fontWeight: 600,
                color: "var(--text-primary)",
                lineHeight: 1,
              }}
            >
              {label}
            </div>
          ) : null}
          {sublabel !== undefined ? (
            <div
              style={{
                fontSize: 9,
                letterSpacing: "0.08em",
                textTransform: "uppercase",
                color: "var(--text-muted)",
                marginTop: 3,
              }}
            >
              {sublabel}
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}
