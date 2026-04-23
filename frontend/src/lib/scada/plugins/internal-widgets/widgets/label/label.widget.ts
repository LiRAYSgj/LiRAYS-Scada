import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  getWidgetConfigValue,
  renderTemplate,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import template from "./label.widget.html?raw";
import styles from "./label.widget.css?raw";

const DEFAULT_TEXT_COLOR = "#e5e7eb";

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

export const labelWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.LABEL,
    displayName: "Label",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "",
    bindings: [],
    configSchema: [
      {
        key: "textColor",
        label: "Text Color",
        type: "color",
      },
    ],
    defaultConfig: {
      textColor: DEFAULT_TEXT_COLOR,
    },
    capabilities: {
      showTitle: false,
      keepAspectRatio: false,
      minWidth: 100,
      minHeight: 50,
    },
  },
  tagName: "lirays-widget-label",
  styles,
  render: (data) => {
    const value = data.title?.trim() || "Label";
    const textColor = normalizeHexColor(
      getWidgetConfigValue(data, "textColor", DEFAULT_TEXT_COLOR),
      DEFAULT_TEXT_COLOR,
    );
    return {
      bodyHtml: renderTemplate(template, {
        value,
        text_color: textColor,
      }),
    };
  },
});
