// LP Backtester settings — rendered inside the top-bar SettingsMenu when
// the LP page is active. The block window is resolved from the chain
// head every run (lookbackBlocks → [head − N, head]); the tick range
// is anchored on the first swap's realised tick (± tickHalfWidth).

import type { RebalanceRule } from "./types";
import type { ChainId } from "./chains";
import { CHAIN_CONFIGS, CHAIN_LIST } from "./chains";
import { POOL_PRESETS, findPoolPreset, type PoolPreset } from "./pools";

export interface LpSettings {
  poolAddress: string;
  /** Chain the pool lives on. Drives subgraph URL + block-time math. */
  chainId: ChainId;
  /** Protocol family — Uniswap V3 / Sushi V3 / Pancake V3. Tier 3
   *  wires the protocol-specific subgraph URLs. */
  protocol: "uniswap-v3" | "sushiswap-v3" | "pancakeswap-v3";
  /** Trailing window size, in blocks. Resolved against the chain head
   *  every run: fromBlock = head − lookbackBlocks, toBlock = head. */
  lookbackBlocks: number;
  /** Half-width of the position's tick range. The auto-run pipeline
   *  sets tickLower = firstSwap.tick − N, tickUpper = firstSwap.tick + N. */
  tickHalfWidth: number;
  depositUsd: number;
  feeTierBps: number;
  mevHaircutBps: number;
  rule: RebalanceRule;
}

export const DEFAULT_LP_SETTINGS: LpSettings = {
  poolAddress: "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640",
  chainId: "ethereum",
  protocol: "uniswap-v3",
  lookbackBlocks: 1000,
  tickHalfWidth: 300,
  depositUsd: 10_000,
  feeTierBps: 5,
  mevHaircutBps: 5,
  rule: { kind: "static" },
};

const RULE_OPTIONS: Array<{ id: RebalanceRule["kind"]; label: string }> = [
  { id: "static", label: "Static" },
  { id: "schedule", label: "Schedule" },
  { id: "price_exit_threshold", label: "Price exit" },
  { id: "out_of_range_duration", label: "OOR" },
];

const LOOKBACK_PRESETS: number[] = [250, 500, 1000, 2000, 5000];

function ruleFromId(id: RebalanceRule["kind"], current: RebalanceRule): RebalanceRule {
  switch (id) {
    case "static":
      return { kind: "static" };
    case "schedule":
      return {
        kind: "schedule",
        every_n_blocks: current.kind === "schedule" ? current.every_n_blocks : 100,
      };
    case "price_exit_threshold":
      return {
        kind: "price_exit_threshold",
        central_pct:
          current.kind === "price_exit_threshold" ? current.central_pct : 0.5,
      };
    case "out_of_range_duration":
      return {
        kind: "out_of_range_duration",
        min_oor_blocks:
          current.kind === "out_of_range_duration" ? current.min_oor_blocks : 50,
      };
  }
}

interface LpSettingsFormProps {
  settings: LpSettings;
  onChange: (next: LpSettings) => void;
  onRerun: () => void;
  busy: boolean;
}

export function LpSettingsForm({
  settings,
  onChange,
  onRerun,
  busy,
}: LpSettingsFormProps) {
  const ruleArgs = (() => {
    if (settings.rule.kind === "schedule") {
      return (
        <NumberStepper
          label="Every N blocks"
          value={settings.rule.every_n_blocks}
          step={50}
          min={1}
          onChange={(v) =>
            onChange({ ...settings, rule: { kind: "schedule", every_n_blocks: v } })
          }
        />
      );
    }
    if (settings.rule.kind === "price_exit_threshold") {
      return (
        <NumberStepper
          label="Central pct"
          value={settings.rule.central_pct}
          step={0.05}
          min={0.05}
          max={1}
          decimals={2}
          onChange={(v) =>
            onChange({
              ...settings,
              rule: { kind: "price_exit_threshold", central_pct: v },
            })
          }
        />
      );
    }
    if (settings.rule.kind === "out_of_range_duration") {
      return (
        <NumberStepper
          label="Min OOR blocks"
          value={settings.rule.min_oor_blocks}
          step={10}
          min={1}
          onChange={(v) =>
            onChange({
              ...settings,
              rule: { kind: "out_of_range_duration", min_oor_blocks: v },
            })
          }
        />
      );
    }
    return null;
  })();

  // Match the current selection against the curated preset list so
  // the dropdown shows the right active item even after the user
  // edits chain/address manually.
  const activePreset = findPoolPreset(settings.poolAddress);
  const presetsForChain = POOL_PRESETS.filter(
    (p) => p.chainId === settings.chainId && p.protocol === settings.protocol,
  );

  function applyPreset(preset: PoolPreset) {
    onChange({
      ...settings,
      poolAddress: preset.address,
      chainId: preset.chainId,
      protocol: preset.protocol,
      // Fee tier is encoded in the preset label; the ingest path will
      // confirm via lp_pool_metadata. For now we leave feeTierBps as
      // user-set so re-runs don't surprise them with auto-overrides.
    });
  }

  return (
    <div className="lp-settings-form">
      <Section label="Chain" hint="Which Uniswap V3 deployment to backtest against">
        <div className="settings-options is-row">
          {CHAIN_LIST.map((chain) => (
            <button
              key={chain.id}
              type="button"
              className={`settings-option ${
                settings.chainId === chain.id ? "is-active" : ""
              }`}
              onClick={() => onChange({ ...settings, chainId: chain.id })}
            >
              {chain.label}
            </button>
          ))}
        </div>
      </Section>

      <Section label="Protocol" hint="V3 forks share the schema; URLs differ">
        <div className="settings-options is-row">
          {(
            [
              { id: "uniswap-v3", label: "Uniswap" },
              { id: "sushiswap-v3", label: "Sushi" },
              { id: "pancakeswap-v3", label: "Pancake" },
            ] as const
          ).map((p) => (
            <button
              key={p.id}
              type="button"
              className={`settings-option ${
                settings.protocol === p.id ? "is-active" : ""
              }`}
              onClick={() => onChange({ ...settings, protocol: p.id })}
            >
              {p.label}
            </button>
          ))}
        </div>
      </Section>

      <Section label="Pool" hint="Curated top-TVL pools — pick or paste a custom address below">
        <div className="settings-pool-list">
          {presetsForChain.length === 0 ? (
            <div className="settings-hint">No presets for this chain yet — paste an address below.</div>
          ) : (
            presetsForChain.map((preset) => (
              <button
                key={preset.id}
                type="button"
                className={`settings-pool-row ${
                  activePreset?.id === preset.id ? "is-active" : ""
                }`}
                onClick={() => applyPreset(preset)}
                title={preset.address}
              >
                <span className="settings-pool-row-label">{preset.label}</span>
                {preset.popular ? (
                  <span className="settings-pool-row-badge">popular</span>
                ) : null}
              </button>
            ))
          )}
        </div>
        <input
          type="text"
          className="settings-input is-mono"
          value={settings.poolAddress}
          onChange={(e) => onChange({ ...settings, poolAddress: e.target.value })}
          spellCheck={false}
          placeholder="0x… custom pool address"
        />
        <span className="settings-hint mono">
          {(() => {
            // Defensive: settings persisted before the chain selector was
            // introduced won't have chainId; usePersistedState now merges
            // defaults, but render path still tolerates an unknown id.
            const cfg = CHAIN_CONFIGS[settings.chainId] ?? CHAIN_CONFIGS.ethereum;
            return `${cfg.label} · ${cfg.blockTimeSeconds}s/block`;
          })()}
        </span>
      </Section>

      <Section label="Lookback" hint="Trailing block window — resolves against the chain head every run">
        <div className="settings-options is-row">
          {LOOKBACK_PRESETS.map((opt) => (
            <button
              key={opt}
              type="button"
              className={`settings-option ${
                settings.lookbackBlocks === opt ? "is-active" : ""
              }`}
              onClick={() => onChange({ ...settings, lookbackBlocks: opt })}
            >
              {opt} blocks
            </button>
          ))}
        </div>
        <span className="settings-hint mono">
          ≈ {Math.round((settings.lookbackBlocks * 12) / 60)} min of mainnet swaps
        </span>
      </Section>

      <Section label="Position">
        <div className="settings-grid-3">
          <NumberStepper
            label="Deposit USD"
            value={settings.depositUsd}
            step={1000}
            min={100}
            onChange={(v) => onChange({ ...settings, depositUsd: v })}
          />
          <NumberStepper
            label="Fee bps"
            value={settings.feeTierBps}
            step={5}
            min={1}
            onChange={(v) => onChange({ ...settings, feeTierBps: v })}
          />
          <NumberStepper
            label="MEV bps"
            value={settings.mevHaircutBps}
            step={1}
            min={0}
            decimals={1}
            onChange={(v) => onChange({ ...settings, mevHaircutBps: v })}
          />
        </div>
      </Section>

      <Section label="Range half-width" hint="Position spans firstSwap.tick ± this many ticks">
        <NumberStepper
          label="Ticks"
          value={settings.tickHalfWidth}
          step={50}
          min={10}
          onChange={(v) => onChange({ ...settings, tickHalfWidth: v })}
        />
      </Section>

      <Section label="Rebalance rule">
        <div className="settings-options is-row">
          {RULE_OPTIONS.map((option) => (
            <button
              key={option.id}
              type="button"
              className={`settings-option ${
                settings.rule.kind === option.id ? "is-active" : ""
              }`}
              onClick={() =>
                onChange({ ...settings, rule: ruleFromId(option.id, settings.rule) })
              }
            >
              {option.label}
            </button>
          ))}
        </div>
        {ruleArgs ? <div className="settings-rule-args">{ruleArgs}</div> : null}
      </Section>

      <div className="settings-rerun-row">
        <button
          type="button"
          className="settings-rerun-button"
          onClick={onRerun}
          disabled={busy}
        >
          {busy ? "Running…" : "Re-run pipeline"}
        </button>
      </div>
    </div>
  );
}

function Section({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="settings-section">
      <div className="settings-section-head">
        <span className="settings-section-label">{label}</span>
        {hint ? <span className="settings-section-hint">{hint}</span> : null}
      </div>
      {children}
    </div>
  );
}

interface NumberStepperProps {
  label: string;
  value: number;
  step?: number;
  min?: number;
  max?: number;
  decimals?: number;
  onChange: (value: number) => void;
}

function NumberStepper({
  label,
  value,
  step = 1,
  min,
  max,
  decimals = 0,
  onChange,
}: NumberStepperProps) {
  const clamp = (v: number) => {
    let next = v;
    if (min !== undefined) next = Math.max(min, next);
    if (max !== undefined) next = Math.min(max, next);
    return next;
  };
  const display = decimals > 0 ? value.toFixed(decimals) : value.toString();
  return (
    <label className="settings-stepper">
      <span className="settings-stepper-label">{label}</span>
      <div className="settings-stepper-row">
        <button
          type="button"
          className="settings-stepper-button"
          onClick={() => onChange(clamp(value - step))}
          aria-label={`Decrease ${label}`}
        >
          −
        </button>
        <input
          type="number"
          className="settings-stepper-input mono"
          value={display}
          step={step}
          onChange={(e) => {
            const parsed = parseFloat(e.target.value);
            if (Number.isFinite(parsed)) onChange(clamp(parsed));
          }}
        />
        <button
          type="button"
          className="settings-stepper-button"
          onClick={() => onChange(clamp(value + step))}
          aria-label={`Increase ${label}`}
        >
          +
        </button>
      </div>
    </label>
  );
}
