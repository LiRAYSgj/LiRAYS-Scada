import { z } from "zod";
import type { FormErrors, FormField, FormValues } from "./types";

/**
 * Pure helpers that turn a plugin form descriptor into editable form values,
 * validate them, and produce the configuration the backend expects.
 *
 * Form values keep UI-friendly shapes: `json` fields hold their text and
 * `secret` fields start empty because the backend never returns them.
 */

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isBlank(value: unknown): boolean {
  return (
    value === undefined ||
    value === null ||
    (typeof value === "number" && Number.isNaN(value)) ||
    (typeof value === "string" && value.trim() === "")
  );
}

function parseJsonObject(text: string): Record<string, unknown> | null {
  try {
    const parsed: unknown = JSON.parse(text);
    return isRecord(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

export function isFieldVisible(
  field: FormField,
  siblings: FormValues,
): boolean {
  const rule = field.visible_when;
  if (!rule) {
    return true;
  }
  return rule.one_of.some((value) => value === siblings[rule.field]);
}

/** Editable values for `fields`: stored values from `source`, otherwise defaults. */
export function createFormValues(
  fields: FormField[],
  source: FormValues = {},
): FormValues {
  const values: FormValues = {};
  for (const field of fields) {
    const stored = source[field.key];
    switch (field.type) {
      case "group":
        values[field.key] = createFormValues(
          field.fields ?? [],
          isRecord(stored)
            ? stored
            : isRecord(field.default)
              ? field.default
              : {},
        );
        break;
      case "list": {
        const items = Array.isArray(stored)
          ? stored
          : Array.isArray(field.default)
            ? field.default
            : [];
        values[field.key] = items.map((item) =>
          createFormValues(field.fields ?? [], isRecord(item) ? item : {}),
        );
        break;
      }
      case "tag_list": {
        const items = Array.isArray(stored)
          ? stored
          : Array.isArray(field.default)
            ? field.default
            : [];
        values[field.key] = items.map(String);
        break;
      }
      case "json": {
        const value = stored !== undefined ? stored : field.default;
        values[field.key] =
          value === undefined ? "" : JSON.stringify(value, null, 2);
        break;
      }
      case "secret":
        values[field.key] = "";
        break;
      case "boolean":
        values[field.key] = Boolean(stored ?? field.default ?? false);
        break;
      default:
        values[field.key] = stored !== undefined ? stored : field.default;
    }
  }
  return values;
}

function leafSchema(field: FormField): z.ZodType {
  switch (field.type) {
    case "number": {
      let schema = z.number({ error: "Must be a number" });
      if (field.integer) {
        schema = schema.int({ error: "Must be a whole number" });
      }
      if (field.min !== undefined) {
        schema = schema.min(field.min, {
          error: `Must be at least ${field.min}`,
        });
      }
      if (field.max !== undefined) {
        schema = schema.max(field.max, {
          error: `Must be at most ${field.max}`,
        });
      }
      return schema;
    }
    case "select": {
      const allowed = (field.options ?? []).map((option) => option.value);
      return z
        .string()
        .refine((value) => allowed.includes(value), {
          error: "Choose a valid option",
        });
    }
    case "boolean":
      return z.boolean();
    case "json":
      return z
        .string()
        .refine((text) => parseJsonObject(text) !== null, {
          error: "Must be a JSON object",
        });
    default:
      return z.string({ error: "Must be text" });
  }
}

export interface ValidateOptions {
  /** Empty secrets keep their stored value when editing an existing instance. */
  keepEmptySecrets?: boolean;
}

/** Validates visible fields; errors are keyed by dotted paths such as `variables.0.path`. */
export function validateFormValues(
  fields: FormField[],
  values: FormValues,
  options: ValidateOptions = {},
  prefix = "",
): FormErrors {
  const errors: FormErrors = {};
  for (const field of fields) {
    if (!isFieldVisible(field, values)) {
      continue;
    }
    const path = prefix ? `${prefix}.${field.key}` : field.key;
    const value = values[field.key];

    if (field.type === "group") {
      Object.assign(
        errors,
        validateFormValues(
          field.fields ?? [],
          isRecord(value) ? value : {},
          options,
          path,
        ),
      );
      continue;
    }

    if (field.type === "list" || field.type === "tag_list") {
      const items = Array.isArray(value) ? value : [];
      const minItems = field.min_items ?? (field.required ? 1 : 0);
      if (items.length < minItems) {
        errors[path] =
          minItems === 1
            ? "Add at least one entry"
            : `Add at least ${minItems} entries`;
      }
      items.forEach((item, index) => {
        if (field.type === "tag_list") {
          if (typeof item !== "string" || item.trim() === "") {
            errors[`${path}.${index}`] = "Must not be empty";
          }
          return;
        }
        Object.assign(
          errors,
          validateFormValues(
            field.fields ?? [],
            isRecord(item) ? item : {},
            options,
            `${path}.${index}`,
          ),
        );
      });
      continue;
    }

    if (isBlank(value)) {
      const keepsStoredSecret =
        field.type === "secret" && options.keepEmptySecrets;
      if (field.required && !keepsStoredSecret) {
        errors[path] = "Required";
      }
      continue;
    }

    const result = leafSchema(field).safeParse(value);
    if (!result.success) {
      errors[path] = result.error.issues[0]?.message ?? "Invalid value";
    }
  }
  return errors;
}

/**
 * The configuration to submit: only declared, visible and non-empty fields, so
 * plugins that reject unknown keys accept it.
 */
export function formValuesToConfig(
  fields: FormField[],
  values: FormValues,
): FormValues {
  const config: FormValues = {};
  for (const field of fields) {
    if (!isFieldVisible(field, values)) {
      continue;
    }
    const value = values[field.key];
    switch (field.type) {
      case "group": {
        const nested = formValuesToConfig(
          field.fields ?? [],
          isRecord(value) ? value : {},
        );
        if (field.required || Object.keys(nested).length > 0) {
          config[field.key] = nested;
        }
        break;
      }
      case "list":
        config[field.key] = (Array.isArray(value) ? value : []).map((item) =>
          formValuesToConfig(field.fields ?? [], isRecord(item) ? item : {}),
        );
        break;
      case "tag_list": {
        const items = (Array.isArray(value) ? value : [])
          .map((item) => String(item).trim())
          .filter((item) => item !== "");
        if (field.required || items.length > 0) {
          config[field.key] = items;
        }
        break;
      }
      case "json": {
        const parsed =
          typeof value === "string" ? parseJsonObject(value) : null;
        if (parsed) {
          config[field.key] = parsed;
        }
        break;
      }
      case "boolean":
        config[field.key] = Boolean(value);
        break;
      case "number":
        if (!isBlank(value)) {
          config[field.key] = Number(value);
        }
        break;
      case "tag":
        if (!isBlank(value)) {
          config[field.key] = String(value).trim();
        }
        break;
      default:
        if (!isBlank(value)) {
          config[field.key] = value;
        }
    }
  }
  return config;
}

/**
 * The `config` for an update request. The backend merges objects recursively and
 * removes keys set to `null`, so keys dropped from the form are sent as `null`.
 * Arrays are replaced as a whole.
 */
export function buildConfigPatch(
  previous: unknown,
  next: FormValues,
): FormValues {
  const patch: FormValues = {};
  const before = isRecord(previous) ? previous : {};
  for (const [key, value] of Object.entries(next)) {
    patch[key] =
      isRecord(value) && isRecord(before[key])
        ? buildConfigPatch(before[key], value)
        : value;
  }
  for (const key of Object.keys(before)) {
    if (!(key in next)) {
      patch[key] = null;
    }
  }
  return patch;
}

export { parseJsonObject };
