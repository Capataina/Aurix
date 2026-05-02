import { Card } from "../primitives/Card";
import { StatusGlyph } from "../primitives/StatusGlyph";
import {
  deriveInsightsView,
  type InsightSeverity,
} from "../../features/arbitrage/insights";
import type { BlockRenderProps } from "./BlockRegistry";

const SEVERITY_LEVEL: Record<InsightSeverity, number> = {
  info: 1,
  watch: 2,
  notable: 3,
  actionable: 4,
};

const SEVERITY_TONE: Record<
  InsightSeverity,
  "neutral" | "info" | "warn" | "up"
> = {
  info: "neutral",
  watch: "info",
  notable: "warn",
  actionable: "up",
};

export function InsightsBlock({ market, onRemove }: BlockRenderProps) {
  const { history } = market;

  if (history.length === 0) {
    return (
      <Card title="Signals" subtitle="auto" onRemove={onRemove}>
        <div className="card-empty">Awaiting first sample.</div>
      </Card>
    );
  }

  const insights = deriveInsightsView(history);
  const signals = insights.secondary.slice(0, 4);

  return (
    <Card title="Signals" subtitle="auto" onRemove={onRemove}>
      <div className="insights-stack">
        <article
          className={`insight-item is-primary severity-${insights.primary.severity}`}
        >
          <div className="insight-row">
            <span className="insight-title">{insights.primary.title}</span>
            <StatusGlyph
              level={SEVERITY_LEVEL[insights.primary.severity]}
              tone={SEVERITY_TONE[insights.primary.severity]}
              total={4}
            />
          </div>
          {insights.primary.metric ? (
            <span className="insight-metric mono">{insights.primary.metric}</span>
          ) : null}
        </article>

        <div className="signals-grid">
          {signals.map((signal) => (
            <div
              key={signal.id}
              className={`signal-row severity-${signal.severity}`}
            >
              <span className="signal-name">{signal.title}</span>
              <span className="signal-glyph">
                <StatusGlyph
                  level={SEVERITY_LEVEL[signal.severity]}
                  tone={SEVERITY_TONE[signal.severity]}
                  total={4}
                />
              </span>
              <span className="signal-metric mono">{signal.metric ?? ""}</span>
            </div>
          ))}
        </div>
      </div>
    </Card>
  );
}
