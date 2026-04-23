import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  getBindingValue,
  renderTemplate,
  toNumeric,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import type { WidgetConfigThresholdRange } from "$lib/scada/plugins/types";
import template from "./alarm.widget.html?raw";
import styles from "./alarm.widget.css?raw";

const DEFAULT_THRESHOLDS: WidgetConfigThresholdRange[] = [
  { max: 25, color: "#3b82f6", label: "" },
  { min: 25, max: 60, color: "#546a0e", label: "" },
  { min: 60, max: 80, color: "#f59e0b", label: "" },
  { min: 80, color: "#ef4444", label: "" },
];

function normalizeHexColor(
  color: string | undefined,
  fallback = "#546a0e",
): string {
  if (!color) {
    return fallback;
  }
  const normalized = color.trim();
  if (/^#[0-9a-fA-F]{6}$/.test(normalized)) {
    return normalized.toLowerCase();
  }
  if (/^#[0-9a-fA-F]{3}$/.test(normalized)) {
    const r = normalized[1];
    const g = normalized[2];
    const b = normalized[3];
    return `#${r}${r}${g}${g}${b}${b}`.toLowerCase();
  }
  return fallback;
}

function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const parsed = normalizeHexColor(hex).slice(1);
  return {
    r: Number.parseInt(parsed.slice(0, 2), 16),
    g: Number.parseInt(parsed.slice(2, 4), 16),
    b: Number.parseInt(parsed.slice(4, 6), 16),
  };
}

function rgbToHex(r: number, g: number, b: number): string {
  const toChannel = (value: number): string =>
    Math.max(0, Math.min(255, Math.round(value)))
      .toString(16)
      .padStart(2, "0");
  return `#${toChannel(r)}${toChannel(g)}${toChannel(b)}`;
}

function shiftColor(hex: string, delta: number): string {
  const { r, g, b } = hexToRgb(hex);
  return rgbToHex(r + delta, g + delta, b + delta);
}

function hexToRgba(hex: string, alpha: number): string {
  const { r, g, b } = hexToRgb(hex);
  const a = Math.max(0, Math.min(1, alpha));
  return `rgba(${r}, ${g}, ${b}, ${a})`;
}

function readThresholdConfig(raw: unknown): WidgetConfigThresholdRange[] {
  if (!Array.isArray(raw)) {
    return DEFAULT_THRESHOLDS;
  }

  const parsed: WidgetConfigThresholdRange[] = [];
  for (const entry of raw) {
    if (!entry || typeof entry !== "object") {
      continue;
    }
    const source = entry as Partial<WidgetConfigThresholdRange>;
    parsed.push({
      min: typeof source.min === "number" ? source.min : undefined,
      max: typeof source.max === "number" ? source.max : undefined,
      color: normalizeHexColor(
        typeof source.color === "string" ? source.color : undefined,
      ),
      label: typeof source.label === "string" ? source.label : "",
    });
  }

  return parsed.length > 0 ? parsed : DEFAULT_THRESHOLDS;
}

function findThreshold(
  value: number | null,
  ranges: WidgetConfigThresholdRange[],
): WidgetConfigThresholdRange | null {
  if (value === null) {
    return null;
  }

  for (const range of ranges) {
    const min = typeof range.min === "number" ? range.min : -Infinity;
    const max = typeof range.max === "number" ? range.max : Infinity;
    if (value >= min && value <= max) {
      return range;
    }
  }

  return ranges[ranges.length - 1] ?? null;
}

export const alarmWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.ALARM,
    displayName: "Alarm Beacon",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "value",
    bindings: [
      {
        key: "value",
        label: "Value",
        access: "read",
        dataTypes: ["VAR_DATA_TYPE_INTEGER", "VAR_DATA_TYPE_FLOAT"],
      },
    ],
    configSchema: [
      {
        key: "thresholds",
        label: "Alarm Thresholds",
        type: "thresholds",
        description:
          "Configure ranges, color, and optional label for each alarm interval.",
      },
    ],
    defaultConfig: {
      thresholds: DEFAULT_THRESHOLDS,
    },
    capabilities: {
      minWidth: 80,
      minHeight: 100,
    },
  },
  tagName: "lirays-widget-alarm",
  styles,
  render: (data) => {
    const raw = getBindingValue(data, "value") ?? data.liveValue;
    const numeric = toNumeric(raw);
    const thresholds = readThresholdConfig(data.widgetConfig?.thresholds);
    const active = findThreshold(numeric, thresholds);

    const baseColor = normalizeHexColor(active?.color, "#3f3f46");
    const highlightColor = shiftColor(baseColor, 38);
    const shadowColor = shiftColor(baseColor, -42);
    const glowColor = hexToRgba(baseColor, 0.28);
    const statusLabel = (active?.label ?? "").trim();

    return {
      bodyHtml: renderTemplate(template, {
        base_color: baseColor,
        highlight_color: highlightColor,
        shadow_color: shadowColor,
        glow_color: glowColor,
        status_label: statusLabel,
        status_label_hidden_class: statusLabel.length > 0 ? "" : "hidden",
      }),
    };
  },
});
