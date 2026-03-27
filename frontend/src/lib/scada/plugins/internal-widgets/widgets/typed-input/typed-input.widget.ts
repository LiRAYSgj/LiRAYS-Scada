import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  getBindingValue,
  renderTemplate,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import template from "./typed-input.widget.html?raw";
import styles from "./typed-input.widget.css?raw";

export const typedInputWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.TYPED_INPUT,
    displayName: "Typed Input",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "command",
    bindings: [{ key: "command", label: "Command", access: "readwrite" }],
  },
  tagName: "lirays-widget-typed-input",
  styles,
  render: (data) => {
    const source = data.bindings?.command?.[0] ?? data.sourceNode;
    const sourceType = String(source?.dataType ?? "Text").toLowerCase();
    const inputType =
      sourceType === "integer" || sourceType === "float" ? "number" : "text";
    const inputStep = sourceType === "integer" ? "1" : sourceType === "float" ? "0.01" : "any";
    const value = String(getBindingValue(data, "command") ?? data.liveValue ?? "");

    return {
      bodyHtml: renderTemplate(template, {
        input_type: inputType,
        input_step: inputStep,
        value,
        data_type: String(source?.dataType ?? "Text"),
      }),
      footerLines: ["Write input"],
    };
  },
  bind: (context) => {
    const input = context.query<HTMLInputElement>('input[data-widget-input="typed"]');
    if (!input) return;

    const commit = () => {
      const source = context.data.bindings?.command?.[0] ?? context.data.sourceNode;
      const sourceType = String(source?.dataType ?? "Text").toLowerCase();
      const raw = input.value;

      if (sourceType === "integer" || sourceType === "float") {
        const num = Number(raw);
        if (!Number.isFinite(num)) return;
        context.writeBinding("command", sourceType === "integer" ? Math.round(num) : num);
        return;
      }

      context.writeBinding("command", raw);
    };

    context.listen(input, "input", () => {
      context.debounce("typed-command", 300, commit);
    });

    context.listen(input, "keydown", (event) => {
      if (!(event instanceof KeyboardEvent)) return;
      if (event.key !== "Enter") return;
      commit();
      input.blur();
    });

    context.listen(input, "blur", () => {
      commit();
    });
  },
});
