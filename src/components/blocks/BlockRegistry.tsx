import type { ComponentType, SVGProps } from "react";

import {
  ChartIcon,
  FlameIcon,
  GridIcon,
  LinkIcon,
  NewsIcon,
  PriceTagIcon,
  PulseIcon,
  StatsIcon,
} from "../primitives/Icon";
import type { MarketState } from "../../hooks/useMarketData";
import type { PnlMode } from "../../lib/arbitrage";

import { ArbitrageMatrixBlock } from "./ArbitrageMatrixBlock";
import { ArbRouteBlock } from "./ArbRouteBlock";
import { ConnectionBlock } from "./ConnectionBlock";
import { EventLogBlock } from "./EventLogBlock";
import { GasOpportunityBlock } from "./GasOpportunityBlock";
import { HeroPriceBlock } from "./HeroPriceBlock";
import { InsightsBlock } from "./InsightsBlock";
import { MomentumBlock } from "./MomentumBlock";
import { PriceChartBlock } from "./PriceChartBlock";
import { PriceLadderBlock } from "./PriceLadderBlock";
import { QuickStatsBlock } from "./QuickStatsBlock";
import { SpreadTrackerBlock } from "./SpreadTrackerBlock";
import { VenueGridBlock } from "./VenueGridBlock";
import { VenueHeatmapBlock } from "./VenueHeatmapBlock";
import { VolatilityBlock } from "./VolatilityBlock";

export interface BlockRenderProps {
  market: MarketState;
  pnlMode: PnlMode;
  onRemove: () => void;
}

export interface BlockDefinition {
  id: string;
  title: string;
  description: string;
  Icon: ComponentType<SVGProps<SVGSVGElement>>;
  Component: ComponentType<BlockRenderProps>;
  /** Default width in grid columns (12-col `lg`). */
  defaultW: number;
  /** Default height in row units. */
  defaultH: number;
  minW: number;
  minH: number;
  maxW?: number;
  maxH?: number;
}

export const BLOCK_REGISTRY: Record<string, BlockDefinition> = {
  "hero-price": {
    id: "hero-price",
    title: "Hero price",
    description: "Headline price + change pill + live sparkline.",
    Icon: PriceTagIcon,
    Component: HeroPriceBlock,
    defaultW: 3,
    defaultH: 3,
    minW: 3,
    minH: 3,
  },
  "arb-route": {
    id: "arb-route",
    title: "Best route",
    description: "BUY at cheapest, SELL at richest — full breakdown of spread − gas.",
    Icon: PriceTagIcon,
    Component: ArbRouteBlock,
    defaultW: 5,
    defaultH: 4,
    minW: 4,
    minH: 4,
  },
  "venue-grid": {
    id: "venue-grid",
    title: "Venue grid",
    description: "All venues with deviation bars; cheapest/richest highlighted.",
    Icon: GridIcon,
    Component: VenueGridBlock,
    defaultW: 4,
    defaultH: 5,
    minW: 3,
    minH: 3,
  },
  "price-chart": {
    id: "price-chart",
    title: "Price chart",
    description: "Raw / deviation / spread / gas-adjusted modes over the rolling window.",
    Icon: ChartIcon,
    Component: PriceChartBlock,
    defaultW: 8,
    defaultH: 5,
    minW: 4,
    minH: 4,
  },
  "price-ladder": {
    id: "price-ladder",
    title: "Price ladder",
    description: "Vertical scale showing each venue's price + median.",
    Icon: StatsIcon,
    Component: PriceLadderBlock,
    defaultW: 3,
    defaultH: 3,
    minW: 3,
    minH: 3,
  },
  insights: {
    id: "insights",
    title: "Signals",
    description: "Auto-derived market interpretation as a stoplight panel.",
    Icon: NewsIcon,
    Component: InsightsBlock,
    defaultW: 4,
    defaultH: 5,
    minW: 3,
    minH: 4,
  },
  "spread-tracker": {
    id: "spread-tracker",
    title: "Spread",
    description: "Current cross-venue spread with distribution histogram.",
    Icon: PulseIcon,
    Component: SpreadTrackerBlock,
    defaultW: 3,
    defaultH: 3,
    minW: 3,
    minH: 3,
  },
  "gas-opportunity": {
    id: "gas-opportunity",
    title: "Gas-adjusted",
    description: "Net spread after gas + profitable-tick ratio.",
    Icon: FlameIcon,
    Component: GasOpportunityBlock,
    defaultW: 3,
    defaultH: 3,
    minW: 3,
    minH: 3,
  },
  momentum: {
    id: "momentum",
    title: "Momentum",
    description: "Short-window % change with directional indicator.",
    Icon: PulseIcon,
    Component: MomentumBlock,
    defaultW: 3,
    defaultH: 3,
    minW: 3,
    minH: 3,
  },
  volatility: {
    id: "volatility",
    title: "Volatility",
    description: "Rolling realised volatility (σ of returns) + trend.",
    Icon: PulseIcon,
    Component: VolatilityBlock,
    defaultW: 3,
    defaultH: 3,
    minW: 3,
    minH: 3,
  },
  "arbitrage-matrix": {
    id: "arbitrage-matrix",
    title: "Arb matrix",
    description: "NxN buy-at / sell-at gas-adjusted P/L heatmap.",
    Icon: GridIcon,
    Component: ArbitrageMatrixBlock,
    defaultW: 4,
    defaultH: 3,
    minW: 3,
    minH: 3,
  },
  "venue-heatmap": {
    id: "venue-heatmap",
    title: "Heatmap",
    description: "Per-venue per-tick deviation over recent history.",
    Icon: GridIcon,
    Component: VenueHeatmapBlock,
    defaultW: 6,
    defaultH: 3,
    minW: 4,
    minH: 3,
  },
  "event-log": {
    id: "event-log",
    title: "Events",
    description: "Vertical timeline of venue ranking flips and threshold crossings.",
    Icon: NewsIcon,
    Component: EventLogBlock,
    defaultW: 4,
    defaultH: 4,
    minW: 3,
    minH: 3,
  },
  "quick-stats": {
    id: "quick-stats",
    title: "Stats",
    description: "Median, range, std%, and sample-window completion.",
    Icon: StatsIcon,
    Component: QuickStatsBlock,
    defaultW: 3,
    defaultH: 3,
    minW: 3,
    minH: 3,
  },
  connection: {
    id: "connection",
    title: "Connection",
    description: "Chain, gas, tick freshness, hero pool — all as glyphs.",
    Icon: LinkIcon,
    Component: ConnectionBlock,
    defaultW: 3,
    defaultH: 3,
    minW: 3,
    minH: 3,
  },
};

export const ALL_BLOCKS: BlockDefinition[] = Object.values(BLOCK_REGISTRY);

export function getBlockDefinition(blockId: string): BlockDefinition | undefined {
  return BLOCK_REGISTRY[blockId];
}
