# Telemetry

## Scope / Purpose

- Cross-cutting IPC tracer that captures pipeline lifecycle events (start / step / error / completion), user clicks and change events, and per-page `lastState` snapshots, then serialises the session log to `~/Library/Logs/com.ataca.aurix/last-session.json` (cleared at each app boot).
- Replaces the older screenshot-based diagnostic loop. Designed so the implementing engineer can `jq` the JSON post-session and reconstruct what happened without needing to re-run with dev tools open.

## Boundaries / Ownership

- Owns: the in-process event buffer, the `telemetry.record(eventName, payload)` API, the persistence path resolution (`telemetry_log_path` IPC + `telemetry_persist` IPC), the per-page `lastState` snapshot mechanism (`useTelemetrySnapshot` hook).
- Does **not** own: the events themselves (every page emits its own); the rendering of recorded events (the user reads `last-session.json` directly via `jq` or similar).
- Cross-cutting (used by every page); not a feature-scoped subsystem. Captured as its own system file because (a) the TS+Rust pair is large enough (~430 lines combined) to warrant focused documentation and (b) the convention of "pipe diagnostics through telemetry, not console.log" is a project-wide rule.

## Current Implemented Reality

**Frontend** — `src/lib/telemetry.ts` (397 lines):

```ts
export const telemetry = {
  record(eventName: string, payload?: object): void { ... },
  recordError(eventName: string, error: unknown, payload?: object): void { ... },
  recordIpcStart(commandName: string, args: object): { id: string },
  recordIpcEnd(id: string, response: unknown): void,
  recordIpcError(id: string, error: unknown): void,
  flush(): Promise<void>,    // calls telemetry_persist IPC
};

export function useTelemetrySnapshot(pageName: string, state: object): void;
// React hook — saves the page's state on each render to lastState[pageName]
```

The recorder maintains an in-memory buffer of events; `flush()` invokes the `telemetry_persist` Tauri command which writes the buffer to disk. Buffer flushes happen periodically and on `beforeunload`.

**Backend** — `src-tauri/src/commands/telemetry.rs`:

```rust
#[tauri::command]
pub async fn telemetry_log_path() -> Result<String, CommandError>;
// returns the resolved path: ~/Library/Logs/com.ataca.aurix/last-session.json on macOS

#[tauri::command]
pub async fn telemetry_persist(events: Vec<TelemetryEvent>) -> Result<(), CommandError>;
// writes the buffered events to disk; appends to the current session file
```

The macOS path follows Apple's user-logs convention. The file is cleared at each app boot so the JSON always reflects the current session, never accumulated history.

**Event shape** (mirrored on both sides via `serde(rename_all = "camelCase")`):

```ts
interface TelemetryEvent {
  timestamp: number;          // unix ms
  eventName: string;          // dotted hierarchical, e.g. "lp.pipeline.chain-head-fetched"
  payload?: object;           // arbitrary JSON
  source: "click" | "change" | "ipc-start" | "ipc-end" | "ipc-error" | "lifecycle" | "snapshot";
}
```

## Key Interfaces / Data Flow

| Boundary | Direction | Shape | Notes |
|---|---|---|---|
| Page component → telemetry recorder | outbound | `telemetry.record(name, payload)` | per-page event emission |
| API client → telemetry recorder | outbound | auto-instrumented IPC call wrapping | `recordIpcStart` / `recordIpcEnd` / `recordIpcError` per Tauri invoke |
| telemetry recorder → backend | outbound (IPC) | `telemetry_persist(events)` | flush on interval + beforeunload |
| backend → disk | outbound | JSON write to `~/Library/Logs/com.ataca.aurix/last-session.json` | overwritten at each app boot |
| user → telemetry log | outbound | `cat ~/Library/Logs/com.ataca.aurix/last-session.json | jq '.'` | post-session diagnostic |

## Implemented Outputs / Artifacts

- `~/Library/Logs/com.ataca.aurix/last-session.json` per app session.
- Telemetry-emit calls scattered across the LP and arbitrage pages.
- Two Tauri commands registered in `lib.rs`: `commands::telemetry::telemetry_log_path`, `commands::telemetry::telemetry_persist`.

## Known Issues / Active Risks

- **`console.log` diagnostic lines coexist with `telemetry.record` calls** in `LpBacktestPage.tsx` (~12 lines). Recorded as cross-cutting Inconsistent Patterns finding in [audit findings](../plans/code-health-audit/cross-cutting.md).
- **No backpressure on the buffer.** A long-running session with frequent events can grow the buffer indefinitely until flush; flush is on-interval, not on-buffer-size. Not currently load-bearing (typical sessions are minutes to hours, not days), but worth noting.
- **macOS-specific log path.** `~/Library/Logs/com.ataca.aurix/` is macOS convention; Windows + Linux paths would need analogous resolution. Out of audit scope (the project is macOS-targeted today).

## Partial / In Progress

- None.

## Planned / Missing / Likely Changes

- Replace remaining `console.log` lines with `telemetry.record` calls (per the cross-cutting audit finding).
- Optional dev-mode toggle to also pipe telemetry to `console.log` for live debugging.
- Buffer size bound + backpressure, when long-running sessions become a use case.

## Durable Notes / Discarded Approaches

- **Telemetry over screenshots.** Earlier diagnostic loop relied on the user taking screenshots at problem points. The telemetry recorder gives the engineer a structured event stream that can be `jq`'d, joined, filtered — radically more useful for debugging multi-step IPC pipelines. Documented in commit 391eadd ("Replaces screenshot-based diagnostic loop").
- **Per-session file over append-only history.** Earlier consideration kept a rolling history of all sessions; cleared-each-boot keeps the file size bounded and ensures the most recent session is always at the top of the file. History across sessions can be reconstructed from git's session-summary commits when needed.

## Obsolete / No Longer Relevant

- The pre-telemetry `console.log` debugging convention.

## Cross-references

- Used by: every page (LP backtest GUI, arbitrage GUI), every Tauri command call.
- Backend pair: `src-tauri/src/commands/telemetry.rs`.
- Related convention: the cross-cutting `console.log` → telemetry replacement, recorded in [audit findings](../plans/code-health-audit/cross-cutting.md) §"Frontend console.log diagnostic lines".
