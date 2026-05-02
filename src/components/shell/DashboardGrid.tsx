import { useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import {
  ResponsiveGridLayout,
  getBreakpointFromWidth,
  type Layout,
  type LayoutItem,
} from "react-grid-layout";

import "react-grid-layout/css/styles.css";

export const BREAKPOINTS = { lg: 1200, md: 996, sm: 768, xs: 0 } as const;
export const COLS = { lg: 12, md: 10, sm: 6, xs: 4 } as const;

export type Breakpoint = keyof typeof BREAKPOINTS;
export type ResponsiveLayoutMap = Partial<Record<Breakpoint, Layout>>;

interface DashboardGridProps {
  layouts: ResponsiveLayoutMap;
  onLayoutsChange: (layouts: ResponsiveLayoutMap) => void;
  onDrop: (
    layout: Layout,
    item: LayoutItem | undefined,
    event: Event,
    breakpoint: Breakpoint,
  ) => void;
  /** Size and id of the placeholder rendered while an external block is dragged in. */
  droppingItem: LayoutItem;
  children: ReactNode;
}

const MARGIN_PX = 14;
const MIN_ROW_HEIGHT = 36;

/**
 * Wrapper around react-grid-layout's ResponsiveGridLayout that:
 *   - Owns its own width via a ResizeObserver (RGL needs a number, not auto).
 *   - Restricts drag initiation to elements with `.card-handle`.
 *   - Tracks the active breakpoint so the parent can drop with correct
 *     coordinates per breakpoint.
 */
export function DashboardGrid({
  layouts,
  onLayoutsChange,
  onDrop,
  droppingItem,
  children,
}: DashboardGridProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [size, setSize] = useState<{ width: number; height: number }>({
    width: 0,
    height: 0,
  });
  const widthRef = useRef<number>(0);

  useEffect(() => {
    const node = containerRef.current;
    if (!node) {
      return;
    }

    const updateSize = () => {
      const nextWidth = node.clientWidth;
      const nextHeight = node.clientHeight;
      widthRef.current = nextWidth;
      setSize({ width: nextWidth, height: nextHeight });
    };

    updateSize();

    const observer = new ResizeObserver(() => updateSize());
    observer.observe(node);

    return () => {
      observer.disconnect();
    };
  }, []);

  /**
   * Total rows occupied by the active breakpoint's layout. Defines the row
   * count we slice the canvas into so the grid always fills 100% of the host.
   * As blocks are added / resized to extend further down, rowHeight shrinks
   * and the canvas continues to fit without scrolling.
   */
  const activeBreakpoint = (
    size.width > 0
      ? (getBreakpointFromWidth(BREAKPOINTS, size.width) as Breakpoint)
      : "lg"
  );
  const activeLayout = layouts[activeBreakpoint] ?? layouts.lg ?? [];
  const maxRow = activeLayout.reduce(
    (max, item) => Math.max(max, item.y + item.h),
    1,
  );

  const rowHeight = size.height > 0
    ? Math.max(
        MIN_ROW_HEIGHT,
        (size.height - MARGIN_PX * Math.max(maxRow - 1, 0)) / maxRow,
      )
    : 64;

  const handleLayoutChange = (
    _layout: Layout,
    allLayouts: Partial<Record<string, Layout>>,
  ) => {
    onLayoutsChange(allLayouts as ResponsiveLayoutMap);
  };

  const dragConfig = useMemo(
    () => ({ enabled: true, handle: ".card-handle", bounded: false, threshold: 3 }),
    [],
  );

  const dropConfig = useMemo(
    () => ({
      enabled: true,
      defaultItem: { w: droppingItem.w, h: droppingItem.h },
    }),
    [droppingItem.w, droppingItem.h],
  );

  return (
    <div ref={containerRef} className="dashboard-grid-host">
      {size.width > 0 ? (
        <ResponsiveGridLayout
          width={size.width}
          breakpoints={BREAKPOINTS}
          cols={COLS}
          layouts={layouts as Record<string, Layout>}
          rowHeight={rowHeight}
          margin={[MARGIN_PX, MARGIN_PX]}
          containerPadding={[0, 0]}
          dragConfig={dragConfig}
          dropConfig={dropConfig}
          droppingItem={droppingItem}
          onLayoutChange={handleLayoutChange}
          onDrop={(layout, item, event) => {
            const breakpoint = getBreakpointFromWidth(BREAKPOINTS, widthRef.current) as Breakpoint;
            onDrop(layout, item, event, breakpoint);
          }}
        >
          {children}
        </ResponsiveGridLayout>
      ) : null}
    </div>
  );
}
