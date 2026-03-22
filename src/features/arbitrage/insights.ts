import type { MarketOverview, PriceSnapshot } from "./types";

const GAS_UNITS_ESTIMATE = 220_000;
const SHORT_WINDOW = 5;
const BASELINE_WINDOW = 20;
const PERSISTENCE_WINDOW = 4;
const EVENT_LIMIT = 4;

export type InsightSeverity = "info" | "watch" | "notable" | "actionable";

export interface InsightCard {
  id: string;
  title: string;
  body: string;
  severity: InsightSeverity;
  metric?: string;
}

export interface InsightEvent {
  id: string;
  summary: string;
  severity: InsightSeverity;
  timestampLabel: string;
}

export interface InsightsViewModel {
  primary: InsightCard;
  secondary: InsightCard[];
  events: InsightEvent[];
}

interface DerivedSample {
  overview: MarketOverview;
  medianPrice: number;
  spreadUsd: number;
  gasCostUsd: number;
  gasAdjustedUsd: number;
  richestVenue: PriceSnapshot;
  cheapestVenue: PriceSnapshot;
  strongestPositiveDeviation: {
    venue: PriceSnapshot;
    deviationPct: number;
  };
  strongestNegativeDeviation: {
    venue: PriceSnapshot;
    deviationPct: number;
  };
}

/**
 * Converts the current in-session history into a stable set of user-facing insights.
 */
export function deriveInsightsView(history: MarketOverview[]): InsightsViewModel {
  const derivedHistory = history.map(deriveSample);
  const latest = derivedHistory[derivedHistory.length - 1];
  const previous = derivedHistory[derivedHistory.length - 2] ?? null;
  const baselineSlice = derivedHistory.slice(-Math.min(derivedHistory.length, BASELINE_WINDOW));
  const averageSpread = average(baselineSlice.map((sample) => sample.spreadUsd));
  const averageNet = average(baselineSlice.map((sample) => sample.gasAdjustedUsd));
  const recentWindow = derivedHistory.slice(-Math.min(derivedHistory.length, SHORT_WINDOW));

  const positiveRunLength = trailingRunLength(
    derivedHistory,
    (sample) => sample.gasAdjustedUsd > 0,
  );
  const richestRunLength = trailingRunLength(
    derivedHistory,
    (sample) => sample.richestVenue.dexName === latest.richestVenue.dexName,
  );
  const cheapestRunLength = trailingRunLength(
    derivedHistory,
    (sample) => sample.cheapestVenue.dexName === latest.cheapestVenue.dexName,
  );
  const elevatedSpreadRunLength = trailingRunLength(
    derivedHistory,
    (sample) => sample.spreadUsd >= averageSpread * 1.15,
  );

  const primary = buildPrimaryInsight(
    latest,
    previous,
    averageSpread,
    positiveRunLength,
    elevatedSpreadRunLength,
  );

  const secondary: InsightCard[] = [
    {
      id: "ranking",
      title: "Venue order",
      severity: latest.spreadUsd >= averageSpread * 1.15 ? "notable" : "info",
      metric: formatUsd(latest.spreadUsd),
      body:
        `${latest.richestVenue.dexName} is currently richest and ` +
        `${latest.cheapestVenue.dexName} is cheapest, leaving a ${formatUsd(latest.spreadUsd)} ` +
        `cross-venue spread.`,
    },
    {
      id: "spread-trend",
      title: "Spread regime",
      severity:
        elevatedSpreadRunLength >= PERSISTENCE_WINDOW
          ? "watch"
          : latest.spreadUsd > averageSpread
            ? "notable"
            : "info",
      metric: describeSpreadTrend(latest, previous, averageSpread),
      body: buildSpreadBody(latest, previous, averageSpread, elevatedSpreadRunLength),
    },
    {
      id: "deviation",
      title: "Deviation leader",
      severity:
        Math.abs(latest.strongestPositiveDeviation.deviationPct) >= 0.12 ||
        Math.abs(latest.strongestNegativeDeviation.deviationPct) >= 0.12
          ? "watch"
          : "info",
      metric: formatSignedPercent(
        largerMagnitude(
          latest.strongestPositiveDeviation.deviationPct,
          latest.strongestNegativeDeviation.deviationPct,
        ),
      ),
      body:
        `${latest.strongestPositiveDeviation.venue.dexName} is ${formatSignedPercent(latest.strongestPositiveDeviation.deviationPct)} ` +
        `above the median while ${latest.strongestNegativeDeviation.venue.dexName} is ` +
        `${formatSignedPercent(latest.strongestNegativeDeviation.deviationPct)} below it.`,
    },
    {
      id: "actionability",
      title: "Gas-adjusted view",
      severity:
        positiveRunLength >= PERSISTENCE_WINDOW
          ? "actionable"
          : latest.gasAdjustedUsd > 0
            ? "watch"
            : "info",
      metric: formatUsd(latest.gasAdjustedUsd),
      body: buildActionabilityBody(
        latest,
        previous,
        positiveRunLength,
        averageNet,
        recentWindow,
        cheapestRunLength,
        richestRunLength,
      ),
    },
  ];

  if (history.length < SHORT_WINDOW) {
    secondary.push({
      id: "caution",
      title: "Signal maturity",
      severity: "info",
      metric: `${history.length} samples`,
      body:
        "Session history is still shallow, so sudden moves should be treated as early signals rather than established patterns.",
    });
  }

  return {
    primary,
    secondary: secondary.slice(0, 4),
    events: buildInsightEvents(derivedHistory, averageSpread),
  };
}

function buildPrimaryInsight(
  latest: DerivedSample,
  previous: DerivedSample | null,
  averageSpread: number,
  positiveRunLength: number,
  elevatedSpreadRunLength: number,
): InsightCard {
  if (positiveRunLength >= PERSISTENCE_WINDOW) {
    return {
      id: "primary-actionable",
      title: "Positive setup holding",
      severity: "actionable",
      metric: formatUsd(latest.gasAdjustedUsd),
      body:
        `The ${latest.cheapestVenue.dexName} to ${latest.richestVenue.dexName} route has stayed positive ` +
        `for ${positiveRunLength} samples, with gas still leaving an estimated ${formatUsd(latest.gasAdjustedUsd)} of room.`,
    };
  }

  if (latest.gasAdjustedUsd > 0) {
    return {
      id: "primary-watch-positive",
      title: "Positive setup emerging",
      severity: "watch",
      metric: formatUsd(latest.gasAdjustedUsd),
      body:
        `The visible spread currently clears the gas estimate. ${latest.cheapestVenue.dexName} remains the cheapest venue and ` +
        `${latest.richestVenue.dexName} remains the richest, so this is worth watching for persistence.`,
    };
  }

  if (elevatedSpreadRunLength >= PERSISTENCE_WINDOW) {
    return {
      id: "primary-elevated-spread",
      title: "Spread staying elevated",
      severity: "notable",
      metric: formatUsd(latest.spreadUsd),
      body:
        `Venue disagreement has held above the recent session baseline for ${elevatedSpreadRunLength} samples, but gas still absorbs the visible edge.`,
    };
  }

  return {
    id: "primary-state",
    title: "Market read",
    severity: latest.spreadUsd > averageSpread ? "notable" : "info",
    metric: describeSpreadTrend(latest, previous, averageSpread),
    body:
      `${latest.richestVenue.dexName} is leading while ${latest.cheapestVenue.dexName} is lagging. ` +
      `The current spread is ${formatUsd(latest.spreadUsd)} and the gas-adjusted estimate sits at ${formatUsd(latest.gasAdjustedUsd)}.`,
  };
}

function buildSpreadBody(
  latest: DerivedSample,
  previous: DerivedSample | null,
  averageSpread: number,
  elevatedSpreadRunLength: number,
): string {
  if (elevatedSpreadRunLength >= PERSISTENCE_WINDOW) {
    return (
      `Spread has stayed above the recent baseline for ${elevatedSpreadRunLength} samples. ` +
      `This looks more like a real dispersion regime than a one-tick jump.`
    );
  }

  if (!previous) {
    return "Baseline formation is still in progress, so the current spread should be read as an opening state rather than a settled regime.";
  }

  const delta = latest.spreadUsd - previous.spreadUsd;
  if (Math.abs(delta) < 0.01) {
    return "Spread is effectively flat versus the previous sample, so venue ordering matters more than immediate momentum.";
  }

  return delta > 0
    ? `Spread widened by ${formatUsd(delta)} versus the last sample and now sits ${relativeDirection(latest.spreadUsd, averageSpread)} the recent baseline.`
    : `Spread narrowed by ${formatUsd(Math.abs(delta))} versus the last sample and now sits ${relativeDirection(latest.spreadUsd, averageSpread)} the recent baseline.`;
}

function buildActionabilityBody(
  latest: DerivedSample,
  previous: DerivedSample | null,
  positiveRunLength: number,
  averageNet: number,
  recentWindow: DerivedSample[],
  cheapestRunLength: number,
  richestRunLength: number,
): string {
  const improvement = previous ? latest.gasAdjustedUsd - previous.gasAdjustedUsd : 0;
  const recentSlope = recentWindow[recentWindow.length - 1].gasAdjustedUsd - recentWindow[0].gasAdjustedUsd;

  if (positiveRunLength >= PERSISTENCE_WINDOW) {
    return (
      `Gas still leaves a positive estimate, and the route has held for ${positiveRunLength} samples. ` +
      `${latest.cheapestVenue.dexName} has stayed cheapest for ${cheapestRunLength} samples while ${latest.richestVenue.dexName} has stayed richest for ${richestRunLength}.`
    );
  }

  if (latest.gasAdjustedUsd > 0) {
    return (
      `The spread currently clears gas by ${formatUsd(latest.gasAdjustedUsd)}. ` +
      `${trendWord(recentSlope)} over the recent window, but it still needs persistence before it looks stable.`
    );
  }

  const erosion = Math.abs(latest.gasAdjustedUsd);
  const deltaText =
    previous && Math.abs(improvement) >= 0.01
      ? ` It ${improvement > 0 ? "improved" : "deteriorated"} by ${formatUsd(Math.abs(improvement))} versus the last sample.`
      : "";

  return (
    `Gas is still erasing the visible spread by ${formatUsd(erosion)}. ` +
    `The estimate remains ${relativeDirection(latest.gasAdjustedUsd, averageNet)} the recent net baseline.` +
    deltaText
  );
}

function buildInsightEvents(
  derivedHistory: DerivedSample[],
  averageSpread: number,
): InsightEvent[] {
  const events: InsightEvent[] = [];

  for (let index = 1; index < derivedHistory.length; index += 1) {
    const previous = derivedHistory[index - 1];
    const current = derivedHistory[index];

    if (previous.richestVenue.dexName !== current.richestVenue.dexName) {
      events.push({
        id: `richest-${current.overview.fetchedAtUnixMs}`,
        severity: "notable",
        timestampLabel: formatTime(current.overview.fetchedAtUnixMs),
        summary: `${current.richestVenue.dexName} became the richest venue.`,
      });
    }

    if (previous.cheapestVenue.dexName !== current.cheapestVenue.dexName) {
      events.push({
        id: `cheapest-${current.overview.fetchedAtUnixMs}`,
        severity: "notable",
        timestampLabel: formatTime(current.overview.fetchedAtUnixMs),
        summary: `${current.cheapestVenue.dexName} became the cheapest venue.`,
      });
    }

    if (previous.gasAdjustedUsd <= 0 && current.gasAdjustedUsd > 0) {
      events.push({
        id: `net-positive-${current.overview.fetchedAtUnixMs}`,
        severity: "actionable",
        timestampLabel: formatTime(current.overview.fetchedAtUnixMs),
        summary: `Gas-adjusted estimate turned positive at ${formatUsd(current.gasAdjustedUsd)}.`,
      });
    }

    if (previous.gasAdjustedUsd > 0 && current.gasAdjustedUsd <= 0) {
      events.push({
        id: `net-negative-${current.overview.fetchedAtUnixMs}`,
        severity: "watch",
        timestampLabel: formatTime(current.overview.fetchedAtUnixMs),
        summary: "Gas-adjusted estimate fell back below zero.",
      });
    }

    if (
      previous.spreadUsd < averageSpread * 1.15 &&
      current.spreadUsd >= averageSpread * 1.15
    ) {
      events.push({
        id: `spread-elevated-${current.overview.fetchedAtUnixMs}`,
        severity: "watch",
        timestampLabel: formatTime(current.overview.fetchedAtUnixMs),
        summary: `Venue spread moved above the recent session baseline at ${formatUsd(current.spreadUsd)}.`,
      });
    }
  }

  return events.slice(-EVENT_LIMIT).reverse();
}

function deriveSample(overview: MarketOverview): DerivedSample {
  const sortedByPrice = [...overview.venues].sort((left, right) => left.priceUsd - right.priceUsd);
  const medianPrice = median(overview.venues.map((venue) => venue.priceUsd));
  const cheapestVenue = sortedByPrice[0];
  const richestVenue = sortedByPrice[sortedByPrice.length - 1];
  const spreadUsd = richestVenue.priceUsd - cheapestVenue.priceUsd;
  const gasCostUsd = (overview.gasPriceGwei * GAS_UNITS_ESTIMATE * medianPrice) / 1_000_000_000;
  const gasAdjustedUsd = spreadUsd - gasCostUsd;
  const deviationPairs = overview.venues.map((venue) => ({
    venue,
    deviationPct: ((venue.priceUsd - medianPrice) / medianPrice) * 100,
  }));
  const strongestPositiveDeviation = [...deviationPairs].sort(
    (left, right) => right.deviationPct - left.deviationPct,
  )[0];
  const strongestNegativeDeviation = [...deviationPairs].sort(
    (left, right) => left.deviationPct - right.deviationPct,
  )[0];

  return {
    overview,
    medianPrice,
    spreadUsd,
    gasCostUsd,
    gasAdjustedUsd,
    richestVenue,
    cheapestVenue,
    strongestPositiveDeviation,
    strongestNegativeDeviation,
  };
}

function median(values: number[]): number {
  const sortedValues = [...values].sort((left, right) => left - right);
  const midpoint = Math.floor(sortedValues.length / 2);

  if (sortedValues.length % 2 === 0) {
    return (sortedValues[midpoint - 1] + sortedValues[midpoint]) / 2;
  }

  return sortedValues[midpoint];
}

function average(values: number[]): number {
  if (values.length === 0) {
    return 0;
  }

  return values.reduce((sum, value) => sum + value, 0) / values.length;
}

function trailingRunLength<T>(items: T[], predicate: (item: T) => boolean): number {
  let count = 0;

  for (let index = items.length - 1; index >= 0; index -= 1) {
    if (!predicate(items[index])) {
      break;
    }

    count += 1;
  }

  return count;
}

function formatUsd(value: number): string {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    maximumFractionDigits: 2,
    signDisplay: "exceptZero",
  }).format(value);
}

function formatSignedPercent(value: number): string {
  return `${value >= 0 ? "+" : ""}${value.toFixed(3)}%`;
}

function formatTime(unixMs: number): string {
  return new Intl.DateTimeFormat("en-GB", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(unixMs);
}

function relativeDirection(value: number, baseline: number): string {
  if (value > baseline * 1.05) {
    return "above";
  }

  if (value < baseline * 0.95) {
    return "below";
  }

  return "near";
}

function describeSpreadTrend(
  latest: DerivedSample,
  previous: DerivedSample | null,
  averageSpread: number,
): string {
  if (!previous) {
    return "Opening read";
  }

  const delta = latest.spreadUsd - previous.spreadUsd;
  if (Math.abs(delta) < 0.01) {
    return latest.spreadUsd >= averageSpread ? "Holding firm" : "Still quiet";
  }

  return delta > 0 ? "Widening" : "Narrowing";
}

function trendWord(value: number): string {
  if (value > 0.01) {
    return "It has improved";
  }

  if (value < -0.01) {
    return "It has deteriorated";
  }

  return "It has stayed broadly flat";
}

function largerMagnitude(left: number, right: number): number {
  return Math.abs(left) >= Math.abs(right) ? left : right;
}
