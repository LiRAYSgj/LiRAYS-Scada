import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  getWidgetConfigValue,
  renderTemplate,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import template from "./push-button.widget.html?raw";
import styles from "./push-button.widget.css?raw";

const PUSH_BUTTON_COLOR_OPTIONS = [
  { value: "outline-accent", label: "Outline Accent" },
  { value: "outline-muted", label: "Outline Muted" },
  { value: "filled-accent", label: "Filled Accent" },
  { value: "destructive", label: "Destructive" },
] as const;
type PushButtonColorVariant =
  (typeof PUSH_BUTTON_COLOR_OPTIONS)[number]["value"];
const PUSH_BUTTON_COLOR_VALUES = PUSH_BUTTON_COLOR_OPTIONS.map(
  (item) => item.value,
) as readonly PushButtonColorVariant[];

function isPushButtonColorVariant(
  value: string,
): value is PushButtonColorVariant {
  return (PUSH_BUTTON_COLOR_VALUES as readonly string[]).includes(value);
}

export const pushButtonWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.PUSH_BUTTON,
    displayName: "Push Button",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "",
    bindings: [],
    events: [
      { name: "click", label: "Click" },
      { name: "doubleClick", label: "Double Click" },
      { name: "rightClick", label: "Right Click" },
    ],
    configSchema: [
      {
        key: "colorVariant",
        label: "Button Style",
        type: "select",
        options: [...PUSH_BUTTON_COLOR_OPTIONS],
      },
    ],
    defaultConfig: {
      colorVariant: "filled-accent",
    },
    capabilities: {
      showTitle: false,
      keepAspectRatio: false,
      minWidth: 210,
      minHeight: 120,
    },
  },
  tagName: "lirays-widget-push-button",
  styles,
  interactionSourceSelector: 'button[data-widget-event-source="true"]',
  render: (data) => {
    const variant = getWidgetConfigValue<string>(
      data,
      "colorVariant",
      "filled-accent",
    );
    const safeVariant = isPushButtonColorVariant(variant)
      ? variant
      : "filled-accent";

    return {
      bodyHtml: renderTemplate(template, {
        button_label: data.title?.trim() || "Push",
        button_variant_class: `variant-${safeVariant}`,
      }),
    };
  },
});
