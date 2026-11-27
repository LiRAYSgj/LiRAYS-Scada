import type {
  WidgetConfigThreshold,
  WidgetConfigThresholds,
} from "$lib/scada/plugins/types";

/**
 * Helpers for the `thresholds` widget config field.
 *
 * Model: a default color (always present) plus a list of single-value
 * thresholds. A value takes the color of the first threshold (ascending)
 * it is `<=` to; values above the highest threshold use the default.
 */

export const FALLBACK_THRESHOLD_COLOR = "#546a0e";

function readColor(raw: unknown, fallback: string): string {
  return typeof raw === "string" && raw.trim().length > 0 ? raw : fallback;
}

function readLabel(raw: unknown): string {
  return typeof raw === "string" ? raw : "";
}

function isRecord(raw: unknown): raw is Record<string, unknown> {
  return !!raw && typeof raw === "object" && !Array.isArray(raw);
}

export function sortThresholdLevels(
  levels: WidgetConfigThreshold[],
): WidgetConfigThreshold[] {
  return [...levels].sort((a, b) => a.value - b.value);
}

/**
 * Converts the legacy `{ min, max, color, label }[]` format: every range with
 * a `max` becomes a threshold, the first open-ended range becomes the default.
 */
function fromLegacyRanges(
  raw: unknown[],
  fallback: WidgetConfigThresholds,
): WidgetConfigThresholds {
  const levels: WidgetConfigThreshold[] = [];
  let defaultLevel: WidgetConfigThresholds["default"] | null = null;
  for (const entry of raw) {
    if (!isRecord(entry)) {
      continue;
    }
    const color = readColor(entry.color, FALLBACK_THRESHOLD_COLOR);
    const label = readLabel(entry.label);
    if (typeof entry.max === "number" && Number.isFinite(entry.max)) {
      levels.push({ value: entry.max, color, label });
    } else if (!defaultLevel) {
      defaultLevel = { color, label };
    }
  }
  if (levels.length === 0 && !defaultLevel) {
    return fallback;
  }
  return {
    default: defaultLevel ?? fallback.default,
    levels: sortThresholdLevels(levels),
  };
}

/** Parses a stored config value (current or legacy format), sorted ascending. */
export function readThresholds(
  raw: unknown,
  fallback: WidgetConfigThresholds,
): WidgetConfigThresholds {
  const parsed = parseThresholds(raw, fallback);
  return { ...parsed, levels: sortThresholdLevels(parsed.levels) };
}

/**
 * Like `readThresholds` but keeps the stored order of the current format, so
 * an editor can defer re-sorting until the user commits a value.
 */
export function parseThresholds(
  raw: unknown,
  fallback: WidgetConfigThresholds,
): WidgetConfigThresholds {
  if (Array.isArray(raw)) {
    return fromLegacyRanges(raw, fallback);
  }
  if (!isRecord(raw)) {
    return fallback;
  }
  const rawDefault = isRecord(raw.default) ? raw.default : {};
  const levels: WidgetConfigThreshold[] = [];
  if (Array.isArray(raw.levels)) {
    for (const entry of raw.levels) {
      if (
        !isRecord(entry) ||
        typeof entry.value !== "number" ||
        !Number.isFinite(entry.value)
      ) {
        continue;
      }
      levels.push({
        value: entry.value,
        color: readColor(entry.color, FALLBACK_THRESHOLD_COLOR),
        label: readLabel(entry.label),
      });
    }
  }
  return {
    default: {
      color: readColor(rawDefault.color, fallback.default.color),
      label: readLabel(rawDefault.label),
    },
    levels,
  };
}

/**
 * Returns the color/label that applies to `value`, or `null` when there is no
 * numeric value. Levels are evaluated ascending regardless of input order.
 */
export function resolveThreshold(
  value: number | null,
  config: WidgetConfigThresholds,
): { color: string; label: string } | null {
  if (value === null || Number.isNaN(value)) {
    return null;
  }
  for (const level of sortThresholdLevels(config.levels)) {
    if (value <= level.value) {
      return { color: level.color, label: level.label ?? "" };
    }
  }
  return { color: config.default.color, label: config.default.label ?? "" };
}

/** Indexes (in the given order) whose value repeats an earlier level. */
export function duplicateThresholdIndexes(
  levels: WidgetConfigThreshold[],
): Set<number> {
  const seen = new Set<number>();
  const duplicates = new Set<number>();
  levels.forEach((level, index) => {
    if (seen.has(level.value)) {
      duplicates.add(index);
    }
    seen.add(level.value);
  });
  return duplicates;
}

/** Value proposed for a newly added threshold: above the current highest. */
export function nextThresholdValue(levels: WidgetConfigThreshold[]): number {
  if (levels.length === 0) {
    return 0;
  }
  return Math.max(...levels.map((level) => level.value)) + 10;
}

/** One colored stretch of a numeric scale, see `thresholdBands`. */
export interface ThresholdBand {
  from: number;
  to: number;
  color: string;
  label: string;
}

/**
 * Splits `[range.min, range.max]` into colored bands following the same rule
 * as `resolveThreshold`: each level colors the stretch up to its value, the
 * default colors the stretch above the highest level. Levels outside the range
 * are clipped and empty bands dropped. No levels means no bands.
 */
export function thresholdBands(
  config: WidgetConfigThresholds,
  range: { min: number; max: number },
): ThresholdBand[] {
  if (config.levels.length === 0 || !(range.max > range.min)) {
    return [];
  }
  const stops = [
    ...sortThresholdLevels(config.levels).map((level) => ({
      to: level.value,
      color: level.color,
      label: level.label ?? "",
    })),
    {
      to: Number.POSITIVE_INFINITY,
      color: config.default.color,
      label: config.default.label ?? "",
    },
  ];
  const bands: ThresholdBand[] = [];
  let from = range.min;
  for (const stop of stops) {
    const to = Math.min(stop.to, range.max);
    if (to > from) {
      bands.push({ from, to, color: stop.color, label: stop.label });
      from = to;
    }
    if (from >= range.max) {
      break;
    }
  }
  return bands;
}
