import { useEffect, useRef } from "react";

import type { PriceSnapshot } from "../../features/arbitrage/types";
import {
  HISTORY_LIMIT_OPTIONS,
  type HistoryLimit,
} from "../../hooks/useMarketData";

export type StaleThresholdMs = 4000 | 8000 | 16000;
export const STALE_THRESHOLD_OPTIONS: StaleThresholdMs[] = [4000, 8000, 16000];

interface SettingsMenuProps {
  open: boolean;
  onClose: () => void;
  /** Available venues for the active pair. Used to populate the hero
   *  selector. */
  venues: PriceSnapshot[];
  /** `null` = "auto" (whatever venues[0] resolves to). */
  heroVenueDexName: string | null;
  onSelectHero: (dexName: string | null) => void;
  historyLimit: HistoryLimit;
  onSelectHistoryLimit: (limit: HistoryLimit) => void;
  staleThresholdMs: StaleThresholdMs;
  onSelectStaleThreshold: (ms: StaleThresholdMs) => void;
}

export function SettingsMenu({
  open,
  onClose,
  venues,
  heroVenueDexName,
  onSelectHero,
  historyLimit,
  onSelectHistoryLimit,
  staleThresholdMs,
  onSelectStaleThreshold,
}: SettingsMenuProps) {
  const menuRef = useRef<HTMLDivElement | null>(null);

  // Click-outside dismisses. The settings button on the topbar is excluded
  // (it has its own toggle handler) so clicking the gear closes via toggle
  // rather than via outside-click logic.
  useEffect(() => {
    if (!open) return;
    function handlePointerDown(event: PointerEvent) {
      const target = event.target as Element | null;
      if (!target) return;
      const menu = menuRef.current;
      const settingsButton = document.querySelector("[data-settings-trigger]");
      if (menu && menu.contains(target)) return;
      if (settingsButton && settingsButton.contains(target)) return;
      onClose();
    }
    document.addEventListener("pointerdown", handlePointerDown);
    return () => document.removeEventListener("pointerdown", handlePointerDown);
  }, [open, onClose]);

  // Escape closes.
  useEffect(() => {
    if (!open) return;
    function handleKey(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      ref={menuRef}
      className="settings-menu"
      role="dialog"
      aria-label="Application settings"
    >
      <header className="settings-menu-header">
        <span className="settings-menu-title">Settings</span>
        <span className="settings-menu-hint">applies live · persisted</span>
      </header>

      <Section label="Hero venue" hint="Which venue's price is the headline reading">
        <div className="settings-options">
          <SettingsOption
            label="Auto (first)"
            active={heroVenueDexName === null}
            onClick={() => onSelectHero(null)}
          />
          {venues.map((venue) => (
            <SettingsOption
              key={venue.dexName}
              label={venue.dexName}
              active={heroVenueDexName === venue.dexName}
              onClick={() => onSelectHero(venue.dexName)}
            />
          ))}
        </div>
      </Section>

      <Section label="History buffer" hint="Rolling window for chart + stats">
        <div className="settings-options is-row">
          {HISTORY_LIMIT_OPTIONS.map((opt) => (
            <SettingsOption
              key={opt}
              label={`${opt}`}
              active={historyLimit === opt}
              onClick={() => onSelectHistoryLimit(opt)}
            />
          ))}
        </div>
      </Section>

      <Section label="Stale threshold" hint="When the connection turns yellow">
        <div className="settings-options is-row">
          {STALE_THRESHOLD_OPTIONS.map((ms) => (
            <SettingsOption
              key={ms}
              label={`${ms / 1000}s`}
              active={staleThresholdMs === ms}
              onClick={() => onSelectStaleThreshold(ms)}
            />
          ))}
        </div>
      </Section>
    </div>
  );
}

function Section({
  label,
  hint,
  children,
}: {
  label: string;
  hint: string;
  children: React.ReactNode;
}) {
  return (
    <div className="settings-section">
      <div className="settings-section-head">
        <span className="settings-section-label">{label}</span>
        <span className="settings-section-hint">{hint}</span>
      </div>
      {children}
    </div>
  );
}

function SettingsOption({
  label,
  active,
  onClick,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className={`settings-option ${active ? "is-active" : ""}`}
      onClick={onClick}
    >
      {label}
    </button>
  );
}
