import type { TagScalarValue } from "$lib/core/ws/types";

export function toNumeric(value: TagScalarValue | undefined): number | null {
  if (typeof value === "number" && Number.isFinite(value)) {
    return value;
  }
  if (typeof value === "boolean") {
    return value ? 100 : 0;
  }
  if (typeof value === "string") {
    const parsed = Number(value);
    if (Number.isFinite(parsed)) {
      return parsed;
    }
  }
  return null;
}

export function toPercent(value: TagScalarValue | undefined): number {
  const numeric = toNumeric(value);
  if (numeric === null) {
    return 0;
  }
  const scaled = Math.abs(numeric) <= 1 ? numeric * 100 : numeric;
  return Math.max(0, Math.min(100, Math.round(scaled)));
}

export function toBoolean(value: TagScalarValue | undefined): boolean {
  if (typeof value === "boolean") {
    return value;
  }
  if (typeof value === "number") {
    return Number.isFinite(value) && value !== 0;
  }
  if (typeof value === "string") {
    const normalized = value.trim().toLowerCase();
    return (
      normalized === "true" ||
      normalized === "1" ||
      normalized === "on" ||
      normalized === "yes"
    );
  }
  return false;
}

export function toText(value: TagScalarValue | undefined): string {
  return value === undefined ? "--" : String(value);
}
