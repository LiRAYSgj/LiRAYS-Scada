import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  getBindingValue,
  renderTemplate,
  toBoolean,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import template from "./light.widget.html?raw";
import styles from "./light.widget.css?raw";

export const lightWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.LIGHT,
    displayName: "Light Indicator",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "state",
    bindings: [{ key: "state", label: "State", access: "read" }],
  },
  tagName: "lirays-widget-light",
  styles,
  render: (data) => {
    const state = toBoolean(getBindingValue(data, "state") ?? data.liveValue);
    return {
      bodyHtml: renderTemplate(template, {
        light_state_class: state ? "on" : "",
        light_label: state ? "ON" : "OFF",
      }),
      footerLines: [`Status: ${state ? "GREEN" : "RED"}`],
    };
  },
});
