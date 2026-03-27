import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  renderTemplate,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import template from "./valve.widget.html?raw";
import styles from "./valve.widget.css?raw";

export const valveWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.VALVE,
    displayName: "Valve",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "state",
    bindings: [{ key: "state", label: "State", access: "read" }],
  },
  tagName: "lirays-widget-valve",
  styles,
  render: () => ({
    bodyHtml: renderTemplate(template, {}),
  }),
});
