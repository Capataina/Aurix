import { useEffect, useState } from "react";

/**
 * State hook that mirrors a single value into localStorage.
 *
 * - Reads on mount; if the stored value cannot be parsed it falls back silently.
 * - Writes on every change; ignores quota/availability errors so the UI never
 *   breaks when storage is unavailable (private modes, embedded webviews).
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
      return JSON.parse(raw) as T;
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
