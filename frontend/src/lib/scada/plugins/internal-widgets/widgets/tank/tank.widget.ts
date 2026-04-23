import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  renderTemplate,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import template from "./tank.widget.html?raw";
import styles from "./tank.widget.css?raw";

export const tankWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.TANK,
    displayName: "Tank",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "",
    bindings: [],
    capabilities: {
      titleOverlaysBody: true,
      keepAspectRatio: false,
      minWidth: 115,
      minHeight: 140,
    },
  },
  tagName: "lirays-widget-tank",
  styles,
  render: () => ({
    bodyHtml: renderTemplate(template, {}),
  }),
});
