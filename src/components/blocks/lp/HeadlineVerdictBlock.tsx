import { Card } from "../../primitives/Card";
import { Pill } from "../../primitives/Pill";
import { RatioBar } from "../../primitives/RatioBar";
import { StatusGlyph } from "../../primitives/StatusGlyph";
import type { HeadlineRunSummary } from "../../../features/lp-backtest/types";

interface HeadlineVerdictBlockProps {
  summary: HeadlineRunSummary | null;
  busy: boolean;
  onRemove?: () => void;
}

function fmtPct(value: number | null | undefined, decimals = 2): string {
  if (value === null || value === undefined) return "—";
  const pct = value * 100;
  const sign = pct >= 0 ? "+" : "";
  return `${sign}${pct.toFixed(decimals)}pp`;
}

export function HeadlineVerdictBlock({
  summary,
  busy,
  onRemove,
}: HeadlineVerdictBlockProps) {
  if (!summary) {
    return (
      <Card title="Verdict" subtitle={busy ? "synthesising…" : "M2.8 capital allocation"} onRemove={onRemove}>
        <div className="lp-card-empty">
          <span>Auto-running the pipeline — verdict will land here.</span>
        </div>
      </Card>
    );
  }

  const won = summary.months_lp_beat_lending;
  const total = summary.months_total;
  const winRatePct = total > 0 ? (won / total) * 100 : 0;

  // Tone from win-rate: > 60% LP wins → up, 40-60 neutral, < 40 down.
  const tone: "up" | "neutral" | "down" =
    winRatePct > 60 ? "up" : winRatePct >= 40 ? "neutral" : "down";

  // Severity glyph: 4-step scale mapped from win rate.
  const severity = Math.max(1, Math.min(4, Math.round((winRatePct / 100) * 4)));

  return (
    <Card
      title="Verdict"
      subtitle={`${summary.regime_method.replace("_", " ")} · ${total}-month lookback`}
      onRemove={onRemove}
      headerExtra={
        <StatusGlyph level={severity} tone={tone === "up" ? "up" : tone === "down" ? "down" : "neutral"} total={4} />
      }
    >
      <div className="verdict-stack">
        <div className="verdict-rate-row">
          <Pill tone={tone === "up" ? "up" : tone === "down" ? "down" : "neutral"} showDot>
            <span className="verdict-rate-pill">
              <span className="verdict-rate-num">{won}</span>
              <span className="verdict-rate-sep">of</span>
              <span className="verdict-rate-num">{total}</span>
              <span className="verdict-rate-sep">months won</span>
            </span>
          </Pill>
          <span className="verdict-rate-pct mono">{winRatePct.toFixed(0)}%</span>
        </div>

        <div className="verdict-ratio">
          <RatioBar
            segments={[
              { value: won, tone: "up", label: "LP beat lending" },
              { value: Math.max(0, total - won), tone: "down", label: "Lending beat LP" },
            ]}
            height={4}
          />
        </div>

        <div className="verdict-regime-grid">
          <RegimeCell
            label="Low vol"
            value={summary.median_low_vol_spread}
          />
          <RegimeCell
            label="Mid vol"
            value={summary.median_med_vol_spread}
          />
          <RegimeCell
            label="High vol"
            value={summary.median_high_vol_spread}
            highlight
          />
        </div>

        <p className="verdict-prose">{summary.verdict_text}</p>
      </div>
    </Card>
  );
}

interface RegimeCellProps {
  label: string;
  value: number | null | undefined;
  highlight?: boolean;
}

function RegimeCell({ label, value, highlight = false }: RegimeCellProps) {
  const tone =
    value === null || value === undefined
      ? "neutral"
      : value > 0
        ? "up"
        : value < 0
          ? "down"
          : "neutral";
  return (
    <div className={`regime-cell ${highlight ? "is-highlight" : ""}`}>
      <span className="regime-cell-label">{label}</span>
      <span className={`regime-cell-value mono is-${tone}`}>
        {fmtPct(value)}
      </span>
    </div>
  );
}
