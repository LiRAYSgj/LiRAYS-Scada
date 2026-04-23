import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  getBindingValue,
  renderTemplate,
  toNumeric,
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
    bindings: [
      {
        key: "speed",
        label: "Speed",
        access: "read",
        dataTypes: ["VAR_DATA_TYPE_INTEGER", "VAR_DATA_TYPE_FLOAT"],
      },
    ],
    capabilities: {
      titleOverlaysBody: true,
      keepAspectRatio: true,
      minWidth: 120,
      minHeight: 120,
    },
  },
  tagName: "lirays-widget-fan",
  styles,
  render: (data) => {
    const rawSpeed = toNumeric(
      getBindingValue(data, "speed") ?? data.liveValue,
    );
    const clampedSpeed = Math.max(-100, Math.min(100, rawSpeed ?? 0));
    const speedRatio = Math.abs(clampedSpeed) / 100;
    const minDuration = 0.45;
    const maxDuration = 2.8;
    const durationSeconds =
      maxDuration - (maxDuration - minDuration) * speedRatio;
    const spinClass =
      clampedSpeed > 0 ? "spin-ccw" : clampedSpeed < 0 ? "spin-cw" : "";

    return {
      bodyHtml: renderTemplate(template, {
        spin_class: spinClass,
        spin_duration: `${durationSeconds.toFixed(3)}s`,
      }),
    };
  },
});
