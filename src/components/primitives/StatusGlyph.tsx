type GlyphTone = "up" | "down" | "warn" | "neutral" | "info" | "accent";

interface StatusGlyphProps {
  /** Filled-dot count out of 5. Clamped. */
  level: number;
  tone?: GlyphTone;
  /** When true, total dot count changes — useful for 4-step or 6-step scales. */
  total?: number;
}

const TONE: Record<GlyphTone, string> = {
  up: "var(--status-up)",
  down: "var(--status-down)",
  warn: "var(--status-warn)",
  neutral: "var(--text-muted)",
  info: "var(--secondary)",
  accent: "var(--accent)",
};

export function StatusGlyph({ level, tone = "accent", total = 5 }: StatusGlyphProps) {
  const filled = Math.max(0, Math.min(total, Math.round(level)));
  const dots = Array.from({ length: total }, (_, idx) => idx < filled);

  return (
    <span
      style={{
        display: "inline-flex",
        gap: 3,
        verticalAlign: "middle",
      }}
    >
      {dots.map((isFilled, idx) => (
        <span
          key={idx}
          style={{
            width: 6,
            height: 6,
            borderRadius: "50%",
            background: isFilled ? TONE[tone] : "rgba(255, 255, 255, 0.12)",
            boxShadow: isFilled ? `0 0 5px ${TONE[tone]}55` : undefined,
            transition: "background 200ms var(--ease-out)",
          }}
        />
      ))}
    </span>
  );
}
