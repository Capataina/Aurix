import type { InsightsViewModel, InsightSeverity } from "../insights";

interface InsightsPanelProps {
  insights: InsightsViewModel;
}

const SEVERITY_LABELS: Record<InsightSeverity, string> = {
  info: "Info",
  watch: "Watch",
  notable: "Notable",
  actionable: "Actionable",
};

/**
 * Presents live summary insights and recent market events below the main chart.
 */
export function InsightsPanel({ insights }: InsightsPanelProps) {
  return (
    <section className="panel insights-panel">
      <div className="section-header insights-header">
        <div>
          <span className="eyebrow">Insights</span>
          <h2 className="section-title">Live interpretation</h2>
        </div>
        <p>
          Automated readouts translate the current venue feed into ranking,
          spread, deviation, and gas-aware signals.
        </p>
      </div>

      <div className="insights-layout">
        <article className="insight-primary-card">
          <div className="insight-card-topline">
            <span className={`status-pill insight-pill insight-pill-${insights.primary.severity}`}>
              {SEVERITY_LABELS[insights.primary.severity]}
            </span>
            {insights.primary.metric ? (
              <span className="insight-metric">{insights.primary.metric}</span>
            ) : null}
          </div>
          <h3>{insights.primary.title}</h3>
          <p>{insights.primary.body}</p>
        </article>

        <div className="insight-secondary-grid">
          {insights.secondary.map((insight) => (
            <article className="insight-secondary-card" key={insight.id}>
              <div className="insight-card-topline">
                <span className={`status-pill insight-pill insight-pill-${insight.severity}`}>
                  {SEVERITY_LABELS[insight.severity]}
                </span>
                {insight.metric ? (
                  <span className="insight-metric">{insight.metric}</span>
                ) : null}
              </div>
              <h3>{insight.title}</h3>
              <p>{insight.body}</p>
            </article>
          ))}
        </div>

        <section className="insight-event-panel">
          <div className="insight-event-header">
            <h3>Recent events</h3>
            <span className="status-pill status-neutral">Derived from session history</span>
          </div>

          {insights.events.length > 0 ? (
            <div className="insight-event-list">
              {insights.events.map((event) => (
                <article className="insight-event-item" key={event.id}>
                  <div className="insight-event-meta">
                    <span className={`status-pill insight-pill insight-pill-${event.severity}`}>
                      {SEVERITY_LABELS[event.severity]}
                    </span>
                    <span>{event.timestampLabel}</span>
                  </div>
                  <p>{event.summary}</p>
                </article>
              ))}
            </div>
          ) : (
            <p className="insight-event-empty">
              Event transitions will appear once the session has enough market
              movement to compare.
            </p>
          )}
        </section>
      </div>
    </section>
  );
}
