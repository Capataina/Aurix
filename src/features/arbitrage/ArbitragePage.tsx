import {
  ALL_BLOCKS,
  type BlockDefinition,
  type BlockRenderProps,
} from "../../components/blocks/arbitrage/BlockRegistry";
import type { MarketState } from "../../hooks/useMarketData";
import type { PnlMode } from "../../lib/arbitrage";
import { useTelemetrySnapshot } from "../../lib/telemetry";

interface ArbitragePageProps {
  market: MarketState;
  pnlMode: PnlMode;
}

/**
 * Tab 1 layout — stacked, scrollable, all blocks always visible.
 *
 * Each row is a CSS grid using the `.dashboard-row-*` ratio classes
 * defined in `dashboard.css`. Block IDs match the keys in `BLOCK_REGISTRY`;
 * an unknown ID is a no-op and is filtered out at render time.
 *
 * Layout intent: front-load action-information so the first viewport
 * answers "what's the trade right now?" before any ambient chart space.
 *
 *   1. At-a-glance trio: live price · current spread · gas-adjusted P/L
 *   2. Action duo:       best route + per-venue grid
 *   3. Reasoning:        auto-derived signals + summary stats
 *   4. Reference chart:  price chart (full width)
 *   5. Analytical trio:  momentum · volatility · price ladder
 *   6. Visual heatmaps:  arb matrix + venue heatmap
 *   7. Footer:           event timeline + connection summary
 */
const ARBITRAGE_LAYOUT: Array<{ row: string; ids: string[] }> = [
  { row: "dashboard-row-1-1-1", ids: ["hero-price", "spread-tracker", "gas-opportunity"] },
  { row: "dashboard-row-1-1", ids: ["arb-route", "venue-grid"] },
  { row: "dashboard-row-2-1", ids: ["insights", "quick-stats"] },
  { row: "dashboard-row-1", ids: ["price-chart"] },
  { row: "dashboard-row-1-1-1", ids: ["momentum", "volatility", "price-ladder"] },
  { row: "dashboard-row-1-1", ids: ["arbitrage-matrix", "venue-heatmap"] },
  { row: "dashboard-row-2-1", ids: ["event-log", "connection"] },
];

function findBlock(id: string): BlockDefinition | undefined {
  return ALL_BLOCKS.find((b) => b.id === id);
}

export function ArbitragePage({ market, pnlMode }: ArbitragePageProps) {
  const renderProps: BlockRenderProps = { market, pnlMode };

  // Snapshot the visible state so the telemetry log mirrors what's
  // on screen without screenshots. Heavy structures (full overview
  // venue array) are reduced to count + first/hero pick.
  const overview = market.overview;
  useTelemetrySnapshot("arbitrage", {
    pnlMode,
    loading: market.loading,
    errorMessage: market.errorMessage,
    heroSnapshot: market.heroSnapshot,
    overview: overview
      ? {
          chain: overview.chain,
          fetchedAtUnixMs: overview.fetchedAtUnixMs,
          venuesCount: overview.venues.length,
          venuesFirst: overview.venues[0] ?? null,
        }
      : null,
    historyCount: market.history?.length ?? 0,
    historyLast: market.history?.length
      ? market.history[market.history.length - 1]
      : null,
  });

  return (
    <div className="dashboard-page">
      <div className="dashboard-grid">
        {ARBITRAGE_LAYOUT.map((rowDef, rowIdx) => {
          const blocks = rowDef.ids
            .map((id) => findBlock(id))
            .filter((b): b is BlockDefinition => Boolean(b));
          if (blocks.length === 0) return null;
          return (
            <div key={rowIdx} className={`dashboard-row ${rowDef.row}`}>
              {blocks.map((block) => (
                <block.Component key={block.id} {...renderProps} />
              ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}
