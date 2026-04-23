import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  getBindingNumericRange,
  getBindingValue,
  renderTemplate,
  toNumeric,
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
    bindings: [
      {
        key: "command",
        label: "Command",
        access: "readwrite",
        dataTypes: ["VAR_DATA_TYPE_INTEGER", "VAR_DATA_TYPE_FLOAT"],
      },
    ],
    capabilities: {
      inputWidget: true,
      minWidth: 210,
      minHeight: 120,
    },
  },
  tagName: "lirays-widget-slider",
  styles,
  render: (data) => {
    const range = getBindingNumericRange(data, "command");
    const current = toNumeric(
      getBindingValue(data, "command") ?? data.liveValue,
    );
    const commandSliderValue =
      current === null
        ? range.min
        : Math.max(range.min, Math.min(range.max, current));
    const commandDisplayValue = current === null ? "--" : String(current);
    const rangeStep =
      data.bindings?.command?.[0]?.dataType === "VAR_DATA_TYPE_FLOAT"
        ? "0.01"
        : "1";

    return {
      bodyHtml: renderTemplate(template, {
        command_slider_value: commandSliderValue,
        command_value_label: commandDisplayValue,
        range_min: range.min,
        range_max: range.max,
        range_min_label: range.min,
        range_max_label: range.max,
        range_step: rangeStep,
      }),
      footerLines: [`Command: ${commandDisplayValue}`],
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
