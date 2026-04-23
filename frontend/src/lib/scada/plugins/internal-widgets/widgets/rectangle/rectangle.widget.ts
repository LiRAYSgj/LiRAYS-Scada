import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  getWidgetConfigValue,
  renderTemplate,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import template from "./rectangle.widget.html?raw";
import styles from "./rectangle.widget.css?raw";

const DEFAULT_FILL_COLOR = "#2f3440";

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

export const rectangleWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.RECTANGLE,
    displayName: "Rectangle",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "",
    bindings: [],
    configSchema: [
      {
        key: "fillColor",
        label: "Color",
        type: "color",
      },
    ],
    defaultConfig: {
      fillColor: DEFAULT_FILL_COLOR,
    },
    capabilities: {
      keepAspectRatio: false,
      minWidth: 140,
      minHeight: 100,
    },
  },
  tagName: "lirays-widget-rectangle",
  styles,
  render: (data) => {
    const fillColor = normalizeHexColor(
      getWidgetConfigValue(data, "fillColor", DEFAULT_FILL_COLOR),
      DEFAULT_FILL_COLOR,
    );

    return {
      bodyHtml: renderTemplate(template, {
        fill_color: fillColor,
      }),
    };
  },
});
