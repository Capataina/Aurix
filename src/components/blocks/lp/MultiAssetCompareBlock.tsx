import { Card } from "../../primitives/Card";
import { Pill } from "../../primitives/Pill";
import { Sparkline } from "../../primitives/Sparkline";
import type {
  HeadlineMonthlyRow,
  HeadlineRunSummary,
} from "../../../features/lp-backtest/types";

interface MultiAssetCompareBlockProps {
  summary: HeadlineRunSummary | null;
  monthly: HeadlineMonthlyRow[];
  busy: boolean;
  onRemove?: () => void;
}

type AssetTone = "accent" | "up" | "info" | "warn" | "down";

interface AssetDef {
  id:
    | "lpBest"
    | "lpNaive"
    | "lpMedian"
    | "sp500"
    | "gold"
    | "aave"
    | "lido"
    | "tbill"
    | "hodl";
  label: string;
  short: string;
  tone: AssetTone;
  pickReturn: (row: HeadlineMonthlyRow) => number;
}

const ASSETS: AssetDef[] = [
  {
    id: "lpBest",
    label: "V3 LP (best cell)",
    short: "LP best",
    tone: "accent",
    pickReturn: (r) => r.bestLpReturn,
  },
  {
    id: "lpMedian",
    label: "V3 LP (median cell)",
    short: "LP median",
    tone: "info",
    pickReturn: (r) => r.medianLpReturn,
  },
  {
    id: "lpNaive",
    label: "V3 LP (static, naive)",
    short: "LP static",
    tone: "info",
    pickReturn: (r) => r.naiveLpReturn,
  },
  {
    id: "sp500",
    label: "S&P 500 (VOO)",
    short: "S&P 500",
    tone: "up",
    pickReturn: (r) => r.sp500Return,
  },
  {
    id: "gold",
    label: "Gold (LBMA fix)",
    short: "Gold",
    tone: "warn",
    pickReturn: (r) => r.goldReturn,
  },
  {
    id: "aave",
    label: "Aave V3 USDC supply",
    short: "Aave USDC",
    tone: "info",
    pickReturn: (r) => r.aaveUsdcReturn,
  },
  {
    id: "lido",
    label: "Lido stETH",
    short: "Lido stETH",
    tone: "info",
    pickReturn: (r) => r.lidoStethReturn,
  },
  {
    id: "tbill",
    label: "3-month T-bill",
    short: "3-mo T-bill",
    tone: "warn",
    pickReturn: (r) => r.tbillReturn,
  },
  {
    id: "hodl",
    label: "Buy & hold",
    short: "HODL",
    tone: "info",
    pickReturn: (r) => r.hodlReturn,
  },
];

interface AssetSummary {
  def: AssetDef;
  monthly: number[];
  cumulative: number;
}

/**
 * Hero comparison block — scores LP returns against five real-world
 * benchmarks (S&P 500, Gold, Aave/Lido lending, 3-mo T-bill) plus a
 * naive HODL baseline over the headline lookback. Two tiers: the
 * cumulative-return league table on top (sorted DESC, LP highlighted),
 * and a per-benchmark win-rate strip below.
 */
export function MultiAssetCompareBlock({
  summary,
  monthly,
  busy,
  onRemove,
}: MultiAssetCompareBlockProps) {
  if (!summary || !monthly.length) {
    return (
      <Card
        title="Multi-asset comparison"
        subtitle={busy ? "synthesising…" : "LP vs S&P / Gold / Lending / T-bill"}
        onRemove={onRemove}
      >
        <div className="lp-card-empty">
          Headline analysis hasn't run yet — comparison will land here.
        </div>
      </Card>
    );
  }

  const sortedMonthly = [...monthly].sort((a, b) =>
    a.yearMonth < b.yearMonth ? -1 : a.yearMonth > b.yearMonth ? 1 : 0,
  );

  const summaries: AssetSummary[] = ASSETS.map((def) => {
    const monthlyReturns = sortedMonthly.map((r) => def.pickReturn(r));
    // Cumulative compounded return over the lookback window.
    const cumulative = monthlyReturns.reduce((acc, r) => (1 + acc) * (1 + r) - 1, 0);
    return { def, monthly: monthlyReturns, cumulative };
  });

  const sorted = [...summaries].sort((a, b) => b.cumulative - a.cumulative);
  const maxAbs = Math.max(
    1e-6,
    ...sorted.map((s) => Math.abs(s.cumulative)),
  );

  return (
    <Card
      title="Multi-asset comparison"
      subtitle={`${monthly.length}-month lookback · cumulative return · LP highlighted`}
      onRemove={onRemove}
      bodyClassName="is-flex"
    >
      <div className="cmp-stack">
        <div className="cmp-league">
          {sorted.map((s, idx) => (
            <CmpRow key={s.def.id} entry={s} maxAbs={maxAbs} rank={idx + 1} />
          ))}
        </div>

        <div className="cmp-divider" />

        <div className="cmp-winrates">
          <div className="cmp-winrates-label">LP win rate vs each benchmark</div>
          <div className="cmp-winrates-grid">
            <WinRateRow
              label="Stable lending"
              won={summary.monthsLpBeatLending}
              total={summary.monthsTotal}
              spreadPp={highVolMedianAsPp(summary)}
              spreadLabel={spreadLabel(summary, "lending")}
            />
            <WinRateRow
              label="S&P 500"
              won={summary.monthsLpBeatSp500}
              total={summary.monthsTotal}
              spreadPp={
                summary.medianSp500Spread !== null
                  ? summary.medianSp500Spread * 100
                  : null
              }
              spreadLabel="median spread"
            />
            <WinRateRow
              label="Gold"
              won={summary.monthsLpBeatGold}
              total={summary.monthsTotal}
              spreadPp={
                summary.medianGoldSpread !== null
                  ? summary.medianGoldSpread * 100
                  : null
              }
              spreadLabel="median spread"
            />
            <WinRateRow
              label="3-mo T-bill"
              won={summary.monthsLpBeatTbill}
              total={summary.monthsTotal}
              spreadPp={
                summary.medianTbillSpread !== null
                  ? summary.medianTbillSpread * 100
                  : null
              }
              spreadLabel="median spread"
            />
          </div>
        </div>
      </div>
    </Card>
  );
}

interface CmpRowProps {
  entry: AssetSummary;
  maxAbs: number;
  rank: number;
}

function CmpRow({ entry, maxAbs, rank }: CmpRowProps) {
  const { def, monthly, cumulative } = entry;
  const isLp = def.id.startsWith("lp");
  const widthPct = Math.min(100, (Math.abs(cumulative) / maxAbs) * 100);
  const sign = cumulative >= 0 ? "positive" : "negative";

  return (
    <div className={`cmp-row ${isLp ? "is-lp" : ""}`}>
      <span className="cmp-row-rank mono">#{rank}</span>
      <span className="cmp-row-label">{def.label}</span>
      <div className="cmp-row-spark">
        {monthly.length > 1 ? (
          <Sparkline values={monthly} tone={def.tone} height={20} filled={false} />
        ) : (
          <span className="cmp-row-empty mono">—</span>
        )}
      </div>
      <div className="cmp-row-bar-stage">
        <div className="cmp-row-bar-axis" />
        <div
          className={`cmp-row-bar-fill is-${sign} cmp-tone-${def.tone}`}
          style={{
            width: `${widthPct / 2}%`,
            [cumulative >= 0 ? "left" : "right"]: "50%",
          }}
        />
      </div>
      <span className={`cmp-row-value mono is-${sign}`}>
        {fmtPctSigned(cumulative)}
      </span>
    </div>
  );
}

interface WinRateRowProps {
  label: string;
  won: number;
  total: number;
  spreadPp: number | null;
  spreadLabel: string;
}

function WinRateRow({ label, won, total, spreadPp, spreadLabel }: WinRateRowProps) {
  const ratio = total > 0 ? won / total : 0;
  const tone: "up" | "neutral" | "down" =
    ratio > 0.55 ? "up" : ratio < 0.45 ? "down" : "neutral";
  // 4-step glyph filled by win rate.
  const filled = Math.round(ratio * 4);
  const glyphs = Array.from({ length: 4 }, (_, i) => (i < filled ? "●" : "○")).join("");
  return (
    <div className="cmp-winrate-row">
      <span className="cmp-winrate-label">{label}</span>
      <span className={`cmp-winrate-glyph is-${tone}`}>{glyphs}</span>
      <Pill tone={tone === "up" ? "up" : tone === "down" ? "down" : "neutral"}>
        {won}/{total}
      </Pill>
      <span className={`cmp-winrate-spread mono is-${spreadSignTone(spreadPp)}`}>
        {spreadPp !== null ? `${fmtPpSigned(spreadPp)} ${spreadLabel}` : "—"}
      </span>
    </div>
  );
}

function fmtPctSigned(v: number, decimals = 2): string {
  const pct = v * 100;
  const sign = pct >= 0 ? "+" : "";
  return `${sign}${pct.toFixed(decimals)}%`;
}

function fmtPpSigned(pp: number, decimals = 2): string {
  const sign = pp >= 0 ? "+" : "";
  return `${sign}${pp.toFixed(decimals)}pp`;
}

function spreadSignTone(pp: number | null): "up" | "down" | "neutral" {
  if (pp === null) return "neutral";
  if (pp > 0.05) return "up";
  if (pp < -0.05) return "down";
  return "neutral";
}

function highVolMedianAsPp(s: HeadlineRunSummary): number | null {
  // The "lending" comparison reuses the regime-stratified medians the
  // verdict already computes — pick the high-vol one as the headline
  // spread (LP shines most in high-vol regimes).
  if (s.medianHighVolSpread !== null) return s.medianHighVolSpread * 100;
  if (s.medianMedVolSpread !== null) return s.medianMedVolSpread * 100;
  if (s.medianLowVolSpread !== null) return s.medianLowVolSpread * 100;
  return null;
}

function spreadLabel(s: HeadlineRunSummary, _kind: "lending"): string {
  if (s.medianHighVolSpread !== null) return "median high-vol spread";
  if (s.medianMedVolSpread !== null) return "median mid-vol spread";
  if (s.medianLowVolSpread !== null) return "median low-vol spread";
  return "median spread";
}
