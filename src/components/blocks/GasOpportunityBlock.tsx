import { Card } from "../primitives/Card";
import { RatioBar } from "../primitives/RatioBar";
import { Sparkline } from "../primitives/Sparkline";
import { GAS_UNITS_ESTIMATE } from "../../lib/config";
import { formatGweiSmart, formatSignedUsd, formatUsd } from "../../lib/format";
import { median } from "../../lib/stats";
import { findExtremes, shortenVenueName, venueSwatchByIndex } from "../../lib/venues";
import type { BlockRenderProps } from "./BlockRegistry";

export function GasOpportunityBlock({ market, onRemove }: BlockRenderProps) {
  const { history, overview } = market;

  const samples = history.map((entry) => {
    const prices = entry.venues.map((venue) => venue.priceUsd);
    const medianPrice = median(prices);
    const spread = Math.max(...prices) - Math.min(...prices);
    const gasCost = (entry.gasPriceGwei * GAS_UNITS_ESTIMATE * medianPrice) / 1_000_000_000;
    return spread - gasCost;
  });

  const current = samples.length > 0 ? samples[samples.length - 1] : null;
  const positive = samples.filter((value) => value > 0).length;
  const negative = samples.length - positive;

  const tone = current === null ? "muted" : current > 0 ? "up" : "down";
  const sparklineTone = current === null ? "info" : current > 0 ? "up" : "down";

  const extremes = overview ? findExtremes(overview.venues) : null;
  const gasCostNow = overview
    ? (overview.gasPriceGwei *
        GAS_UNITS_ESTIMATE *
        median(overview.venues.map((venue) => venue.priceUsd))) /
      1_000_000_000
    : null;

  return (
    <Card
      title="Gas-adj. spread"
      subtitle={overview ? formatGweiSmart(overview.gasPriceGwei) : "—"}
      onRemove={onRemove}
    >
      <div className="gas-content">
        <div className="gas-headline">
          <span
            className={`metric-value is-large ${
              tone === "up" ? "is-up" : tone === "down" ? "is-down" : "is-muted"
            }`}
          >
            {current === null ? "—" : formatSignedUsd(current)}
          </span>
          {extremes ? (
            <div className="gas-route">
              <span className="gas-route-tag is-up">BUY</span>
              <span className="route-row-venue">
                <span className={`dot ${venueSwatchByIndex(extremes.cheapestIndex)}`} />
                {shortenVenueName(extremes.cheapest.dexName)}
              </span>
              <span className="gas-route-arrow">→</span>
              <span className="gas-route-tag is-down">SELL</span>
              <span className="route-row-venue">
                <span className={`dot ${venueSwatchByIndex(extremes.richestIndex)}`} />
                {shortenVenueName(extremes.richest.dexName)}
              </span>
            </div>
          ) : null}
        </div>

        <div style={{ flex: 1, minHeight: 24 }}>
          <Sparkline values={samples} tone={sparklineTone} filled />
        </div>

        <RatioBar
          segments={[
            { value: positive, tone: "up" },
            { value: negative, tone: "down" },
          ]}
          height={4}
        />

        <span className="gas-trail">
          {gasCostNow !== null
            ? `gas −${formatUsd(gasCostNow)} · ${positive}/${samples.length} ticks +ve`
            : "Awaiting"}
        </span>
      </div>
    </Card>
  );
}
