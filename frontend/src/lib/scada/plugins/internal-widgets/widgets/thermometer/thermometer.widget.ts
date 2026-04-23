import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  getBindingNumericRange,
  getBindingUnit,
  getBindingValue,
  renderTemplate,
  toPercentInRange,
  toText,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import template from "./thermometer.widget.html?raw";
import styles from "./thermometer.widget.css?raw";

export const thermometerWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.THERMOMETER,
    displayName: "Thermometer",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "temperature",
    bindings: [
      {
        key: "temperature",
        label: "Temperature",
        access: "read",
        dataTypes: ["VAR_DATA_TYPE_INTEGER", "VAR_DATA_TYPE_FLOAT"],
      },
    ],
    capabilities: {
      minWidth: 210,
      minHeight: 280,
    },
  },
  tagName: "lirays-widget-thermometer",
  styles,
  render: (data) => {
    const temperature = getBindingValue(data, "temperature") ?? data.liveValue;
    const range = getBindingNumericRange(data, "temperature");
    const hasValue =
      temperature !== undefined &&
      temperature !== null &&
      String(temperature).trim().length > 0;
    const effectiveTemperature = hasValue ? temperature : range.min;
    const fillPercent = toPercentInRange(effectiveTemperature, range);
    const displayPercent = fillPercent;
    const valueLabel = toText(effectiveTemperature);
    const unit = getBindingUnit(data, "temperature") || "°C";

    return {
      bodyHtml: renderTemplate(template, {
        fill_percent: fillPercent,
        value_label: valueLabel,
        min_label: range.min,
        max_label: range.max,
        unit,
        fill_height: (23.703535 * displayPercent) / 100,
        fill_y: 77.816071 + (23.703535 * (100 - displayPercent)) / 100,
        fill_state_class: "value-present",
      }),
      footerLines: [`Temperature: ${valueLabel}${unit}`],
    };
  },
});
