import type { ReactNode } from "react";

import { CloseIcon, GripIcon } from "./Icon";

interface CardProps {
  title: string;
  subtitle?: string;
  /** Content rendered in the right side of the header before the remove button. */
  headerExtra?: ReactNode;
  /** Body content. Use the `bodyClassName` prop to opt into stack/grid presets. */
  children: ReactNode;
  /** Additional class for the body wrapper. */
  bodyClassName?: string;
  /** Called when the user clicks the remove button. */
  onRemove?: () => void;
  /** When true, the body skips the default padding. Useful for charts. */
  bodyFlush?: boolean;
}

/**
 * The card primitive every block renders inside. The `.card-handle` element
 * is the registered drag handle for react-grid-layout — clicks elsewhere
 * (e.g. on chart-mode buttons or the remove icon) do not initiate a drag.
 */
export function Card({
  title,
  subtitle,
  headerExtra,
  children,
  bodyClassName,
  onRemove,
  bodyFlush = false,
}: CardProps) {
  const bodyClasses = [
    "card-body",
    bodyFlush ? "is-padded-tight" : "",
    bodyClassName ?? "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div className="card">
      <header className="card-header">
        <span className="card-handle" aria-label="Drag block">
          <GripIcon />
        </span>
        <div className="card-title">
          {title}
          {subtitle ? <span className="card-subtitle">{subtitle}</span> : null}
        </div>
        <div className="card-actions">
          {headerExtra}
          {onRemove ? (
            <button
              type="button"
              className="card-action is-danger"
              aria-label="Remove block"
              onClick={onRemove}
            >
              <CloseIcon />
            </button>
          ) : null}
        </div>
      </header>
      <div className={bodyClasses}>{children}</div>
    </div>
  );
}
