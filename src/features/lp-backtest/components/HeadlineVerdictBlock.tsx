import { Card } from "../../../components/primitives/Card";
import { Pill } from "../../../components/primitives/Pill";
import type { HeadlineRunSummary } from "../types";

interface HeadlineVerdictBlockProps {
  summary: HeadlineRunSummary | null;
  onRunHeadline: () => void;
  busy: boolean;
  onRemove?: () => void;
}

function fmtPct(value: number | null | undefined): string {
  if (value === null || value === undefined) return "—";
  const pct = value * 100;
  return `${pct >= 0 ? "+" : ""}${pct.toFixed(2)}pp`;
}

export function HeadlineVerdictBlock({
  summary,
  onRunHeadline,
  busy,
  onRemove,
}: HeadlineVerdictBlockProps) {
  if (!summary) {
    return (
      <Card
        title="Should you have LP'd?"
        subtitle="M2.8 capital-allocation verdict"
        onRemove={onRemove}
      >
        <div className="lp-headline-empty">
          Pull benchmark series + run a strategy grid first, then synthesise the
          headline.
          <button
            type="button"
            className="lp-button is-primary"
            onClick={onRunHeadline}
            disabled={busy}
          >
            Synthesise headline
          </button>
        </div>
      </Card>
    );
  }
  const won = summary.months_lp_beat_lending;
  const total = summary.months_total;
  const winRate = total > 0 ? (won / total) * 100 : 0;
  const tone =
    winRate > 60 ? "up" : winRate > 40 ? "neutral" : "down";
  return (
    <Card
      title="Should you have LP'd?"
      subtitle={`${summary.regime_method} · ${total} months`}
      onRemove={onRemove}
    >
      <div className="lp-headline">
        <div className="lp-headline-rate">
          <Pill tone={tone}>
            {won} of {total} months won
          </Pill>
          <span className="lp-headline-rate-label">
            {winRate.toFixed(0)}% win rate vs lending
          </span>
        </div>
        <div className="lp-headline-grid">
          <div className="lp-headline-cell">
            <span className="lp-headline-cell-label">Low vol</span>
            <span className="lp-headline-cell-value">
              {fmtPct(summary.median_low_vol_spread)}
            </span>
          </div>
          <div className="lp-headline-cell">
            <span className="lp-headline-cell-label">Med vol</span>
            <span className="lp-headline-cell-value">
              {fmtPct(summary.median_med_vol_spread)}
            </span>
          </div>
          <div className="lp-headline-cell">
            <span className="lp-headline-cell-label">High vol</span>
            <span className="lp-headline-cell-value">
              {fmtPct(summary.median_high_vol_spread)}
            </span>
          </div>
        </div>
        <p className="lp-headline-prose">{summary.verdict_text}</p>
        <div className="lp-headline-actions">
          <button
            type="button"
            className="lp-button"
            onClick={onRunHeadline}
            disabled={busy}
          >
            Re-synthesise
          </button>
        </div>
      </div>
    </Card>
  );
}
