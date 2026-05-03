import { Card } from "../../primitives/Card";
import { StatusGlyph } from "../../primitives/StatusGlyph";
import { formatGwei, formatRelativeTime } from "../../../lib/format";
import type { BlockRenderProps } from "./BlockRegistry";

function shortAddress(address: string | undefined): string {
  if (!address) return "—";
  if (address.length <= 14) return address;
  return `${address.slice(0, 6)}…${address.slice(-4)}`;
}

function gasLevel(gwei: number | undefined): number {
  if (gwei === undefined) return 0;
  if (gwei < 10) return 5;
  if (gwei < 25) return 4;
  if (gwei < 50) return 3;
  if (gwei < 100) return 2;
  return 1;
}

function freshnessLevel(deltaMs: number): number {
  if (deltaMs < 1500) return 5;
  if (deltaMs < 3000) return 4;
  if (deltaMs < 6000) return 3;
  if (deltaMs < 12_000) return 2;
  return 1;
}

export function ConnectionBlock({ market, onRemove }: BlockRenderProps) {
  const { overview, heroSnapshot } = market;
  const now = Date.now();
  const tickAge = overview ? now - overview.fetchedAtUnixMs : null;

  return (
    <Card title="Connection" subtitle={overview?.chain ?? "—"} onRemove={onRemove}>
      <div className="connection-quad">
        <div className="conn-row">
          <span className="conn-icon">⛓</span>
          <span className="conn-name">Chain</span>
          <span className="conn-glyph">
            <StatusGlyph level={overview ? 5 : 0} tone="up" total={5} />
          </span>
          <span className="conn-value mono">{overview?.chain.split(" ")[0] ?? "—"}</span>
        </div>
        <div className="conn-row">
          <span className="conn-icon">⛽</span>
          <span className="conn-name">Gas</span>
          <span className="conn-glyph">
            <StatusGlyph
              level={gasLevel(overview?.gasPriceGwei)}
              tone={
                overview && overview.gasPriceGwei < 30
                  ? "up"
                  : overview && overview.gasPriceGwei < 70
                    ? "warn"
                    : "down"
              }
              total={5}
            />
          </span>
          <span className="conn-value mono">
            {overview ? formatGwei(overview.gasPriceGwei, 0) : "—"}
          </span>
        </div>
        <div className="conn-row">
          <span className="conn-icon">⏱</span>
          <span className="conn-name">Tick</span>
          <span className="conn-glyph">
            <StatusGlyph
              level={tickAge !== null ? freshnessLevel(tickAge) : 0}
              tone={
                tickAge === null
                  ? "neutral"
                  : tickAge < 3000
                    ? "up"
                    : tickAge < 8000
                      ? "warn"
                      : "down"
              }
              total={5}
            />
          </span>
          <span className="conn-value mono">
            {tickAge !== null ? formatRelativeTime(now - tickAge, now) : "—"}
          </span>
        </div>
        <div className="conn-row">
          <span className="conn-icon">◎</span>
          <span className="conn-name">Pool</span>
          <span className="conn-glyph">
            <StatusGlyph level={heroSnapshot ? 5 : 0} tone="info" total={5} />
          </span>
          <span className="conn-value mono">{shortAddress(heroSnapshot?.poolAddress)}</span>
        </div>
      </div>
    </Card>
  );
}
