import { Card } from "../../primitives/Card";
import { Sparkline } from "../../primitives/Sparkline";
import { StatusGlyph } from "../../primitives/StatusGlyph";
import type { BenchmarkPoint } from "../../../features/lp-backtest/types";

interface BenchmarkCacheBlockProps {
  series: Record<string, BenchmarkPoint[]>;
  onFetch: () => void;
  busy: boolean;
  onRemove?: () => void;
}

const KNOWN_SERIES: Array<{ key: string; label: string; tone: "accent" | "info" | "up" | "warn" }> = [
  { key: "aave_v3_usdc_supply_apy", label: "Aave V3 USDC", tone: "accent" },
  { key: "lido_steth_apy", label: "Lido stETH", tone: "info" },
  { key: "fred_dgs3mo", label: "3-mo T-bill", tone: "warn" },
  { key: "stooq_voo", label: "S&P 500 (VOO)", tone: "up" },
  { key: "fred_gold_lbma", label: "Gold (LBMA)", tone: "warn" },
];

export function BenchmarkCacheBlock({
  series,
  onFetch,
  busy,
  onRemove,
}: BenchmarkCacheBlockProps) {
  const totalCached = Object.values(series).reduce((sum, arr) => sum + arr.length, 0);
  const cachedCount = KNOWN_SERIES.filter(
    (s) => (series[s.key]?.length ?? 0) > 0,
  ).length;

  return (
    <Card
      title="Benchmarks"
      subtitle={`${cachedCount} of ${KNOWN_SERIES.length} cached`}
      onRemove={onRemove}
      headerExtra={<StatusGlyph level={cachedCount} tone="info" total={KNOWN_SERIES.length} />}
    >
      <div className="bench-stack">
        <div className="bench-rows">
          {KNOWN_SERIES.map((s) => {
            const values = (series[s.key] ?? []).map((p) => p.value);
            const present = values.length > 0;
            const last = present ? values[values.length - 1] : null;
            return (
              <div key={s.key} className={`bench-row ${present ? "is-present" : "is-empty"}`}>
                <span className="bench-row-label">{s.label}</span>
                <div className="bench-row-spark">
                  {values.length > 1 ? (
                    <Sparkline values={values} tone={s.tone} filled height={18} />
                  ) : (
                    <div className="bench-row-empty">—</div>
                  )}
                </div>
                <span className="bench-row-last mono">
                  {last !== null ? `${last.toFixed(2)}%` : "—"}
                </span>
              </div>
            );
          })}
        </div>
        <div className="bench-footer">
          <span className="bench-footer-meta mono">{totalCached} pts cached</span>
          <button
            type="button"
            className="ctrl-button is-ghost"
            onClick={onFetch}
            disabled={busy}
          >
            {busy ? "Fetching…" : "Fetch live"}
          </button>
        </div>
      </div>
    </Card>
  );
}
