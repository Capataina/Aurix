// Session telemetry — lightweight diagnostic recorder.
//
// Captures user interactions, IPC calls, errors, and lifecycle events
// into an in-memory queue and periodically flushes to a single JSON
// file at ~/.aurix/last-session.json (overwriting on every flush).
// "No log accumulation" falls out naturally because the queue starts
// empty on every app boot — the first flush replaces whatever the
// previous run left behind.
//
// Public API:
//   telemetry.record(type, data?)       — log a custom event
//   telemetry.wrap(name, fn)            — wrap an async call,
//                                          records start/end/error
//   telemetry.installGlobalHandlers()    — call once on app boot to
//                                          capture clicks / errors /
//                                          lifecycle automatically.

import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface TelemetryEvent {
  /** Wall-clock ms since epoch. */
  ts: number;
  /** Ms since session start. */
  rel: number;
  /** Event family. See SECTION_TYPE constants. */
  type: string;
  /** Free-form payload — must JSON-serialise without errors. */
  data?: Record<string, unknown>;
}

interface SessionMetadata {
  bootTs: number;
  userAgent: string;
  pathname: string;
}

interface TelemetryFile {
  meta: SessionMetadata;
  /** Latest snapshot of rendered state, keyed by page/scope. Replaces
   *  the need for screenshots when diagnosing "what was on screen at
   *  the time of the bug". Updated by pages via `telemetry.snapshot`
   *  or the `useTelemetrySnapshot` hook. */
  lastState: Record<string, unknown>;
  events: TelemetryEvent[];
  /** Number of events dropped because of the in-memory cap. */
  dropped: number;
}

// Caps to keep the file small + the runtime cheap.
const MAX_EVENTS = 10_000;
const FLUSH_INTERVAL_MS = 1_000;
/** Approx upper bound for any single response payload after smart-
 *  summarisation. Arrays get truncated, strings get clipped, and the
 *  whole thing is checked one final time after JSON.stringify — past
 *  this we replace with an `_oversize` marker so one giant response
 *  can't dominate the file. */
const MAX_RESPONSE_BYTES = 8_192;
/** Long arrays inside responses are summarised to the first
 *  `ARRAY_HEAD_KEEP` + the last `ARRAY_TAIL_KEEP` elements, with a
 *  `_truncated` marker carrying the original length. */
const ARRAY_HEAD_KEEP = 3;
const ARRAY_TAIL_KEEP = 1;
const ARRAY_TRUNCATE_THRESHOLD = 20;
const SENSITIVE_KEY_RE = /(?:^|_)(?:key|secret|password|token|auth)(?:_|$)/i;

class Telemetry {
  private events: TelemetryEvent[] = [];
  private lastState: Record<string, unknown> = {};
  private dropped = 0;
  private dirty = false;
  private flushTimer: number | null = null;
  private installed = false;
  private bootTs = Date.now();
  private meta: SessionMetadata;

  constructor() {
    this.meta = {
      bootTs: this.bootTs,
      userAgent:
        typeof navigator !== "undefined" ? navigator.userAgent : "non-dom",
      pathname:
        typeof window !== "undefined" ? window.location.pathname : "/",
    };
  }

  /** Records a custom event. Common types: "boot", "shutdown",
   *  "navigate", "settings.change", "pipeline.milestone", etc. */
  record(type: string, data?: Record<string, unknown>): void {
    const event: TelemetryEvent = {
      ts: Date.now(),
      rel: Date.now() - this.bootTs,
      type,
      data: data ? this.maskSensitive(data) : undefined,
    };
    if (this.events.length >= MAX_EVENTS) {
      // Drop the oldest event (FIFO ring) so recent state stays.
      this.events.shift();
      this.dropped += 1;
    }
    this.events.push(event);
    this.dirty = true;
    this.scheduleFlush();
    if (type === "error" || type === "unhandledrejection" || type === "ipc.error") {
      void this.flushNow();
    }
  }

  /** Wraps an async call so its start/end/error are recorded.
   *  The response payload is included in `ipc.end` after smart
   *  summarisation: long arrays collapse to head+tail+count, long
   *  strings truncate at 500 chars, sensitive keys mask, and the
   *  whole thing is size-capped at MAX_RESPONSE_BYTES. */
  async wrap<T>(
    name: string,
    fn: () => Promise<T>,
    args?: Record<string, unknown>,
  ): Promise<T> {
    const startTs = Date.now();
    this.record("ipc.start", { name, args });
    try {
      const result = await fn();
      this.record("ipc.end", {
        name,
        durationMs: Date.now() - startTs,
        ok: true,
        result: this.summariseResponse(result),
      });
      return result;
    } catch (err) {
      this.record("ipc.error", {
        name,
        durationMs: Date.now() - startTs,
        error: this.errorAsObject(err),
      });
      throw err;
    }
  }

  /** Replaces a scoped slice of `lastState` with the supplied snapshot.
   *  Pages call this every render so the file always reflects what
   *  was on screen at flush time — i.e. no screenshots needed for the
   *  diagnostic loop.
   *
   *  `scope` is a stable per-page key like "lp-dashboard" or
   *  "arbitrage". Keys deep-merge so callers can update one slice
   *  without disturbing other pages' state. */
  snapshot(scope: string, state: Record<string, unknown>): void {
    this.lastState[scope] = this.maskSensitive(state);
    this.dirty = true;
    this.scheduleFlush();
  }

  /** Flushes the queue to disk immediately. Called automatically on
   *  shutdown / errors; can be called manually for testing. */
  async flushNow(): Promise<void> {
    if (!this.dirty) return;
    const file: TelemetryFile = {
      meta: this.meta,
      lastState: this.lastState,
      events: this.events,
      dropped: this.dropped,
    };
    try {
      await invoke("telemetry_persist", { json: JSON.stringify(file) });
      this.dirty = false;
    } catch (e) {
      // Don't recurse-record — would spin forever if persist itself
      // fails. Log to console only.
      // eslint-disable-next-line no-console
      console.warn("[telemetry] flush failed:", e);
    }
  }

  /** Installs DOM + window listeners for click/change/error capture +
   *  registers a beforeunload final-flush. Idempotent. */
  installGlobalHandlers(): void {
    if (this.installed) return;
    this.installed = true;
    if (typeof window === "undefined") return;

    document.addEventListener("click", this.onClick, { capture: true });
    document.addEventListener("change", this.onChange, { capture: true });
    window.addEventListener("error", this.onWindowError);
    window.addEventListener("unhandledrejection", this.onUnhandledRejection);
    window.addEventListener("beforeunload", this.onBeforeUnload);
    window.addEventListener("pagehide", this.onBeforeUnload);
  }

  // ----- DOM handlers -----------------------------------------------

  private onClick = (event: Event) => {
    const target = event.target as Element | null;
    if (!target) return;
    const meaningful = target.closest(
      "button, a, input, select, textarea, [role='button'], [data-settings-trigger], .topbar-tab, .topbar-segment, .topbar-switch, .ctrl-segment, .settings-option, .card-action",
    );
    if (!meaningful) return;
    this.record("click", {
      tag: meaningful.tagName.toLowerCase(),
      text: this.elementText(meaningful),
      ariaLabel: meaningful.getAttribute("aria-label") ?? undefined,
      classes: meaningful.className || undefined,
      datasetTrigger:
        (meaningful as HTMLElement).dataset?.settingsTrigger ?? undefined,
    });
  };

  private onChange = (event: Event) => {
    const target = event.target as HTMLInputElement | HTMLSelectElement | null;
    if (!target) return;
    const tag = target.tagName.toLowerCase();
    if (tag !== "input" && tag !== "select" && tag !== "textarea") return;
    const name =
      target.getAttribute("name") ??
      target.getAttribute("aria-label") ??
      target.getAttribute("placeholder") ??
      "(unnamed)";
    const value = SENSITIVE_KEY_RE.test(name) ? "[masked]" : target.value;
    this.record("change", {
      tag,
      type: target.getAttribute("type") ?? undefined,
      name,
      value: typeof value === "string" ? value.slice(0, 200) : value,
    });
  };

  private onWindowError = (event: ErrorEvent) => {
    this.record("error", {
      message: event.message,
      filename: event.filename,
      line: event.lineno,
      col: event.colno,
      error: event.error ? this.errorAsObject(event.error) : undefined,
    });
  };

  private onUnhandledRejection = (event: PromiseRejectionEvent) => {
    this.record("unhandledrejection", {
      reason: this.errorAsObject(event.reason),
    });
  };

  private onBeforeUnload = () => {
    this.record("shutdown");
    // Best-effort sync flush. Tauri's invoke is async; the OS may
    // or may not let it complete during unload. Either way, the most
    // recent flush from the periodic timer covers the tail.
    void this.flushNow();
  };

  // ----- Helpers ----------------------------------------------------

  private scheduleFlush(): void {
    if (this.flushTimer !== null) return;
    if (typeof window === "undefined") return;
    this.flushTimer = window.setTimeout(() => {
      this.flushTimer = null;
      void this.flushNow();
    }, FLUSH_INTERVAL_MS);
  }

  private elementText(el: Element): string | undefined {
    const text = (el.textContent ?? "").trim();
    if (!text) return undefined;
    return text.slice(0, 80);
  }

  private maskSensitive(
    obj: Record<string, unknown>,
  ): Record<string, unknown> {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(obj)) {
      if (SENSITIVE_KEY_RE.test(k)) {
        out[k] = "[masked]";
      } else {
        out[k] = this.summariseValue(v);
      }
    }
    return out;
  }

  /** Walks a value tree, applying: array head/tail truncation when
   *  longer than ARRAY_TRUNCATE_THRESHOLD, string truncation past 500
   *  chars, sensitive-key masking, and recursive descent into objects.
   *  Doesn't enforce the byte budget; that's `summariseResponse`'s job. */
  private summariseValue(value: unknown): unknown {
    if (value === null || value === undefined) return value;
    if (typeof value === "string") {
      return value.length > 500
        ? `${value.slice(0, 500)}…(+${value.length - 500} chars)`
        : value;
    }
    if (typeof value !== "object") return value;
    if (Array.isArray(value)) {
      if (value.length <= ARRAY_TRUNCATE_THRESHOLD) {
        return value.map((item) => this.summariseValue(item));
      }
      const head = value
        .slice(0, ARRAY_HEAD_KEEP)
        .map((item) => this.summariseValue(item));
      const tail = value
        .slice(value.length - ARRAY_TAIL_KEEP)
        .map((item) => this.summariseValue(item));
      return {
        _truncated: true,
        _length: value.length,
        head,
        tail,
      };
    }
    return this.maskSensitive(value as Record<string, unknown>);
  }

  /** Top-level wrapper around `summariseValue` that also enforces the
   *  byte budget. If a response is structurally fine but still too
   *  large (e.g. a 4000-element array of small objects), we replace
   *  with an `_oversize` marker carrying the original byte size. */
  private summariseResponse(value: unknown): unknown {
    const summarised = this.summariseValue(value);
    let serialised: string;
    try {
      serialised = JSON.stringify(summarised);
    } catch {
      return { _unserialisable: true };
    }
    if (serialised.length > MAX_RESPONSE_BYTES) {
      return {
        _oversize: true,
        _bytes: serialised.length,
        _preview: serialised.slice(0, MAX_RESPONSE_BYTES),
      };
    }
    return summarised;
  }

  private errorAsObject(err: unknown): Record<string, unknown> {
    if (err === null || err === undefined) return { value: String(err) };
    if (err instanceof Error) {
      return {
        name: err.name,
        message: err.message,
        stack: err.stack?.split("\n").slice(0, 6).join("\n"),
      };
    }
    if (typeof err === "object") {
      try {
        return JSON.parse(JSON.stringify(err)) as Record<string, unknown>;
      } catch {
        return { value: String(err) };
      }
    }
    return { value: String(err) };
  }
}

export const telemetry = new Telemetry();

/**
 * Wrapped Tauri invoke that records start/end/error for every call.
 * Use this in IPC client modules instead of importing `invoke` directly.
 */
export function loggedInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  return telemetry.wrap(command, () => invoke<T>(command, args), args);
}

/**
 * React hook — drops the supplied snapshot into `lastState[scope]` on
 * every change. The snapshot's identity is compared by JSON-stringify
 * so an object whose contents are equal across renders only triggers
 * one telemetry write.
 *
 * Example:
 *   useTelemetrySnapshot("lp-dashboard", {
 *     busy, status, summaryStats, curveCount, headline, ...
 *   });
 *
 * The result is that `cat last-session.json | jq .lastState["lp-dashboard"]`
 * always reflects the latest rendered state — i.e. the file replaces
 * a screenshot for diagnostic purposes.
 */
export function useTelemetrySnapshot(
  scope: string,
  state: Record<string, unknown>,
): void {
  const lastJson = useRef<string>("");
  useEffect(() => {
    const next = JSON.stringify(state);
    if (next !== lastJson.current) {
      lastJson.current = next;
      telemetry.snapshot(scope, state);
    }
  });
}
