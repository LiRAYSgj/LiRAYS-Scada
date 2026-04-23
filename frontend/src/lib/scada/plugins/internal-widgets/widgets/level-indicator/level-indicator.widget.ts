import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  getBindingNumericRange,
  getBindingUnit,
  getBindingValue,
  getWidgetConfigValue,
  renderTemplate,
  toNumeric,
  toPercentInRange,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import template from "./level-indicator.widget.html?raw";
import styles from "./level-indicator.widget.css?raw";

const DEFAULT_LEVEL_COLOR = "#8f9a6a";
const FACE_BORDER_COLOR = "#2c311c";

type RenderMode = "2d" | "3d";

function normalizeHexColor(
  color: string | undefined,
  fallback: string,
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

function hexToRgb(color: string): { r: number; g: number; b: number } {
  const safe = normalizeHexColor(color, DEFAULT_LEVEL_COLOR).slice(1);
  return {
    r: Number.parseInt(safe.slice(0, 2), 16),
    g: Number.parseInt(safe.slice(2, 4), 16),
    b: Number.parseInt(safe.slice(4, 6), 16),
  };
}

function toHex(value: number): string {
  return Math.min(255, Math.max(0, Math.round(value)))
    .toString(16)
    .padStart(2, "0");
}

function mixHex(color: string, target: string, ratio: number): string {
  const from = hexToRgb(color);
  const to = hexToRgb(target);
  const clamped = Math.min(1, Math.max(0, ratio));
  return `#${toHex(from.r + (to.r - from.r) * clamped)}${toHex(
    from.g + (to.g - from.g) * clamped,
  )}${toHex(from.b + (to.b - from.b) * clamped)}`;
}

function resolveRenderMode(rawMode: unknown): RenderMode {
  return rawMode === "2d" ? "2d" : "3d";
}

export const levelIndicatorWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.LEVEL_INDICATOR,
    displayName: "Level Indicator",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "level",
    bindings: [
      {
        key: "level",
        label: "Level",
        access: "read",
        dataTypes: ["VAR_DATA_TYPE_INTEGER", "VAR_DATA_TYPE_FLOAT"],
        required: true,
      },
    ],
    configSchema: [
      {
        key: "renderMode",
        label: "Mode",
        type: "select",
        options: [
          { value: "2d", label: "2D" },
          { value: "3d", label: "3D" },
        ],
      },
      {
        key: "levelColor",
        label: "Level Color",
        type: "color",
      },
      {
        key: "showFaceBorders",
        label: "Show Face Borders",
        type: "boolean",
      },
    ],
    defaultConfig: {
      renderMode: "3d",
      levelColor: DEFAULT_LEVEL_COLOR,
      showFaceBorders: true,
    },
    capabilities: {
      keepAspectRatio: false,
      minWidth: 250,
      minHeight: 280,
    },
  },
  tagName: "lirays-widget-level-indicator",
  styles,
  render: (data) => {
    const raw = getBindingValue(data, "level") ?? data.liveValue;
    const numeric = toNumeric(raw);
    const range = getBindingNumericRange(data, "level");
    const levelPercent = toPercentInRange(raw, range);
    const levelLabel = numeric === null ? "--" : String(numeric);
    const unit = getBindingUnit(data, "level");

    const renderMode = resolveRenderMode(
      getWidgetConfigValue<string>(data, "renderMode", "3d"),
    );
    const levelColor = normalizeHexColor(
      getWidgetConfigValue<string>(data, "levelColor", DEFAULT_LEVEL_COLOR),
      DEFAULT_LEVEL_COLOR,
    );
    const showFaceBorders = Boolean(
      getWidgetConfigValue<boolean>(data, "showFaceBorders", true),
    );

    const levelHighlight = mixHex(levelColor, "#ffffff", 0.42);
    const levelShadow = mixHex(levelColor, "#000000", 0.34);
    const levelTop = mixHex(levelColor, "#ffffff", 0.26);
    const levelBottom = mixHex(levelColor, "#000000", 0.24);

    return {
      bodyHtml: renderTemplate(template, {
        mode_class: renderMode === "3d" ? "mode-3d" : "mode-2d",
        borders_class: showFaceBorders ? "show-borders" : "hide-borders",
        value_state_class: numeric === null ? "value-missing" : "value-present",
        level_percent: String(levelPercent),
        value_label: levelLabel,
        min_label: String(range.min),
        max_label: String(range.max),
        unit,
        level_color: levelColor,
        level_highlight: levelHighlight,
        level_shadow: levelShadow,
        level_top: levelTop,
        level_bottom: levelBottom,
        face_border: FACE_BORDER_COLOR,
      }),
      footerLines: [`Level: ${levelLabel} (${levelPercent}%)`],
    };
  },
});
