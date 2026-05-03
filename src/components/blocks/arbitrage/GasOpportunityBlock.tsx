import { Card } from "../../primitives/Card";
import { RatioBar } from "../../primitives/RatioBar";
import { Sparkline } from "../../primitives/Sparkline";
import { findBestRoute } from "../../../lib/arbitrage";
import { formatGweiSmart, formatSignedUsd, formatUsd } from "../../../lib/format";
import { shortenVenueName, venueSwatchByIndex } from "../../../lib/venues";
import type { BlockRenderProps } from "./BlockRegistry";

export function GasOpportunityBlock({ market, pnlMode, onRemove }: BlockRenderProps) {
  const { history, overview } = market;

  // Per-tick best-route net under the active mode. The history series is what
  // the sparkline + ratio bar visualise, so switching mode re-derives both
  // without needing to mutate any state — same history, different math.
  const samples = history
    .map((entry) => findBestRoute(entry.venues, entry.gasPriceGwei, pnlMode))
    .filter((route): route is NonNullable<typeof route> => route !== null);

  const nets = samples.map((route) => route.netUsd);
  const current = nets.length > 0 ? nets[nets.length - 1] : null;
  const positive = nets.filter((value) => value > 0).length;
  const negative = nets.length - positive;

  const tone = current === null ? "muted" : current > 0 ? "up" : "down";
  const sparklineTone = current === null ? "info" : current > 0 ? "up" : "down";

  const live = overview ? findBestRoute(overview.venues, overview.gasPriceGwei, pnlMode) : null;
  const subtitleParts: string[] = [];
  if (overview) subtitleParts.push(formatGweiSmart(overview.gasPriceGwei));
  if (pnlMode === "gas-and-fees") subtitleParts.push("incl. fees");
  const subtitle = subtitleParts.join(" · ") || "—";

  return (
    <Card title="Net spread" subtitle={subtitle} onRemove={onRemove}>
      <div className="gas-content">
        <div className="gas-headline">
          <span
            className={`metric-value is-large ${
              tone === "up" ? "is-up" : tone === "down" ? "is-down" : "is-muted"
            }`}
          >
            {current === null ? "—" : formatSignedUsd(current)}
          </span>
          {live ? (
            <div className="gas-route">
              <span className="gas-route-tag is-up">BUY</span>
              <span className="route-row-venue">
                <span className={`dot ${venueSwatchByIndex(live.buyIndex)}`} />
                {shortenVenueName(live.buy.dexName)}
              </span>
              <span className="gas-route-arrow">→</span>
              <span className="gas-route-tag is-down">SELL</span>
              <span className="route-row-venue">
                <span className={`dot ${venueSwatchByIndex(live.sellIndex)}`} />
                {shortenVenueName(live.sell.dexName)}
              </span>
            </div>
          ) : null}
        </div>

        <div style={{ flex: 1, minHeight: 24 }}>
          <Sparkline values={nets} tone={sparklineTone} filled />
        </div>

        <RatioBar
          segments={[
            { value: positive, tone: "up" },
            { value: negative, tone: "down" },
          ]}
          height={4}
        />

        <span className="gas-trail">
          {live
            ? pnlMode === "gas-and-fees"
              ? `gas −${formatUsd(live.gasCostUsd)} · fees −${formatUsd(live.feeCostUsd)} · ${positive}/${nets.length} +ve`
              : `gas −${formatUsd(live.gasCostUsd)} · ${positive}/${nets.length} ticks +ve`
            : "Awaiting"}
        </span>
      </div>
    </Card>
  );
}
