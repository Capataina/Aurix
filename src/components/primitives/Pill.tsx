import type { ReactNode } from "react";

type PillTone = "neutral" | "up" | "down" | "warn" | "accent" | "info";

interface PillProps {
  tone?: PillTone;
  /** When true, render a small leading dot in the same colour. */
  showDot?: boolean;
  /** When true, the leading dot pulses (useful for live indicators). */
  pulse?: boolean;
  children: ReactNode;
}

const TONE_CLASS: Record<PillTone, string> = {
  neutral: "",
  up: "is-up",
  down: "is-down",
  warn: "is-warn",
  accent: "is-accent",
  info: "is-info",
};

export function Pill({ tone = "neutral", showDot = false, pulse = false, children }: PillProps) {
  return (
    <span className={`pill ${TONE_CLASS[tone]}`.trim()}>
      {showDot ? (
        <span className={`pill-dot ${pulse ? "is-pulse" : ""}`.trim()} />
      ) : null}
      {children}
    </span>
  );
}
