import { useEffect, useState } from "react";

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return (
    typeof value === "object" &&
    value !== null &&
    !Array.isArray(value) &&
    Object.getPrototypeOf(value) === Object.prototype
  );
}

/**
 * State hook that mirrors a single value into localStorage.
 *
 * - Reads on mount; if the stored value cannot be parsed it falls back silently.
 * - When both the stored value and `initialValue` are plain objects, the
 *   stored value is shallow-merged over `initialValue`. New fields added
 *   to a settings shape automatically pick up their defaults on next load,
 *   instead of leaving `undefined` holes that would crash form components
 *   reading them. Closes the bug class where adding a field (e.g. `chainId`
 *   to `LpSettings`) crashes any user whose localStorage was written
 *   before the field existed.
 * - Writes on every change; ignores quota/availability errors so the UI
 *   never breaks when storage is unavailable.
 */
export function usePersistedState<T>(
  key: string,
  initialValue: T,
): [T, (next: T | ((prev: T) => T)) => void] {
  const [value, setValue] = useState<T>(() => {
    if (typeof window === "undefined") {
      return initialValue;
    }

    try {
      const raw = window.localStorage.getItem(key);
      if (raw === null) {
        return initialValue;
      }
      const parsed = JSON.parse(raw) as unknown;
      if (isPlainObject(parsed) && isPlainObject(initialValue)) {
        return { ...initialValue, ...parsed } as T;
      }
      return parsed as T;
    } catch {
      return initialValue;
    }
  });

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }

    try {
      window.localStorage.setItem(key, JSON.stringify(value));
    } catch {
      /* ignore quota / private mode errors */
    }
  }, [key, value]);

  return [value, setValue];
}
