import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  getBindingValue,
  renderTemplate,
  toPercent,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import template from "./slider.widget.html?raw";
import styles from "./slider.widget.css?raw";

export const sliderWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.SLIDER,
    displayName: "Slider",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "command",
    bindings: [{ key: "command", label: "Command", access: "readwrite" }],
  },
  tagName: "lirays-widget-slider",
  styles,
  render: (data) => {
    const command = toPercent(
      getBindingValue(data, "command") ?? data.liveValue,
    );
    return {
      bodyHtml: renderTemplate(template, { command }),
      footerLines: [`Command: ${command}%`],
    };
  },
  bind: (context) => {
    const slider = context.query<HTMLInputElement>(
      'input[data-widget-input="slider"]',
    );
    if (!slider) return;

    context.listen(slider, "input", () => {
      const parsed = Number(slider.value);
      context.debounce("slider-command", 300, () => {
        context.writeBinding("command", Number.isFinite(parsed) ? parsed : 0);
      });
    });
  },
});
