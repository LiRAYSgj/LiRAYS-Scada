import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  getBindingValue,
  renderTemplate,
  toText,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import template from "./label.widget.html?raw";
import styles from "./label.widget.css?raw";

export const labelWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.LABEL,
    displayName: "Label",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "value",
    bindings: [{ key: "value", label: "Value", access: "read" }],
  },
  tagName: "lirays-widget-label",
  styles,
  render: (data) => {
    const value = toText(getBindingValue(data, "value") ?? data.liveValue);
    return {
      bodyHtml: renderTemplate(template, { value }),
      footerLines: [`Text: ${value}`],
    };
  },
});
