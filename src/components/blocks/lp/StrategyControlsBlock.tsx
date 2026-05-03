import { Card } from "../../primitives/Card";
import { Pill } from "../../primitives/Pill";
import { RefreshIcon } from "../../primitives/Icon";
import type { RebalanceRule } from "../../../features/lp-backtest/types";

export interface StrategyControlsState {
  poolAddress: string;
  fromBlock: number;
  toBlock: number;
  tickLower: number;
  tickUpper: number;
  depositUsd: number;
  feeTierBps: number;
  mevHaircutBps: number;
  rule: RebalanceRule;
}

interface StrategyControlsBlockProps {
  state: StrategyControlsState;
  onChange: (state: StrategyControlsState) => void;
  onRunBacktest: () => void;
  onRunSyntheticIngest: () => void;
  onRunLiveIngest: () => void;
  onRunGrid: () => void;
  onSynthesiseHeadline: () => void;
  busy: boolean;
  status: string;
  onRemove?: () => void;
}

const RULE_OPTIONS: Array<{ id: RebalanceRule["kind"]; label: string }> = [
  { id: "static", label: "Static" },
  { id: "schedule", label: "Schedule" },
  { id: "price_exit_threshold", label: "Price exit" },
  { id: "out_of_range_duration", label: "OOR" },
];

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
        central_pct: current.kind === "price_exit_threshold" ? current.central_pct : 0.5,
      };
    case "out_of_range_duration":
      return {
        kind: "out_of_range_duration",
        min_oor_blocks:
          current.kind === "out_of_range_duration" ? current.min_oor_blocks : 50,
      };
  }
}

export function StrategyControlsBlock({
  state,
  onChange,
  onRunBacktest,
  onRunSyntheticIngest,
  onRunLiveIngest,
  onRunGrid,
  onSynthesiseHeadline,
  busy,
  status,
  onRemove,
}: StrategyControlsBlockProps) {
  const ruleArgs = (() => {
    if (state.rule.kind === "schedule") {
      return (
        <NumberStepper
          label="Every N blocks"
          value={state.rule.every_n_blocks}
          step={50}
          min={1}
          onChange={(v) =>
            onChange({
              ...state,
              rule: { kind: "schedule", every_n_blocks: v },
            })
          }
        />
      );
    }
    if (state.rule.kind === "price_exit_threshold") {
      return (
        <NumberStepper
          label="Central pct"
          value={state.rule.central_pct}
          step={0.05}
          min={0.05}
          max={1}
          decimals={2}
          onChange={(v) =>
            onChange({
              ...state,
              rule: { kind: "price_exit_threshold", central_pct: v },
            })
          }
        />
      );
    }
    if (state.rule.kind === "out_of_range_duration") {
      return (
        <NumberStepper
          label="Min OOR blocks"
          value={state.rule.min_oor_blocks}
          step={10}
          min={1}
          onChange={(v) =>
            onChange({
              ...state,
              rule: { kind: "out_of_range_duration", min_oor_blocks: v },
            })
          }
        />
      );
    }
    return <div className="ctrl-arg-spacer" />;
  })();

  return (
    <Card
      title="Controls"
      subtitle={busy ? "running…" : status || "auto-runs on load"}
      onRemove={onRemove}
      headerExtra={
        status ? (
          <Pill tone={busy ? "info" : "up"} showDot pulse={busy}>
            {busy ? "Working" : "Ready"}
          </Pill>
        ) : null
      }
    >
      <div className="ctrl-stack">
        <div className="ctrl-section">
          <span className="ctrl-section-label">Pool address</span>
          <input
            type="text"
            className="ctrl-input is-mono"
            value={state.poolAddress}
            onChange={(e) => onChange({ ...state, poolAddress: e.target.value })}
            spellCheck={false}
          />
        </div>

        <div className="ctrl-section">
          <span className="ctrl-section-label">Block window</span>
          <div className="ctrl-grid-2">
            <NumberStepper
              label="From"
              value={state.fromBlock}
              step={100}
              min={0}
              onChange={(v) => onChange({ ...state, fromBlock: v })}
            />
            <NumberStepper
              label="To"
              value={state.toBlock}
              step={100}
              min={0}
              onChange={(v) => onChange({ ...state, toBlock: v })}
            />
          </div>
        </div>

        <div className="ctrl-section">
          <span className="ctrl-section-label">Tick range</span>
          <div className="ctrl-grid-2">
            <NumberStepper
              label="Lower"
              value={state.tickLower}
              step={10}
              onChange={(v) => onChange({ ...state, tickLower: v })}
            />
            <NumberStepper
              label="Upper"
              value={state.tickUpper}
              step={10}
              onChange={(v) => onChange({ ...state, tickUpper: v })}
            />
          </div>
        </div>

        <div className="ctrl-section">
          <span className="ctrl-section-label">Position</span>
          <div className="ctrl-grid-3">
            <NumberStepper
              label="Deposit USD"
              value={state.depositUsd}
              step={1000}
              min={100}
              onChange={(v) => onChange({ ...state, depositUsd: v })}
            />
            <NumberStepper
              label="Fee bps"
              value={state.feeTierBps}
              step={5}
              min={1}
              onChange={(v) => onChange({ ...state, feeTierBps: v })}
            />
            <NumberStepper
              label="MEV bps"
              value={state.mevHaircutBps}
              step={1}
              min={0}
              decimals={1}
              onChange={(v) => onChange({ ...state, mevHaircutBps: v })}
            />
          </div>
        </div>

        <div className="ctrl-section">
          <span className="ctrl-section-label">Rebalance rule</span>
          <div className="ctrl-segmented">
            {RULE_OPTIONS.map((option) => {
              const active = state.rule.kind === option.id;
              return (
                <button
                  key={option.id}
                  type="button"
                  className={`ctrl-segment ${active ? "is-active" : ""}`}
                  onClick={() =>
                    onChange({ ...state, rule: ruleFromId(option.id, state.rule) })
                  }
                >
                  {option.label}
                </button>
              );
            })}
          </div>
          <div className="ctrl-rule-args">{ruleArgs}</div>
        </div>

        <div className="ctrl-actions">
          <button
            type="button"
            className="ctrl-button is-primary"
            onClick={onRunBacktest}
            disabled={busy}
          >
            <RefreshIcon className="ctrl-button-icon" />
            Run backtest
          </button>
          <button
            type="button"
            className="ctrl-button"
            onClick={onRunGrid}
            disabled={busy}
          >
            Run grid
          </button>
          <button
            type="button"
            className="ctrl-button"
            onClick={onSynthesiseHeadline}
            disabled={busy}
          >
            Synthesise headline
          </button>
        </div>

        <div className="ctrl-actions is-secondary">
          <button
            type="button"
            className="ctrl-button is-ghost"
            onClick={onRunSyntheticIngest}
            disabled={busy}
            title="Generate sinusoidal swap stream over the block range — no API key needed."
          >
            Synthetic ingest
          </button>
          <button
            type="button"
            className="ctrl-button is-ghost"
            onClick={onRunLiveIngest}
            disabled={busy}
            title="Live archive ingest via Alchemy. Requires MAINNET_RPC_URL or ALCHEMY_API_KEY."
          >
            Live ingest
          </button>
        </div>
      </div>
    </Card>
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
    <label className="ctrl-stepper">
      <span className="ctrl-stepper-label">{label}</span>
      <div className="ctrl-stepper-row">
        <button
          type="button"
          className="ctrl-stepper-button"
          onClick={() => onChange(clamp(value - step))}
          aria-label={`Decrease ${label}`}
        >
          −
        </button>
        <input
          type="number"
          className="ctrl-stepper-input mono"
          value={display}
          step={step}
          onChange={(e) => {
            const parsed = parseFloat(e.target.value);
            if (Number.isFinite(parsed)) onChange(clamp(parsed));
          }}
        />
        <button
          type="button"
          className="ctrl-stepper-button"
          onClick={() => onChange(clamp(value + step))}
          aria-label={`Increase ${label}`}
        >
          +
        </button>
      </div>
    </label>
  );
}
