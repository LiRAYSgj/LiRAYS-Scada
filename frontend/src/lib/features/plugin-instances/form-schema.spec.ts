import { describe, expect, it } from "vitest";
import {
  buildConfigPatch,
  createFormValues,
  formValuesToConfig,
  isFieldVisible,
  validateFormValues,
} from "./form-schema";
import type { FormField } from "./types";

const periodic = ["sine", "square", "ramp"];

const simulatorFields: FormField[] = [
  {
    key: "interval_ms",
    label: "Interval",
    type: "number",
    integer: true,
    min: 1,
    required: true,
    default: 1000,
  },
  {
    key: "variables",
    label: "Variables",
    type: "list",
    required: true,
    min_items: 1,
    fields: [
      { key: "path", label: "Path", type: "tag", required: true },
      {
        key: "waveform",
        label: "Waveform",
        type: "select",
        required: true,
        default: "random",
        options: [
          { value: "random", label: "Random" },
          { value: "sine", label: "Sine" },
        ],
      },
      {
        key: "period_ms",
        label: "Period",
        type: "number",
        integer: true,
        min: 1,
        required: true,
        default: 10000,
        visible_when: { field: "waveform", one_of: periodic },
      },
      {
        key: "decimal_places",
        label: "Decimals",
        type: "number",
        integer: true,
        min: 0,
        max: 15,
      },
    ],
  },
];

const outboundFields: FormField[] = [
  { key: "variables", label: "Variables", type: "tag_list", required: true },
  {
    key: "mode",
    label: "Mode",
    type: "group",
    fields: [
      {
        key: "type",
        label: "Type",
        type: "select",
        required: true,
        default: "subscription",
        options: [
          { value: "subscription", label: "On change" },
          { value: "polling", label: "Polling" },
        ],
      },
      {
        key: "interval_ms",
        label: "Interval",
        type: "number",
        required: true,
        default: 1000,
        visible_when: { field: "type", one_of: ["polling"] },
      },
    ],
  },
  {
    key: "target",
    label: "Target",
    type: "group",
    required: true,
    fields: [
      { key: "host", label: "Host", type: "text", required: true },
      { key: "password", label: "Password", type: "secret", required: true },
    ],
  },
];

describe("plugin form schema", () => {
  it("creates defaults and overlays stored values", () => {
    expect(createFormValues(simulatorFields)).toEqual({
      interval_ms: 1000,
      variables: [],
    });
    const values = createFormValues(simulatorFields, {
      variables: [{ path: "/a", waveform: "sine", period_ms: 500 }],
    });
    expect(values.variables).toEqual([
      {
        path: "/a",
        waveform: "sine",
        period_ms: 500,
        decimal_places: undefined,
      },
    ]);
  });

  it("evaluates visibility against sibling values", () => {
    const period = simulatorFields[1].fields![2];
    expect(isFieldVisible(period, { waveform: "random" })).toBe(false);
    expect(isFieldVisible(period, { waveform: "sine" })).toBe(true);
  });

  it("drops hidden and empty fields from the submitted configuration", () => {
    const values = createFormValues(outboundFields);
    values.variables = ["/plant/**", " "];
    (values.target as Record<string, unknown>).host = "127.0.0.1";
    expect(formValuesToConfig(outboundFields, values)).toEqual({
      variables: ["/plant/**"],
      mode: { type: "subscription" },
      target: { host: "127.0.0.1" },
    });

    const simulator = createFormValues(simulatorFields, {
      variables: [{ path: " /a ", waveform: "random" }],
    });
    expect(formValuesToConfig(simulatorFields, simulator)).toEqual({
      interval_ms: 1000,
      variables: [{ path: "/a", waveform: "random" }],
    });
  });

  it("validates required, ranges and nested list items with dotted paths", () => {
    const values = createFormValues(simulatorFields);
    expect(validateFormValues(simulatorFields, values)).toEqual({
      variables: "Add at least one entry",
    });

    values.interval_ms = 0.5;
    values.variables = [
      { path: "", waveform: "sine", period_ms: null, decimal_places: 20 },
    ];
    const errors = validateFormValues(simulatorFields, values);
    expect(errors.interval_ms).toBe("Must be a whole number");
    expect(errors["variables.0.path"]).toBe("Required");
    expect(errors["variables.0.period_ms"]).toBe("Required");
    expect(errors["variables.0.decimal_places"]).toBe("Must be at most 15");
  });

  it("only lets empty secrets through when editing", () => {
    const values = createFormValues(outboundFields, {
      variables: ["/a"],
      target: { host: "h", password: "ignored" },
    });
    expect((values.target as Record<string, unknown>).password).toBe("");
    expect(validateFormValues(outboundFields, values)["target.password"]).toBe(
      "Required",
    );
    expect(
      validateFormValues(outboundFields, values, { keepEmptySecrets: true }),
    ).toEqual({});
    expect(formValuesToConfig(outboundFields, values).target).toEqual({
      host: "h",
    });
  });

  it("sends null for removed keys and replaces arrays", () => {
    const previous = {
      interval_ms: 1000,
      mode: { type: "polling", interval_ms: 500 },
      variables: ["/a", "/b"],
      legacy: true,
    };
    const next = {
      interval_ms: 1000,
      mode: { type: "subscription" },
      variables: ["/a"],
    };
    expect(buildConfigPatch(previous, next)).toEqual({
      interval_ms: 1000,
      mode: { type: "subscription", interval_ms: null },
      variables: ["/a"],
      legacy: null,
    });
  });

  it("round-trips raw JSON fields", () => {
    const fields: FormField[] = [
      { key: "target", label: "Target", type: "json", default: {} },
    ];
    const values = createFormValues(fields, { target: { a: 1 } });
    expect(values.target).toBe('{\n  "a": 1\n}');
    expect(formValuesToConfig(fields, values)).toEqual({ target: { a: 1 } });
    values.target = "[1]";
    expect(validateFormValues(fields, values)).toEqual({
      target: "Must be a JSON object",
    });
  });
});
