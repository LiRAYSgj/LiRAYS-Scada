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
    primaryBindingKey: "",
    bindings: [],
    capabilities: {
      titleOverlaysBody: true,
      keepAspectRatio: true,
      minWidth: 120,
      minHeight: 120,
    },
  },
  tagName: "lirays-widget-valve",
  styles,
  render: () => ({
    bodyHtml: renderTemplate(template, {}),
  }),
});
