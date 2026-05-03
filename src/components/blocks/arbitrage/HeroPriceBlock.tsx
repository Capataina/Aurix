import { Card } from "../../primitives/Card";
import { Pill } from "../../primitives/Pill";
import { Sparkline } from "../../primitives/Sparkline";
import { formatSignedPercent, formatUsd } from "../../../lib/format";
import { shortenVenueName, venueSwatchByIndex } from "../../../lib/venues";
import type { BlockRenderProps } from "./BlockRegistry";

export function HeroPriceBlock({ market, onRemove }: BlockRenderProps) {
  const { heroSnapshot, history, overview, errorMessage } = market;
  const previous = history.length > 1 ? history[history.length - 2] : null;
  const previousHero = previous?.venues[0]?.priceUsd ?? null;
  const change =
    previousHero && heroSnapshot
      ? ((heroSnapshot.priceUsd - previousHero) / previousHero) * 100
      : null;

  const priceSeries = history.map((entry) => entry.venues[0]?.priceUsd ?? 0);
  const tone = change === null ? "neutral" : change > 0 ? "up" : change < 0 ? "down" : "neutral";

  const heroVenueLabel = heroSnapshot
    ? shortenVenueName(heroSnapshot.dexName)
    : "—";
  const subtitle = overview
    ? `${heroVenueLabel} · ${overview.pairLabel}`
    : "—";

  return (
    <Card title="Live price" subtitle={subtitle} onRemove={onRemove}>
      <div className="hero-block">
        <div className="hero-meta-row">
          <Pill tone={errorMessage ? "down" : "up"} showDot pulse={!errorMessage}>
            {errorMessage ? "Err" : "Live"}
          </Pill>
          {heroSnapshot ? (
            <span className="hero-venue-label">
              <span className={`dot ${venueSwatchByIndex(0)}`} />
              {heroVenueLabel}
            </span>
          ) : null}
          {change !== null ? (
            <Pill tone={tone}>{formatSignedPercent(change, 3)}</Pill>
          ) : null}
        </div>

        <div className="hero-price-row">
          <span className="hero-price-value">
            {heroSnapshot ? formatUsd(heroSnapshot.priceUsd) : "—"}
          </span>
          <div className="hero-spark">
            <Sparkline values={priceSeries} tone={tone === "down" ? "down" : "accent"} filled />
          </div>
        </div>
      </div>
    </Card>
  );
}
