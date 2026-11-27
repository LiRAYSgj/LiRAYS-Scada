import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  escapeHtml,
  getBindingNumericRange,
  getBindingUnit,
  getBindingValue,
  renderTemplate,
  toNumeric,
  toPercentInRange,
  toText,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import {
  readThresholds,
  resolveThreshold,
  thresholdBands,
} from "$lib/scada/plugins/thresholds";
import type { WidgetConfigThresholds } from "$lib/scada/plugins/types";
import template from "./gauge.widget.html?raw";
import styles from "./gauge.widget.css?raw";

const DEFAULT_THRESHOLDS: WidgetConfigThresholds = {
  default: { color: "#ef4444", label: "" },
  levels: [],
};

const DEFAULT_VALUE_COLOR = "#f3f4f6";
const BAND_OUTER_RADIUS = 38;
const BAND_INNER_RADIUS = 30;

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

function bandPath(startDeg: number, endDeg: number): string {
  const largeArc = endDeg - startDeg > 180 ? 1 : 0;
  const outerStart = polarPoint(50, 50, BAND_OUTER_RADIUS, startDeg);
  const outerEnd = polarPoint(50, 50, BAND_OUTER_RADIUS, endDeg);
  const innerEnd = polarPoint(50, 50, BAND_INNER_RADIUS, endDeg);
  const innerStart = polarPoint(50, 50, BAND_INNER_RADIUS, startDeg);
  return [
    `M ${outerStart.x.toFixed(3)} ${outerStart.y.toFixed(3)}`,
    `A ${BAND_OUTER_RADIUS} ${BAND_OUTER_RADIUS} 0 ${largeArc} 1 ${outerEnd.x.toFixed(3)} ${outerEnd.y.toFixed(3)}`,
    `L ${innerEnd.x.toFixed(3)} ${innerEnd.y.toFixed(3)}`,
    `A ${BAND_INNER_RADIUS} ${BAND_INNER_RADIUS} 0 ${largeArc} 0 ${innerStart.x.toFixed(3)} ${innerStart.y.toFixed(3)}`,
    "Z",
  ].join(" ");
}

function buildBands(
  thresholds: WidgetConfigThresholds,
  range: { min: number; max: number },
): string {
  const span = range.max - range.min;
  const toAngle = (value: number): number =>
    gaugeAngle(((value - range.min) / span) * 100);
  return thresholdBands(thresholds, range)
    .map((band) => {
      const title = band.label.trim()
        ? `<title>${escapeHtml(band.label)}</title>`
        : "";
      return `<path class="gauge-band" d="${bandPath(
        toAngle(band.from),
        toAngle(band.to),
      )}" fill="${escapeHtml(band.color)}">${title}</path>`;
    })
    .join("");
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
    configSchema: [
      {
        key: "thresholds",
        label: "Scale Bands",
        type: "thresholds",
        description:
          "Colors painted on the scale. Values <= a threshold use its color, values above the highest use the default. Without thresholds no bands are drawn.",
      },
    ],
    defaultConfig: {
      thresholds: DEFAULT_THRESHOLDS,
    },
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
    const thresholds = readThresholds(
      data.widgetConfig?.thresholds,
      DEFAULT_THRESHOLDS,
    );
    const bandsHtml = buildBands(thresholds, range);
    // The value text follows the active band; without bands it keeps its color.
    const valueColor =
      thresholds.levels.length > 0
        ? (resolveThreshold(numeric, thresholds)?.color ?? DEFAULT_VALUE_COLOR)
        : DEFAULT_VALUE_COLOR;
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
      bands_html: bandsHtml,
      ticks_html: ticksHtml,
      angle,
      value_label: valueLabel,
      value_color: valueColor,
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
