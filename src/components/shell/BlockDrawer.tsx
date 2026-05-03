import { useEffect } from "react";
import type { DragEvent } from "react";

import type { BlockDefinition } from "../blocks/arbitrage/BlockRegistry";
import { CloseIcon, ResetIcon } from "../primitives/Icon";

interface BlockDrawerProps {
  open: boolean;
  onClose: () => void;
  blocks: BlockDefinition[];
  /** Block ids currently present on the dashboard. Used to dim duplicates. */
  activeBlockIds: Set<string>;
  /** Drag started — the parent uses this to set the dropping placeholder size. */
  onDragStart: (block: BlockDefinition, event: DragEvent<HTMLDivElement>) => void;
  onDragEnd: () => void;
  onResetLayout: () => void;
}

/**
 * Click-outside-to-close is handled at the document level rather than via the
 * backdrop element. The backdrop has `pointer-events: none` so it never
 * intercepts drag events on their way to the grid behind it; that means we
 * cannot rely on its `onClick`. Instead, we attach a pointerdown listener
 * that explicitly excludes the drawer body and the topbar (so clicking the
 * pair selector or the Blocks toggle while the drawer is open behaves
 * correctly).
 */
export function BlockDrawer({
  open,
  onClose,
  blocks,
  activeBlockIds,
  onDragStart,
  onDragEnd,
  onResetLayout,
}: BlockDrawerProps) {
  useEffect(() => {
    if (!open) {
      return;
    }

    function handlePointerDown(event: PointerEvent) {
      const target = event.target as Element | null;
      if (!target) {
        return;
      }

      const drawerEl = document.querySelector(".drawer");
      const topbarEl = document.querySelector(".topbar");

      if (drawerEl && drawerEl.contains(target)) {
        return;
      }
      if (topbarEl && topbarEl.contains(target)) {
        return;
      }

      onClose();
    }

    document.addEventListener("pointerdown", handlePointerDown);
    return () => document.removeEventListener("pointerdown", handlePointerDown);
  }, [open, onClose]);

  return (
    <>
      <div
        className={`drawer-backdrop ${open ? "is-open" : ""}`}
        aria-hidden
      />
      <aside
        className={`drawer ${open ? "is-open" : ""}`}
        aria-hidden={!open}
        role="dialog"
        aria-label="Block library"
      >
        <header className="drawer-header">
          <div className="drawer-header-row">
            <span className="drawer-title">Block library</span>
            <button
              type="button"
              className="drawer-close"
              onClick={onClose}
              aria-label="Close block library"
            >
              <CloseIcon />
            </button>
          </div>
          <p className="drawer-hint">
            Drag any block onto the canvas. Drop where you want it; resize from the bottom-right of each card.
          </p>
        </header>

        <div className="drawer-body">
          <span className="drawer-section-label">Arbitrage</span>
          {blocks.map((block) => {
            const onGrid = activeBlockIds.has(block.id);
            return (
              <div
                key={block.id}
                className={`drawer-block ${onGrid ? "is-on-grid" : ""}`}
                draggable={!onGrid}
                onDragStart={(event) => {
                  if (onGrid) {
                    event.preventDefault();
                    return;
                  }
                  onDragStart(block, event);
                }}
                onDragEnd={onDragEnd}
                aria-disabled={onGrid}
              >
                <span className="drawer-block-icon" aria-hidden>
                  <block.Icon />
                </span>
                <span className="drawer-block-text">
                  <span className="drawer-block-title">
                    {block.title}
                    {onGrid ? <span className="drawer-block-on-grid-tag">On grid</span> : null}
                  </span>
                  <span className="drawer-block-desc">{block.description}</span>
                </span>
              </div>
            );
          })}
        </div>

        <footer className="drawer-footer">
          <span>Layout saved automatically</span>
          <button
            type="button"
            className="drawer-reset"
            onClick={onResetLayout}
            aria-label="Reset layout"
          >
            <ResetIcon style={{ width: 12, height: 12 }} />
            Reset
          </button>
        </footer>
      </aside>
    </>
  );
}
