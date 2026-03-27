import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  getBindingValue,
  renderTemplate,
  toPercent,
  toText,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import template from "./tank.widget.html?raw";
import styles from "./tank.widget.css?raw";

export const tankWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.TANK,
    displayName: "Tank",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "level",
    bindings: [
      { key: "level", label: "Level", access: "read", required: true },
      { key: "temperature", label: "Temperature", access: "read" },
      { key: "pressure", label: "Pressure", access: "read" },
    ],
  },
  tagName: "lirays-widget-tank",
  styles,
  render: (data) => {
    const level = toPercent(getBindingValue(data, "level") ?? data.liveValue);
    const temperature = toText(getBindingValue(data, "temperature"));
    const pressure = toText(getBindingValue(data, "pressure"));

    return {
      bodyHtml: renderTemplate(template, { level }),
      footerLines: [
        `Level: ${level}%`,
        `Temp: ${temperature}`,
        `Pressure: ${pressure}`,
      ],
    };
  },
});
