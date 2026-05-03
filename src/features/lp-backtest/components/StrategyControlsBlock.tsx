import { Card } from "../../../components/primitives/Card";
import { Pill } from "../../../components/primitives/Pill";
import type { RebalanceRule } from "../types";

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
  busy: boolean;
  status: string;
  onRemove?: () => void;
}

const RULE_OPTIONS = [
  { id: "static", label: "Static" },
  { id: "schedule", label: "Schedule" },
  { id: "price_exit_threshold", label: "Price exit" },
  { id: "out_of_range_duration", label: "OOR duration" },
];

function ruleId(rule: RebalanceRule): string {
  return rule.kind;
}

function ruleFromId(id: string, current: RebalanceRule): RebalanceRule {
  switch (id) {
    case "static":
      return { kind: "static" };
    case "schedule":
      return {
        kind: "schedule",
        every_n_blocks:
          current.kind === "schedule" ? current.every_n_blocks : 7200,
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
          current.kind === "out_of_range_duration"
            ? current.min_oor_blocks
            : 600,
      };
    default:
      return { kind: "static" };
  }
}

export function StrategyControlsBlock({
  state,
  onChange,
  onRunBacktest,
  onRunSyntheticIngest,
  onRunLiveIngest,
  busy,
  status,
  onRemove,
}: StrategyControlsBlockProps) {
  const ruleArgs = (() => {
    if (state.rule.kind === "schedule") {
      return (
        <label className="lp-controls-arg">
          <span>Every N blocks</span>
          <input
            type="number"
            min={1}
            value={state.rule.every_n_blocks}
            onChange={(e) =>
              onChange({
                ...state,
                rule: {
                  kind: "schedule",
                  every_n_blocks: Number(e.target.value),
                },
              })
            }
          />
        </label>
      );
    }
    if (state.rule.kind === "price_exit_threshold") {
      return (
        <label className="lp-controls-arg">
          <span>Central %</span>
          <input
            type="number"
            min={0}
            max={1}
            step={0.05}
            value={state.rule.central_pct}
            onChange={(e) =>
              onChange({
                ...state,
                rule: {
                  kind: "price_exit_threshold",
                  central_pct: Number(e.target.value),
                },
              })
            }
          />
        </label>
      );
    }
    if (state.rule.kind === "out_of_range_duration") {
      return (
        <label className="lp-controls-arg">
          <span>Min OOR blocks</span>
          <input
            type="number"
            min={1}
            value={state.rule.min_oor_blocks}
            onChange={(e) =>
              onChange({
                ...state,
                rule: {
                  kind: "out_of_range_duration",
                  min_oor_blocks: Number(e.target.value),
                },
              })
            }
          />
        </label>
      );
    }
    return null;
  })();

  return (
    <Card
      title="Strategy controls"
      subtitle="Position config + rebalance rule"
      onRemove={onRemove}
    >
      <div className="lp-controls">
        <div className="lp-controls-row">
          <label className="lp-controls-arg lp-controls-pool">
            <span>Pool address</span>
            <input
              type="text"
              value={state.poolAddress}
              onChange={(e) =>
                onChange({ ...state, poolAddress: e.target.value })
              }
            />
          </label>
        </div>

        <div className="lp-controls-row">
          <label className="lp-controls-arg">
            <span>From block</span>
            <input
              type="number"
              value={state.fromBlock}
              onChange={(e) =>
                onChange({ ...state, fromBlock: Number(e.target.value) })
              }
            />
          </label>
          <label className="lp-controls-arg">
            <span>To block</span>
            <input
              type="number"
              value={state.toBlock}
              onChange={(e) =>
                onChange({ ...state, toBlock: Number(e.target.value) })
              }
            />
          </label>
          <label className="lp-controls-arg">
            <span>Tick lower</span>
            <input
              type="number"
              value={state.tickLower}
              onChange={(e) =>
                onChange({ ...state, tickLower: Number(e.target.value) })
              }
            />
          </label>
          <label className="lp-controls-arg">
            <span>Tick upper</span>
            <input
              type="number"
              value={state.tickUpper}
              onChange={(e) =>
                onChange({ ...state, tickUpper: Number(e.target.value) })
              }
            />
          </label>
        </div>

        <div className="lp-controls-row">
          <label className="lp-controls-arg">
            <span>Deposit USD</span>
            <input
              type="number"
              min={0}
              value={state.depositUsd}
              onChange={(e) =>
                onChange({ ...state, depositUsd: Number(e.target.value) })
              }
            />
          </label>
          <label className="lp-controls-arg">
            <span>Fee tier bps</span>
            <input
              type="number"
              min={1}
              value={state.feeTierBps}
              onChange={(e) =>
                onChange({ ...state, feeTierBps: Number(e.target.value) })
              }
            />
          </label>
          <label className="lp-controls-arg">
            <span>MEV haircut bps</span>
            <input
              type="number"
              min={0}
              step={0.5}
              value={state.mevHaircutBps}
              onChange={(e) =>
                onChange({ ...state, mevHaircutBps: Number(e.target.value) })
              }
            />
          </label>
        </div>

        <div className="lp-controls-row">
          <label className="lp-controls-arg lp-controls-rule">
            <span>Rebalance rule</span>
            <select
              value={ruleId(state.rule)}
              onChange={(e) =>
                onChange({
                  ...state,
                  rule: ruleFromId(e.target.value, state.rule),
                })
              }
            >
              {RULE_OPTIONS.map((option) => (
                <option key={option.id} value={option.id}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          {ruleArgs}
        </div>

        <div className="lp-controls-actions">
          <button
            type="button"
            className="lp-button is-primary"
            onClick={onRunBacktest}
            disabled={busy}
          >
            Run backtest
          </button>
          <button
            type="button"
            className="lp-button"
            onClick={onRunSyntheticIngest}
            disabled={busy}
            title="Generate sinusoidal synthetic swap stream over the block range — no API key needed."
          >
            Synthetic ingest
          </button>
          <button
            type="button"
            className="lp-button"
            onClick={onRunLiveIngest}
            disabled={busy}
            title="Live archive ingest via Alchemy. Requires MAINNET_RPC_URL or ALCHEMY_API_KEY."
          >
            Live ingest
          </button>
          {status ? <Pill tone={busy ? "neutral" : "up"}>{status}</Pill> : null}
        </div>
      </div>
    </Card>
  );
}
