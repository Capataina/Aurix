import { useMemo, useState } from "react";
import type { DragEvent } from "react";
import type { Layout, LayoutItem } from "react-grid-layout";

import { BlockDrawer } from "../../components/shell/BlockDrawer";
import {
  COLS,
  DashboardGrid,
  type Breakpoint,
  type ResponsiveLayoutMap,
} from "../../components/shell/DashboardGrid";
import {
  ALL_BLOCKS,
  getBlockDefinition,
  type BlockDefinition,
} from "../../components/blocks/BlockRegistry";
import type { MarketState } from "../../hooks/useMarketData";
import { usePersistedState } from "../../hooks/usePersistedState";
import type { PnlMode } from "../../lib/arbitrage";

interface ArbitragePageProps {
  market: MarketState;
  pnlMode: PnlMode;
  drawerOpen: boolean;
  onCloseDrawer: () => void;
}

const TAB_ID = "arbitrage";
const LAYOUT_STORAGE_KEY = `aurix:layout:${TAB_ID}`;

/**
 * RGL's internal placeholder sentinel id. RGL hardcodes this in its drop
 * filtering (`layout.filter(l.i !== droppingItem.i)` before each
 * onLayoutChange) so we MUST keep `droppingItem.i` set to this string —
 * otherwise the filter would strip a real block from every layout-change
 * event and we'd lose blocks on drop.
 */
const DROPPING_ITEM_ID = "__dropping-elem__";

const BREAKPOINT_ORDER: Breakpoint[] = ["lg", "md", "sm", "xs"];

/**
 * Curated default. Three vertical zones across all twelve columns:
 *   Row 1 (h=4): Live | Spread (h=2) / Gas (h=2) sit above Price Chart |
 *                Arb matrix
 *   Row 2 (h=3): Venues under Live; Price Chart fills middle; Volatility
 *                under Arb matrix
 *   Row 3 (h=4): Best route | Signals | Ladder
 *
 * `Spread` and `Gas-adj` are kept thin (h=2) on purpose — the route
 * info sits inline with the headline number, so the histograms below
 * fit comfortably in two rows.
 */
const DEFAULT_LAYOUTS: ResponsiveLayoutMap = {
  lg: [
    // Top band — hero + small at-a-glance metrics + arb matrix
    { i: "hero-price", x: 0, y: 0, w: 3, h: 4, minW: 3, minH: 3 },
    { i: "spread-tracker", x: 3, y: 0, w: 3, h: 2, minW: 3, minH: 2 },
    { i: "gas-opportunity", x: 6, y: 0, w: 3, h: 2, minW: 3, minH: 2 },
    { i: "arbitrage-matrix", x: 9, y: 0, w: 3, h: 4, minW: 3, minH: 3 },
    // Middle band — chart hero + complementary blocks left/right
    { i: "price-chart", x: 3, y: 2, w: 6, h: 5, minW: 4, minH: 4 },
    { i: "venue-grid", x: 0, y: 4, w: 3, h: 3, minW: 3, minH: 3 },
    { i: "volatility", x: 9, y: 4, w: 3, h: 3, minW: 3, minH: 3 },
    // Bottom band — explicit route + signals + ladder
    { i: "arb-route", x: 0, y: 7, w: 4, h: 4, minW: 4, minH: 4 },
    { i: "insights", x: 4, y: 7, w: 5, h: 4, minW: 3, minH: 4 },
    { i: "price-ladder", x: 9, y: 7, w: 3, h: 4, minW: 3, minH: 3 },
  ],
  md: [
    { i: "hero-price", x: 0, y: 0, w: 3, h: 4, minW: 3, minH: 3 },
    { i: "spread-tracker", x: 3, y: 0, w: 3, h: 2, minW: 3, minH: 2 },
    { i: "gas-opportunity", x: 6, y: 0, w: 4, h: 2, minW: 3, minH: 2 },
    { i: "price-chart", x: 3, y: 2, w: 7, h: 5, minW: 4, minH: 4 },
    { i: "venue-grid", x: 0, y: 4, w: 3, h: 3, minW: 3, minH: 3 },
    { i: "arb-route", x: 0, y: 7, w: 4, h: 4, minW: 4, minH: 4 },
    { i: "insights", x: 4, y: 7, w: 6, h: 4, minW: 3, minH: 4 },
    { i: "arbitrage-matrix", x: 0, y: 11, w: 4, h: 3, minW: 3, minH: 3 },
    { i: "volatility", x: 4, y: 11, w: 3, h: 3, minW: 3, minH: 3 },
    { i: "price-ladder", x: 7, y: 11, w: 3, h: 3, minW: 3, minH: 3 },
  ],
  sm: [
    { i: "hero-price", x: 0, y: 0, w: 3, h: 4, minW: 3, minH: 3 },
    { i: "arb-route", x: 3, y: 0, w: 3, h: 4, minW: 3, minH: 4 },
    { i: "price-chart", x: 0, y: 4, w: 6, h: 5, minW: 4, minH: 4 },
    { i: "venue-grid", x: 0, y: 9, w: 3, h: 3, minW: 3, minH: 3 },
    { i: "price-ladder", x: 3, y: 9, w: 3, h: 3, minW: 3, minH: 3 },
    { i: "spread-tracker", x: 0, y: 12, w: 3, h: 3, minW: 3, minH: 3 },
    { i: "gas-opportunity", x: 3, y: 12, w: 3, h: 3, minW: 3, minH: 3 },
    { i: "arbitrage-matrix", x: 0, y: 15, w: 3, h: 3, minW: 3, minH: 3 },
    { i: "volatility", x: 3, y: 15, w: 3, h: 3, minW: 3, minH: 3 },
    { i: "insights", x: 0, y: 18, w: 6, h: 4, minW: 3, minH: 4 },
  ],
  xs: [
    { i: "hero-price", x: 0, y: 0, w: 4, h: 3, minW: 3, minH: 3 },
    { i: "arb-route", x: 0, y: 3, w: 4, h: 4, minW: 3, minH: 4 },
    { i: "price-chart", x: 0, y: 7, w: 4, h: 5, minW: 3, minH: 4 },
    { i: "venue-grid", x: 0, y: 12, w: 4, h: 3, minW: 3, minH: 3 },
    { i: "spread-tracker", x: 0, y: 15, w: 4, h: 3, minW: 3, minH: 3 },
    { i: "gas-opportunity", x: 0, y: 18, w: 4, h: 3, minW: 3, minH: 3 },
    { i: "arbitrage-matrix", x: 0, y: 21, w: 4, h: 3, minW: 3, minH: 3 },
    { i: "price-ladder", x: 0, y: 24, w: 4, h: 4, minW: 3, minH: 3 },
  ],
};

function activeBlockIdsFromLayouts(layouts: ResponsiveLayoutMap): Set<string> {
  const ids = new Set<string>();
  for (const breakpointLayout of Object.values(layouts)) {
    if (!breakpointLayout) continue;
    for (const item of breakpointLayout) {
      if (item.i !== DROPPING_ITEM_ID) {
        ids.add(item.i);
      }
    }
  }
  return ids;
}

function clampPosition(x: number, w: number, cols: number): number {
  return Math.max(0, Math.min(x, cols - w));
}

export function ArbitragePage({ market, pnlMode, drawerOpen, onCloseDrawer }: ArbitragePageProps) {
  const [layouts, setLayouts] = usePersistedState<ResponsiveLayoutMap>(
    LAYOUT_STORAGE_KEY,
    DEFAULT_LAYOUTS,
  );
  /**
   * Block definition currently being dragged. Used only to size the
   * dropping placeholder; the placeholder's id stays the RGL sentinel so
   * RGL's internal filtering doesn't strip real blocks from layout-change
   * events.
   */
  const [draggedBlock, setDraggedBlock] = useState<BlockDefinition | null>(null);

  const droppingItem = useMemo<LayoutItem>(() => {
    if (draggedBlock) {
      return {
        i: DROPPING_ITEM_ID,
        w: draggedBlock.defaultW,
        h: draggedBlock.defaultH,
        x: 0,
        y: 0,
      };
    }
    return {
      i: DROPPING_ITEM_ID,
      w: 4,
      h: 3,
      x: 0,
      y: 0,
    };
  }, [draggedBlock]);

  const activeBlockIds = useMemo(() => activeBlockIdsFromLayouts(layouts), [layouts]);

  const handleDragStart = (
    block: BlockDefinition,
    event: DragEvent<HTMLDivElement>,
  ) => {
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("application/x-aurix-block", block.id);
    setDraggedBlock(block);
  };

  const handleDragEnd = () => {
    setDraggedBlock(null);
  };

  const handleDrop = (
    _layout: Layout,
    item: LayoutItem | undefined,
    event: Event,
    activeBreakpoint: Breakpoint,
  ) => {
    if (!item) {
      setDraggedBlock(null);
      return;
    }

    // The placeholder id is the RGL sentinel; the real block id comes from
    // the dataTransfer payload set in handleDragStart.
    const dragEvent = event as unknown as DragEvent;
    const blockId =
      dragEvent.dataTransfer?.getData("application/x-aurix-block") ?? "";
    const definition = getBlockDefinition(blockId);

    if (!definition) {
      setDraggedBlock(null);
      return;
    }
    if (activeBlockIds.has(definition.id)) {
      setDraggedBlock(null);
      return;
    }

    setLayouts((current) => {
      const next: ResponsiveLayoutMap = {};
      for (const breakpoint of BREAKPOINT_ORDER) {
        const existing = (current[breakpoint] ?? []).filter(
          (entry) => entry.i !== definition.id && entry.i !== DROPPING_ITEM_ID,
        );

        const cols = COLS[breakpoint];
        const widthForBp = Math.min(definition.defaultW, cols);
        const isActive = breakpoint === activeBreakpoint;
        const newItem: LayoutItem = {
          i: definition.id,
          x: isActive ? clampPosition(item.x, widthForBp, cols) : 0,
          y: isActive ? item.y : Number.POSITIVE_INFINITY,
          w: widthForBp,
          h: definition.defaultH,
          minW: Math.min(definition.minW, cols),
          minH: definition.minH,
        };

        next[breakpoint] = [...existing, newItem];
      }
      return next;
    });

    setDraggedBlock(null);
  };

  /**
   * Merge incoming layouts with the parent's current state.
   *
   * RGL fires `onLayoutChange` immediately after its drop handler runs. At
   * that moment, RGL's internal layout has been stripped of the placeholder
   * (via `removeDroppingPlaceholder`) but the new block we just added in
   * `handleDrop` has only been written to OUR parent state — RGL hasn't
   * synced from props yet. If we just overwrote with the incoming layout
   * here, the just-added block would disappear and reappear as RGL re-syncs,
   * producing the flickering. By preserving items that exist in `current`
   * but not in `incoming`, we hold onto the new block until RGL catches up
   * (after which the item is in both, and the merge is a no-op).
   *
   * For drag/resize of existing blocks, every block in `current` is also in
   * `incoming` (RGL doesn't drop items spontaneously), so the preservation
   * logic is a no-op and the new positions/sizes flow through normally.
   *
   * For deletion via the X button, the block is removed from `current`
   * before RGL fires this callback, so the merge correctly produces a layout
   * without the deleted block.
   */
  const handleLayoutsChange = (incoming: ResponsiveLayoutMap) => {
    setLayouts((current) => {
      const next: ResponsiveLayoutMap = {};
      for (const breakpoint of BREAKPOINT_ORDER) {
        const incomingForBp = (incoming[breakpoint] ?? []).filter(
          (item) => item.i !== DROPPING_ITEM_ID,
        );
        const currentForBp = current[breakpoint] ?? [];

        const incomingIds = new Set(incomingForBp.map((entry) => entry.i));
        const preserved = currentForBp.filter(
          (entry) => entry.i !== DROPPING_ITEM_ID && !incomingIds.has(entry.i),
        );

        next[breakpoint] = [...incomingForBp, ...preserved];
      }
      return next;
    });
  };

  const handleRemoveBlock = (blockId: string) => {
    setLayouts((current) => {
      const next: ResponsiveLayoutMap = {};
      for (const breakpoint of BREAKPOINT_ORDER) {
        const existing = current[breakpoint];
        if (existing) {
          next[breakpoint] = existing.filter((item) => item.i !== blockId);
        }
      }
      return next;
    });
  };

  const handleResetLayout = () => {
    setLayouts(DEFAULT_LAYOUTS);
  };

  const liveLayout = layouts.lg ?? [];
  const visibleBlocks = liveLayout
    .map((item) => getBlockDefinition(item.i))
    .filter((definition): definition is BlockDefinition => Boolean(definition));

  return (
    <>
      <div className="app-canvas">
        {visibleBlocks.length === 0 ? (
          <div className="app-empty">
            <div className="app-empty-card">
              <span className="eyebrow">Empty canvas</span>
              <h2 className="app-empty-title">No blocks placed yet</h2>
              <p className="app-empty-hint">
                Open the block library and drag any card onto the canvas. Resize from
                the bottom-right of each card.
              </p>
              <button
                type="button"
                className="app-empty-cta"
                onClick={handleResetLayout}
              >
                Restore default layout
              </button>
            </div>
          </div>
        ) : (
          <DashboardGrid
            layouts={layouts}
            onLayoutsChange={handleLayoutsChange}
            onDrop={handleDrop}
            droppingItem={droppingItem}
          >
            {visibleBlocks.map((block) => (
              <div key={block.id}>
                <block.Component
                  market={market}
                  pnlMode={pnlMode}
                  onRemove={() => handleRemoveBlock(block.id)}
                />
              </div>
            ))}
          </DashboardGrid>
        )}
      </div>

      <BlockDrawer
        open={drawerOpen}
        onClose={onCloseDrawer}
        blocks={ALL_BLOCKS}
        activeBlockIds={activeBlockIds}
        onDragStart={handleDragStart}
        onDragEnd={handleDragEnd}
        onResetLayout={handleResetLayout}
      />
    </>
  );
}
