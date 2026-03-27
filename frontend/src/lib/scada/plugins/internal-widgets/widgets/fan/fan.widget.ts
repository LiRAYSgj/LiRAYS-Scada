import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  getBindingValue,
  renderTemplate,
  toPercent,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import template from "./fan.widget.html?raw";
import styles from "./fan.widget.css?raw";

export const fanWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.FAN,
    displayName: "Fan",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "speed",
    bindings: [{ key: "speed", label: "Speed", access: "read" }],
  },
  tagName: "lirays-widget-fan",
  styles,
  render: (data) => {
    const speed = toPercent(getBindingValue(data, "speed") ?? data.liveValue);
    return {
      bodyHtml: renderTemplate(template, {
        speed,
        spin_class: speed > 0 ? "spin" : "",
      }),
      footerLines: [`Speed: ${speed}%`],
    };
  },
});
