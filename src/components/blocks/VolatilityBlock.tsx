import { Card } from "../primitives/Card";
import { Dial } from "../primitives/Dial";
import { Sparkline } from "../primitives/Sparkline";
import { mean, standardDeviation } from "../../lib/stats";
import type { BlockRenderProps } from "./BlockRegistry";

const ROLLING_WINDOW = 20;

/**
 * Rolling realised volatility (standard deviation of percent returns over the
 * last N ticks, expressed as percent). Dial gauge shows where the current
 * regime sits on a coarse "calm → frantic" scale; sparkline shows how it's
 * evolving.
 */
export function VolatilityBlock({ market, onRemove }: BlockRenderProps) {
  const { history } = market;

  if (history.length < 3) {
    return (
      <Card title="Volatility" subtitle="rolling" onRemove={onRemove}>
        <div className="card-empty">Awaiting samples.</div>
      </Card>
    );
  }

  const heroSeries = history.map((entry) => entry.venues[0]?.priceUsd ?? 0);
  const returns = heroSeries.slice(1).map((price, idx) => {
    const previous = heroSeries[idx];
    return previous === 0 ? 0 : ((price - previous) / previous) * 100;
  });

  const recentReturns = returns.slice(-ROLLING_WINDOW);
  const stdNow = standardDeviation(recentReturns);

  // Build a series of rolling stds for the sparkline.
  const stdSeries: number[] = [];
  for (let idx = ROLLING_WINDOW; idx <= returns.length; idx += 1) {
    const window = returns.slice(idx - ROLLING_WINDOW, idx);
    stdSeries.push(standardDeviation(window));
  }
  if (stdSeries.length === 0) {
    stdSeries.push(stdNow);
  }

  const baseline = mean(stdSeries);
  // Map current vs baseline onto a [0,1] dial: 0 if quiet, ~0.5 at baseline,
  // saturates at baseline*3.
  const dialValue = Math.max(0, Math.min(1, stdNow / Math.max(baseline * 3, 0.0001)));

  const tone =
    stdNow > baseline * 1.5 ? "warn" : stdNow > baseline * 0.5 ? "info" : "up";

  return (
    <Card title="Volatility" subtitle="rolling" onRemove={onRemove}>
      <div className="volatility-content">
        <Dial
          value={dialValue}
          label={`${stdNow.toFixed(3)}%`}
          sublabel="σ"
          tone={tone}
          size={72}
        />
        <div className="volatility-side">
          <span className="volatility-baseline mono">
            μ {baseline.toFixed(3)}%
          </span>
          <div className="volatility-spark">
            <Sparkline values={stdSeries} tone={tone} filled />
          </div>
        </div>
      </div>
    </Card>
  );
}
