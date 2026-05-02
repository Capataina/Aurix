/**
 * Centralised number, currency, percent, and time formatters.
 * Inputs/Outputs/Errors/Side effects:
 *   - All formatters are pure and locale-fixed (en-US for currency, en-GB for dates).
 *   - Functions never throw on finite numeric input.
 */

const usdFormatter = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  maximumFractionDigits: 2,
});

const usdSignedFormatter = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  maximumFractionDigits: 2,
  signDisplay: "exceptZero",
});

const usdCompactFormatter = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  maximumFractionDigits: 4,
});

const timeFormatter = new Intl.DateTimeFormat("en-GB", {
  hour: "2-digit",
  minute: "2-digit",
  second: "2-digit",
});

const dateTimeFormatter = new Intl.DateTimeFormat("en-GB", {
  dateStyle: "medium",
  timeStyle: "medium",
});

export function formatUsd(value: number): string {
  return usdFormatter.format(value);
}

export function formatSignedUsd(value: number): string {
  return usdSignedFormatter.format(value);
}

export function formatPreciseUsd(value: number, decimals = 4): string {
  return usdCompactFormatter.format(Number(value.toFixed(decimals)));
}

export function formatPercent(value: number, decimals = 2): string {
  return `${value.toFixed(decimals)}%`;
}

export function formatSignedPercent(value: number, decimals = 3): string {
  const sign = value > 0 ? "+" : value < 0 ? "" : "";
  return `${sign}${value.toFixed(decimals)}%`;
}

export function formatTime(unixMs: number): string {
  return timeFormatter.format(unixMs);
}

export function formatTimestamp(unixMs: number): string {
  return dateTimeFormatter.format(unixMs);
}

export function formatGwei(value: number, decimals = 2): string {
  return `${value.toFixed(decimals)} gwei`;
}

/**
 * Auto-precision gwei formatter: 0 decimals for ≥10 gwei, 1 decimal for 1-10
 * gwei, 2 decimals for sub-1 gwei. Avoids the misleading "0 gwei" rounding
 * when the actual value is 0.4 gwei.
 */
export function formatGweiSmart(value: number): string {
  if (value >= 10) return `${value.toFixed(0)} gwei`;
  if (value >= 1) return `${value.toFixed(1)} gwei`;
  return `${value.toFixed(2)} gwei`;
}

export function formatRelativeTime(unixMs: number, nowMs: number): string {
  const deltaSeconds = Math.max(0, Math.floor((nowMs - unixMs) / 1000));

  if (deltaSeconds < 1) {
    return "just now";
  }

  if (deltaSeconds < 60) {
    return `${deltaSeconds}s ago`;
  }

  if (deltaSeconds < 3600) {
    return `${Math.floor(deltaSeconds / 60)}m ago`;
  }

  return `${Math.floor(deltaSeconds / 3600)}h ago`;
}
