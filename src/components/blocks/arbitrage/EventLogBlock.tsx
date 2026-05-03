import { Card } from "../../primitives/Card";
import {
  deriveInsightsView,
  type InsightSeverity,
} from "../../../features/arbitrage/insights";
import type { BlockRenderProps } from "./BlockRegistry";

const SEVERITY_DOT: Record<InsightSeverity, string> = {
  info: "var(--text-muted)",
  watch: "var(--secondary)",
  notable: "var(--status-warn)",
  actionable: "var(--status-up)",
};

export function EventLogBlock({ market, pnlMode, onRemove }: BlockRenderProps) {
  const { history } = market;

  if (history.length === 0) {
    return (
      <Card title="Events" subtitle="session" onRemove={onRemove}>
        <div className="event-empty">Events appear after tick #2.</div>
      </Card>
    );
  }

  const { events } = deriveInsightsView(history, pnlMode);

  return (
    <Card title="Events" subtitle={`${events.length}`} onRemove={onRemove}>
      {events.length === 0 ? (
        <div className="event-empty">Quiet — no transitions yet.</div>
      ) : (
        <div className="event-timeline">
          {events.map((event, idx) => (
            <div className="event-tl-row" key={event.id}>
              <div className="event-tl-rail">
                <span
                  className="event-tl-dot"
                  style={{ background: SEVERITY_DOT[event.severity] }}
                />
                {idx < events.length - 1 ? (
                  <span className="event-tl-line" />
                ) : null}
              </div>
              <div className="event-tl-body">
                <span className="event-tl-summary">{event.summary}</span>
                <span className="event-tl-time mono">{event.timestampLabel}</span>
              </div>
            </div>
          ))}
        </div>
      )}
    </Card>
  );
}
