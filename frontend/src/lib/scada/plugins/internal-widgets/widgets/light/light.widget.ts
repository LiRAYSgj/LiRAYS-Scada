import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  getBindingValue,
  getWidgetConfigValue,
  renderTemplate,
  toBoolean,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import template from "./light.widget.html?raw";
import styles from "./light.widget.css?raw";

const DEFAULT_ON_COLOR = "#84cc16";
const DEFAULT_OFF_COLOR = "#3f3f46";
const DEFAULT_ON_LABEL = "ON";
const DEFAULT_OFF_LABEL = "OFF";

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

function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const parsed = normalizeHexColor(hex, DEFAULT_OFF_COLOR).slice(1);
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

export const lightWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.LIGHT,
    displayName: "Light Indicator",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "state",
    bindings: [
      {
        key: "state",
        label: "State",
        access: "read",
        dataTypes: ["VAR_DATA_TYPE_BOOLEAN"],
      },
    ],
    configSchema: [
      {
        key: "onColor",
        label: "ON Color",
        type: "color",
      },
      {
        key: "onStateLabel",
        label: "ON Label",
        type: "text",
        placeholder: "ON",
      },
      {
        key: "offStateLabel",
        label: "OFF Label",
        type: "text",
        placeholder: "OFF",
      },
    ],
    defaultConfig: {
      onColor: DEFAULT_ON_COLOR,
      onStateLabel: DEFAULT_ON_LABEL,
      offStateLabel: DEFAULT_OFF_LABEL,
    },
    capabilities: {
      minWidth: 80,
      minHeight: 100,
    },
  },
  tagName: "lirays-widget-light",
  styles,
  render: (data) => {
    const isOn = toBoolean(getBindingValue(data, "state") ?? data.liveValue);
    const onColor = normalizeHexColor(
      getWidgetConfigValue(data, "onColor", DEFAULT_ON_COLOR),
      DEFAULT_ON_COLOR,
    );
    const baseColor = isOn ? onColor : DEFAULT_OFF_COLOR;
    const highlightColor = shiftColor(baseColor, 38);
    const shadowColor = shiftColor(baseColor, -42);
    const glowColor = isOn ? hexToRgba(onColor, 0.3) : "transparent";

    const stateLabel = String(
      getWidgetConfigValue(
        data,
        isOn ? "onStateLabel" : "offStateLabel",
        isOn ? DEFAULT_ON_LABEL : DEFAULT_OFF_LABEL,
      ),
    ).trim();

    return {
      bodyHtml: renderTemplate(template, {
        base_color: baseColor,
        highlight_color: highlightColor,
        shadow_color: shadowColor,
        glow_color: glowColor,
        glow_class: isOn ? "glow" : "",
        state_label: stateLabel,
        state_label_hidden_class: stateLabel.length > 0 ? "" : "hidden",
      }),
    };
  },
});
