interface HeatmapProps {
  /** 2D array of cell values. Rows × cols. */
  data: number[][];
  /** Range used for normalisation (signed: red < 0 < green). */
  symmetricRange?: number;
  /** Per-row label rendered to the left. */
  rowLabels?: string[];
  /** Per-column label rendered below; rendered every Nth col to fit. */
  colLabels?: string[];
  cellGap?: number;
  height?: number;
}

/**
 * Signed-diverging heatmap: blue/cyan for positive, red for negative,
 * neutral grey at zero. Row and column labels rendered as monospace text.
 *
 * Used by VenueHeatmapBlock (venues × time deviation) and
 * ArbitrageMatrixBlock (buy×sell gas-adjusted P/L).
 */
export function Heatmap({
  data,
  symmetricRange,
  rowLabels,
  colLabels,
  cellGap = 1,
  height,
}: HeatmapProps) {
  if (data.length === 0 || data[0].length === 0) {
    return <div style={{ height }} />;
  }

  const rows = data.length;
  const cols = data[0].length;
  const flat = data.flat();
  const fallbackRange = Math.max(
    ...flat.map((value) => Math.abs(value)),
    0.0001,
  );
  const range = symmetricRange ?? fallbackRange;

  const cellColor = (value: number): string => {
    if (!Number.isFinite(value)) return "rgba(255,255,255,0.04)";
    const ratio = Math.max(-1, Math.min(1, value / range));
    if (Math.abs(ratio) < 0.02) {
      return "rgba(255,255,255,0.06)";
    }
    if (ratio > 0) {
      // Cyan / green for positive
      const alpha = 0.18 + ratio * 0.62;
      return `rgba(125, 211, 252, ${alpha})`;
    }
    const alpha = 0.18 + Math.abs(ratio) * 0.62;
    return `rgba(248, 113, 113, ${alpha})`;
  };

  const showRowLabels = !!rowLabels && rowLabels.length === rows;
  const showColLabels = !!colLabels && colLabels.length === cols;

  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: showRowLabels ? "auto 1fr" : "1fr",
        gridTemplateRows: showColLabels ? "1fr auto" : "1fr",
        gap: 4,
        width: "100%",
        height: height ?? "100%",
        minHeight: 0,
      }}
    >
      {showRowLabels ? (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: cellGap,
            justifyContent: "stretch",
          }}
        >
          {rowLabels.map((label, idx) => (
            <div
              key={idx}
              style={{
                flex: 1,
                display: "flex",
                alignItems: "center",
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                color: "var(--text-muted)",
                paddingRight: 6,
                whiteSpace: "nowrap",
              }}
            >
              {label}
            </div>
          ))}
        </div>
      ) : null}

      <div
        style={{
          display: "grid",
          gridTemplateColumns: `repeat(${cols}, 1fr)`,
          gridTemplateRows: `repeat(${rows}, 1fr)`,
          gap: cellGap,
          minHeight: 0,
        }}
      >
        {data.flatMap((row, rIdx) =>
          row.map((value, cIdx) => (
            <div
              key={`${rIdx}-${cIdx}`}
              title={`${rowLabels?.[rIdx] ?? rIdx} × ${colLabels?.[cIdx] ?? cIdx}: ${value.toFixed(4)}`}
              style={{
                background: cellColor(value),
                borderRadius: 2,
                transition: "background 200ms var(--ease-out)",
              }}
            />
          )),
        )}
      </div>

      {showColLabels ? (
        <>
          {showRowLabels ? <div /> : null}
          <div
            style={{
              display: "grid",
              gridTemplateColumns: `repeat(${cols}, 1fr)`,
              gap: cellGap,
            }}
          >
            {colLabels.map((label, idx) => (
              <div
                key={idx}
                style={{
                  textAlign: "center",
                  fontFamily: "var(--font-mono)",
                  fontSize: 10,
                  color: "var(--text-muted)",
                  whiteSpace: "nowrap",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                }}
              >
                {label}
              </div>
            ))}
          </div>
        </>
      ) : null}
    </div>
  );
}
