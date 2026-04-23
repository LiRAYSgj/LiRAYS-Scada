import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  getBindingValue,
  renderTemplate,
  toText,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import template from "./text.widget.html?raw";
import styles from "./text.widget.css?raw";

export const textWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.TEXT,
    displayName: "Text",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "value",
    bindings: [
      {
        key: "value",
        label: "Value",
        access: "read",
        dataTypes: [
          "VAR_DATA_TYPE_INTEGER",
          "VAR_DATA_TYPE_FLOAT",
          "VAR_DATA_TYPE_TEXT",
          "VAR_DATA_TYPE_BOOLEAN",
        ],
      },
    ],
    capabilities: {
      keepAspectRatio: false,
      minWidth: 120,
      minHeight: 50,
    },
  },
  tagName: "lirays-widget-text",
  styles,
  render: (data) => {
    const value = toText(getBindingValue(data, "value") ?? data.liveValue);
    return {
      bodyHtml: renderTemplate(template, { value }),
    };
  },
});
