import { Card } from "../primitives/Card";
import { Pill } from "../primitives/Pill";
import { GAS_UNITS_ESTIMATE } from "../../lib/config";
import { findBestRoute } from "../../lib/arbitrage";
import {
  formatGweiSmart,
  formatPreciseUsd,
  formatSignedUsd,
  formatUsd,
} from "../../lib/format";
import { shortenVenueName, venueSwatchByIndex } from "../../lib/venues";
import type { BlockRenderProps } from "./BlockRegistry";

/**
 * Read-it-in-one-glance trade summary. In `gas` mode the math is
 *   spread − gas = net.
 * In `gas-and-fees` mode the math is the full pool-fee-aware round trip:
 *   USDC paid (price/(1-buyFee))   minus
 *   USDC received (price·(1-sellFee)) minus
 *   gas cost = net.
 *
 * The same `findBestRoute()` call powers both modes — switching modes only
 * changes which pair wins (fees can flip the optimum) and what's shown in
 * the math breakdown.
 */
export function ArbRouteBlock({ market, pnlMode, onRemove }: BlockRenderProps) {
  const { overview } = market;

  if (!overview || overview.venues.length === 0) {
    return (
      <Card title="Best route" subtitle="—" onRemove={onRemove}>
        <div className="card-empty">Awaiting first sample.</div>
      </Card>
    );
  }

  const route = findBestRoute(overview.venues, overview.gasPriceGwei, pnlMode);
  if (!route) {
    return (
      <Card title="Best route" subtitle="—" onRemove={onRemove}>
        <div className="card-empty">Awaiting first sample.</div>
      </Card>
    );
  }

  const verdictTone = route.netUsd > 0 ? "up" : "down";
  const verdictLabel = route.netUsd > 0 ? "PROFITABLE" : "UNPROFITABLE";

  const subtitle = `${shortenVenueName(route.buy.dexName)} → ${shortenVenueName(
    route.sell.dexName,
  )}`;

  return (
    <Card
      title="Best route"
      subtitle={subtitle}
      onRemove={onRemove}
      headerExtra={
        <Pill tone={verdictTone} showDot pulse={route.netUsd > 0}>
          {verdictLabel}
        </Pill>
      }
    >
      <div className="arb-route-content">
        <div className="arb-route-leg">
          <span className="arb-route-leg-tag is-up">BUY</span>
          <span className="arb-route-venue">
            <span className={`dot ${venueSwatchByIndex(route.buyIndex)}`} />
            {shortenVenueName(route.buy.dexName)}
          </span>
          <span className="arb-route-price mono">
            {pnlMode === "gas-and-fees"
              ? formatPreciseUsd(route.buyCostUsd, 2)
              : formatPreciseUsd(route.buy.priceUsd, 2)}
          </span>
        </div>

        <div className="arb-route-leg">
          <span className="arb-route-leg-tag is-down">SELL</span>
          <span className="arb-route-venue">
            <span className={`dot ${venueSwatchByIndex(route.sellIndex)}`} />
            {shortenVenueName(route.sell.dexName)}
          </span>
          <span className="arb-route-price mono">
            {pnlMode === "gas-and-fees"
              ? formatPreciseUsd(route.sellProceedsUsd, 2)
              : formatPreciseUsd(route.sell.priceUsd, 2)}
          </span>
        </div>

        <div className="route-divider" />

        <div className="arb-route-math">
          {pnlMode === "gas-and-fees" ? (
            <>
              <div className="arb-route-math-row">
                <span className="arb-route-math-label">spread</span>
                <span className="arb-route-math-value mono">
                  {formatUsd(route.spreadUsd)}
                </span>
              </div>
              <div className="arb-route-math-row">
                <span className="arb-route-math-label">
                  pool fees{" "}
                  <span
                    className="mono"
                    style={{ fontSize: 10, color: "var(--text-muted)" }}
                  >
                    ({(route.buy.feeTierBps / 100).toFixed(2)}% +{" "}
                    {(route.sell.feeTierBps / 100).toFixed(2)}%)
                  </span>
                </span>
                <span className="arb-route-math-value mono is-down">
                  −{formatUsd(route.feeCostUsd)}
                </span>
              </div>
              <div className="arb-route-math-row">
                <span className="arb-route-math-label">
                  gas{" "}
                  <span
                    className="mono"
                    style={{ fontSize: 10, color: "var(--text-muted)" }}
                  >
                    ({formatGweiSmart(overview.gasPriceGwei)} ×{" "}
                    {GAS_UNITS_ESTIMATE.toLocaleString()})
                  </span>
                </span>
                <span className="arb-route-math-value mono is-down">
                  −{formatUsd(route.gasCostUsd)}
                </span>
              </div>
            </>
          ) : (
            <>
              <div className="arb-route-math-row">
                <span className="arb-route-math-label">spread</span>
                <span className="arb-route-math-value mono">
                  {formatUsd(route.spreadUsd)}
                </span>
              </div>
              <div className="arb-route-math-row">
                <span className="arb-route-math-label">
                  gas{" "}
                  <span
                    className="mono"
                    style={{ fontSize: 10, color: "var(--text-muted)" }}
                  >
                    ({formatGweiSmart(overview.gasPriceGwei)} ×{" "}
                    {GAS_UNITS_ESTIMATE.toLocaleString()})
                  </span>
                </span>
                <span className="arb-route-math-value mono is-down">
                  −{formatUsd(route.gasCostUsd)}
                </span>
              </div>
            </>
          )}
          <div className="arb-route-math-row arb-route-math-net">
            <span className="arb-route-math-label">net</span>
            <span
              className={`arb-route-math-value mono ${
                route.netUsd > 0 ? "is-up" : "is-down"
              }`}
            >
              {formatSignedUsd(route.netUsd)}
            </span>
          </div>
        </div>
      </div>
    </Card>
  );
}
