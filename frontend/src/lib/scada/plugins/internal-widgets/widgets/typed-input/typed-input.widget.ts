import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  escapeHtml,
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
    bindings: [
      {
        key: "command",
        label: "Command",
        access: "readwrite",
        dataTypes: [
          "VAR_DATA_TYPE_INTEGER",
          "VAR_DATA_TYPE_FLOAT",
          "VAR_DATA_TYPE_TEXT",
        ],
      },
    ],
    capabilities: {
      inputWidget: true,
      keepAspectRatio: false,
      minHeight: 80,
    },
  },
  tagName: "lirays-widget-typed-input",
  styles,
  render: (data) => {
    const source = data.bindings?.command?.[0] ?? data.sourceNode;
    const sourceType = resolveSourceType(source?.dataType);
    const isNumeric = sourceType === "integer" || sourceType === "float";
    const hasOptions =
      sourceType === "text" &&
      Array.isArray(source?.options) &&
      source.options.length > 0;
    const inputStep =
      sourceType === "integer" ? "1" : sourceType === "float" ? "0.01" : "any";
    const value = String(
      getBindingValue(data, "command") ?? data.liveValue ?? "",
    );
    const numericValue = Number(value);
    const numberValue = Number.isFinite(numericValue)
      ? String(numericValue)
      : "";
    const optionsHtml = (source?.options ?? [])
      .map((option: string) => {
        const selected = option === value ? "selected" : "";
        const escaped = escapeHtml(option);
        return `<option value="${escaped}" ${selected}>${escaped}</option>`;
      })
      .join("");
    const bodyHtml = renderTemplate(template, {
      input_step: inputStep,
      number_value: numberValue,
      text_value: value,
      select_hidden_class: hasOptions ? "" : "hidden",
      text_hidden_class: !isNumeric && !hasOptions ? "" : "hidden",
      number_hidden_class: isNumeric ? "" : "hidden",
      options_html: optionsHtml,
    });

    return {
      bodyHtml,
      footerLines: ["Write input"],
    };
  },
  bind: (context) => {
    const textInput = context.query<HTMLInputElement>(
      'input[data-widget-input="typed-text"]',
    );
    const numberInput = context.query<HTMLInputElement>(
      'input[data-widget-input="typed-number"]',
    );
    const select = context.query<HTMLSelectElement>(
      'select[data-widget-input="typed-select"]',
    );
    const decreaseButton = context.query<HTMLButtonElement>(
      "button[data-widget-number-decrease]",
    );
    const increaseButton = context.query<HTMLButtonElement>(
      "button[data-widget-number-increase]",
    );

    const commit = () => {
      const source =
        context.data.bindings?.command?.[0] ?? context.data.sourceNode;
      const sourceType = resolveSourceType(source?.dataType);
      const hasOptions =
        sourceType === "text" &&
        Array.isArray(source?.options) &&
        source.options.length > 0;
      const isNumeric = sourceType === "integer" || sourceType === "float";
      const raw = isNumeric
        ? (numberInput?.value ?? "")
        : hasOptions && select
          ? select.value
          : (textInput?.value ?? "");

      if (isNumeric) {
        const num = Number(raw);
        if (!Number.isFinite(num)) return;
        context.writeBinding(
          "command",
          sourceType === "integer" ? Math.round(num) : num,
        );
        return;
      }

      context.writeBinding("command", raw);
    };

    if (textInput) {
      context.listen(textInput, "input", () => {
        context.debounce("typed-command", 300, commit);
      });

      context.listen(textInput, "keydown", (event) => {
        if (!(event instanceof KeyboardEvent)) return;
        if (event.key !== "Enter") return;
        commit();
        textInput.blur();
      });

      context.listen(textInput, "blur", () => {
        commit();
      });
    }

    if (numberInput) {
      context.listen(numberInput, "input", () => {
        context.debounce("typed-command", 250, commit);
      });

      context.listen(numberInput, "keydown", (event) => {
        if (!(event instanceof KeyboardEvent)) return;
        if (event.key !== "Enter") return;
        commit();
        numberInput.blur();
      });

      context.listen(numberInput, "blur", () => {
        commit();
      });
    }

    if (select) {
      context.listen(select, "change", () => {
        commit();
      });
    }

    const applyStep = (direction: 1 | -1): void => {
      const source =
        context.data.bindings?.command?.[0] ?? context.data.sourceNode;
      const sourceType = resolveSourceType(source?.dataType);
      if (
        !numberInput ||
        (sourceType !== "integer" && sourceType !== "float")
      ) {
        return;
      }
      const step = sourceType === "integer" ? 1 : 0.01;
      const base = Number(numberInput.value);
      const current = Number.isFinite(base) ? base : 0;
      const next = addWithPrecision(current, direction * step);
      numberInput.value =
        sourceType === "integer"
          ? String(Math.round(next))
          : formatWithPrecision(next, step);
      commit();
    };

    if (decreaseButton) {
      context.listen(decreaseButton, "click", () => {
        applyStep(-1);
      });
    }
    if (increaseButton) {
      context.listen(increaseButton, "click", () => {
        applyStep(1);
      });
    }
  },
});

function resolveSourceType(rawDataType: unknown): "integer" | "float" | "text" {
  const value = String(rawDataType ?? "").toLowerCase();
  if (value === "var_data_type_integer" || value === "integer") {
    return "integer";
  }
  if (value === "var_data_type_float" || value === "float") {
    return "float";
  }
  return "text";
}

function addWithPrecision(base: number, delta: number): number {
  const precision = Math.max(decimalPlaces(base), decimalPlaces(delta));
  if (precision <= 0) {
    return base + delta;
  }
  const factor = 10 ** Math.min(precision, 10);
  return Math.round((base + delta) * factor) / factor;
}

function formatWithPrecision(value: number, step: number): string {
  const precision = Math.min(Math.max(decimalPlaces(step), 0), 10);
  if (precision === 0) {
    return String(Math.round(value));
  }
  return value.toFixed(precision).replace(/\.?0+$/, "");
}

function decimalPlaces(value: number): number {
  if (!Number.isFinite(value)) return 0;
  const normalized = Math.abs(value).toString().toLowerCase();
  if (normalized.includes("e-")) {
    const [coefficient, exponentPart] = normalized.split("e-");
    const exponent = Number(exponentPart);
    const coefficientDecimals = coefficient.includes(".")
      ? coefficient.split(".")[1].length
      : 0;
    return exponent + coefficientDecimals;
  }
  if (!normalized.includes(".")) {
    return 0;
  }
  return normalized.split(".")[1].length;
}
