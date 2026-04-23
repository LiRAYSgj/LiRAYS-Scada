import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  getBindingValue,
  renderTemplate,
  toText,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import template from "./seven-segment.widget.html?raw";
import styles from "./seven-segment.widget.css?raw";

function normalizeDisplayValue(raw: unknown): string {
  if (raw === undefined || raw === null) {
    return "0";
  }
  if (typeof raw === "boolean") {
    return raw ? "1" : "0";
  }
  const value = toText(raw as string | number | boolean | undefined).trim();
  return value.length > 0 ? value : "0";
}

const DIGIT_SEGMENTS: Record<string, string[]> = {
  "0": ["a", "b", "c", "d", "e", "f"],
  "1": ["b", "c"],
  "2": ["a", "b", "g", "e", "d"],
  "3": ["a", "b", "c", "d", "g"],
  "4": ["f", "g", "b", "c"],
  "5": ["a", "f", "g", "c", "d"],
  "6": ["a", "f", "e", "d", "c", "g"],
  "7": ["a", "b", "c"],
  "8": ["a", "b", "c", "d", "e", "f", "g"],
  "9": ["a", "b", "c", "d", "f", "g"],
  "-": ["g"],
};

function digitSvg(character: string, showDot: boolean): string {
  const active = DIGIT_SEGMENTS[character] ?? [];
  const has = (seg: string): string => (active.includes(seg) ? "on" : "off");
  return `
<svg viewBox="0 0 24 44" class="digit">
  <rect class="seg ${has("a")}" x="5" y="1.2" width="14" height="2.4" rx="1.1" />
  <rect class="seg ${has("b")}" x="19.2" y="4.2" width="2.4" height="15" rx="1.1" />
  <rect class="seg ${has("c")}" x="19.2" y="23.8" width="2.4" height="15" rx="1.1" />
  <rect class="seg ${has("d")}" x="5" y="40.4" width="14" height="2.4" rx="1.1" />
  <rect class="seg ${has("e")}" x="2.4" y="23.8" width="2.4" height="15" rx="1.1" />
  <rect class="seg ${has("f")}" x="2.4" y="4.2" width="2.4" height="15" rx="1.1" />
  <rect class="seg ${has("g")}" x="5" y="20.8" width="14" height="2.4" rx="1.1" />
  <circle class="dot ${showDot ? "on" : "off"}" cx="22.5" cy="40.8" r="1.4" />
</svg>`;
}

const MIN_VISIBLE_DIGITS = 4;

function displaySvg(value: string): { html: string; count: number } {
  const chars = value.replace(/\s+/g, "").slice(0, 6).split("");
  const tokens: Array<{ ch: string; dot: boolean }> = [];
  for (let i = 0; i < chars.length; i += 1) {
    const ch = chars[i];
    if (ch === ".") {
      continue;
    }
    tokens.push({ ch, dot: chars[i + 1] === "." });
  }

  const count = Math.max(MIN_VISIBLE_DIGITS, tokens.length, 1);
  const paddedTokens = [
    ...Array.from({ length: Math.max(0, count - tokens.length) }, () => ({
      ch: " ",
      dot: false,
    })),
    ...tokens,
  ];
  const html = paddedTokens
    .map((token) => digitSvg(token.ch, token.dot))
    .join("");
  return { html, count };
}

export const sevenSegmentWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.SEVEN_SEGMENT,
    displayName: "7 Segment Display",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "value",
    bindings: [
      {
        key: "value",
        label: "Value",
        access: "read",
        dataTypes: [
          "VAR_DATA_TYPE_INTEGER",
          "VAR_DATA_TYPE_FLOAT",
          "VAR_DATA_TYPE_TEXT",
        ],
      },
    ],
  },
  tagName: "lirays-widget-seven-segment",
  styles,
  render: (data) => {
    const raw = getBindingValue(data, "value") ?? data.liveValue;
    const display = normalizeDisplayValue(raw);
    const { html: digitsHtml, count: digitCount } = displaySvg(display);
    const bodyHtml = renderTemplate(template, {
      digits_html: digitsHtml,
      digit_count: digitCount,
      display,
    });

    return {
      bodyHtml,
      footerLines: [`Display: ${display}`],
    };
  },
});
