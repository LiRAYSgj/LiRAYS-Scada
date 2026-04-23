import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  renderTemplate,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import template from "./pump.widget.html?raw";
import styles from "./pump.widget.css?raw";

export const pumpWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.PUMP,
    displayName: "Pump",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "",
    bindings: [],
    capabilities: {
      titleOverlaysBody: true,
      keepAspectRatio: true,
      minWidth: 160,
      minHeight: 120,
    },
  },
  tagName: "lirays-widget-pump",
  styles,
  render: () => ({
    bodyHtml: renderTemplate(template, {}),
  }),
});
