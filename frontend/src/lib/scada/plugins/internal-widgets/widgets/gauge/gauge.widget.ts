import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  getBindingNumericRange,
  getBindingUnit,
  getBindingValue,
  renderTemplate,
  toNumeric,
  toPercentInRange,
  toText,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import template from "./gauge.widget.html?raw";
import styles from "./gauge.widget.css?raw";

function gaugeAngle(percent: number): number {
  return -110 + (220 * percent) / 100;
}

function polarPoint(
  cx: number,
  cy: number,
  radius: number,
  angleDeg: number,
): {
  x: number;
  y: number;
} {
  const rad = (angleDeg - 90) * (Math.PI / 180);
  return {
    x: cx + radius * Math.cos(rad),
    y: cy + radius * Math.sin(rad),
  };
}

function buildTickLines(count: number): string {
  const lines: string[] = [];
  const cx = 50;
  const cy = 50;
  for (let i = 0; i <= count; i += 1) {
    const t = i / count;
    const angle = -110 + t * 220;
    const isMajor = i % 5 === 0;
    const outer = polarPoint(cx, cy, 38, angle);
    const inner = polarPoint(cx, cy, isMajor ? 30 : 33, angle);
    lines.push(
      `<line class="gauge-tick ${isMajor ? "major" : "minor"}" x1="${outer.x.toFixed(
        3,
      )}" y1="${outer.y.toFixed(3)}" x2="${inner.x.toFixed(
        3,
      )}" y2="${inner.y.toFixed(3)}" />`,
    );
  }
  return lines.join("");
}

export const gaugeWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.GAUGE,
    displayName: "Gauge",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "value",
    bindings: [
      {
        key: "value",
        label: "Value",
        access: "read",
        dataTypes: ["VAR_DATA_TYPE_INTEGER", "VAR_DATA_TYPE_FLOAT"],
      },
    ],
    capabilities: {
      minWidth: 240,
      minHeight: 300,
    },
  },
  tagName: "lirays-widget-gauge",
  styles,
  render: (data) => {
    const raw = getBindingValue(data, "value") ?? data.liveValue;
    const numeric = toNumeric(raw);
    const range = getBindingNumericRange(data, "value");
    const pct = toPercentInRange(raw, range);
    const angle = gaugeAngle(pct);
    const unit = getBindingUnit(data, "value");
    const valueLabel = toText(raw);
    const ticksHtml = buildTickLines(30);
    const minTickPoint = polarPoint(50, 50, 38, -110);
    const maxTickPoint = polarPoint(50, 50, 38, 110);
    const minLabelPoint = {
      x: minTickPoint.x + 4,
      y: Math.min(95, minTickPoint.y + 8),
    };
    const maxLabelPoint = {
      x: maxTickPoint.x - 4,
      y: Math.min(95, maxTickPoint.y + 8),
    };
    const bodyHtml = renderTemplate(template, {
      ticks_html: ticksHtml,
      angle,
      value_label: valueLabel,
      unit,
      min_x: minLabelPoint.x.toFixed(3),
      min_y: minLabelPoint.y.toFixed(3),
      max_x: maxLabelPoint.x.toFixed(3),
      max_y: maxLabelPoint.y.toFixed(3),
      min_label: range.min,
      max_label: range.max,
      has_value_class: numeric === null ? "no-value" : "",
    });

    return {
      bodyHtml,
      footerLines: [`Value: ${valueLabel}${unit}`],
    };
  },
});
