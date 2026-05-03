import {
  ALL_BLOCKS,
  type BlockDefinition,
  type BlockRenderProps,
} from "../../components/blocks/arbitrage/BlockRegistry";
import type { MarketState } from "../../hooks/useMarketData";
import type { PnlMode } from "../../lib/arbitrage";

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
 * Grouping intent (top → bottom):
 *   1. Hero strip: live price (wide) + glance stats
 *   2. Big chart: full-width price chart
 *   3. Trade reading: best route + auto-derived signals
 *   4. Venue spatial: per-venue grid + venue-tick heatmap
 *   5. Pairwise small metrics: spread / gas-adjusted / momentum
 *   6. Analytical row: volatility / arb matrix / price ladder
 *   7. Footer: events timeline + connection summary
 */
const ARBITRAGE_LAYOUT: Array<{ row: string; ids: string[] }> = [
  { row: "dashboard-row-2-1", ids: ["hero-price", "quick-stats"] },
  { row: "dashboard-row-1", ids: ["price-chart"] },
  { row: "dashboard-row-1-1", ids: ["arb-route", "insights"] },
  { row: "dashboard-row-1-1", ids: ["venue-grid", "venue-heatmap"] },
  { row: "dashboard-row-1-1-1", ids: ["spread-tracker", "gas-opportunity", "momentum"] },
  { row: "dashboard-row-1-1-1", ids: ["volatility", "arbitrage-matrix", "price-ladder"] },
  { row: "dashboard-row-2-1", ids: ["event-log", "connection"] },
];

function findBlock(id: string): BlockDefinition | undefined {
  return ALL_BLOCKS.find((b) => b.id === id);
}

export function ArbitragePage({ market, pnlMode }: ArbitragePageProps) {
  const renderProps: BlockRenderProps = { market, pnlMode };

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
