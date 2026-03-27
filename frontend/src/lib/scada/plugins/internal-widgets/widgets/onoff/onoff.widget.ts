import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  getBindingValue,
  renderTemplate,
  toBoolean,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import template from "./onoff.widget.html?raw";
import styles from "./onoff.widget.css?raw";

export const onoffWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.ONOFF,
    displayName: "On/Off Input",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "command",
    bindings: [{ key: "command", label: "Command", access: "readwrite" }],
  },
  tagName: "lirays-widget-onoff",
  styles,
  render: (data) => {
    const isOn = toBoolean(getBindingValue(data, "command") ?? data.liveValue);
    return {
      bodyHtml: renderTemplate(template, {
        button_label: isOn ? "ON" : "OFF",
        button_state_class: isOn ? "active" : "",
      }),
      footerLines: [`Command: ${isOn ? "ON" : "OFF"}`],
    };
  },
  bind: (context) => {
    const button = context.query<HTMLButtonElement>('button[data-widget-input="toggle"]');
    if (!button) return;

    context.listen(button, "click", () => {
      const current = toBoolean(
        context.getBindingValue("command") ?? context.data.liveValue,
      );
      context.writeBinding("command", !current);
    });
  },
});
