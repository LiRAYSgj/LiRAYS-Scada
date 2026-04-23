import { describe, expect, it } from "vitest";
import type { PlantAssetNodeData } from "$lib/features/graph/assets/types";
import {
  getBindingNumericRange,
  resolveNumericRange,
  toPercentInRange,
} from "./widget-runtime";

describe("resolveNumericRange", () => {
  it("uses default 0-100 when both min and max are missing", () => {
    expect(resolveNumericRange(undefined, undefined)).toEqual({
      min: 0,
      max: 100,
    });
  });

  it("uses min and derives max as min + 100 when max is missing", () => {
    expect(resolveNumericRange(20, undefined)).toEqual({ min: 20, max: 120 });
  });

  it("uses max and derives min as max - 100 when min is missing", () => {
    expect(resolveNumericRange(undefined, 80)).toEqual({ min: -20, max: 80 });
  });

  it("keeps provided min/max and normalizes order", () => {
    expect(resolveNumericRange(10, 90)).toEqual({ min: 10, max: 90 });
    expect(resolveNumericRange(90, 10)).toEqual({ min: 10, max: 90 });
  });
});

describe("toPercentInRange", () => {
  it("maps numeric value into clamped percentage", () => {
    expect(toPercentInRange(50, { min: 0, max: 100 })).toBe(50);
    expect(toPercentInRange(200, { min: 0, max: 100 })).toBe(100);
    expect(toPercentInRange(-20, { min: 0, max: 100 })).toBe(0);
  });
});

describe("getBindingNumericRange", () => {
  it("resolves from first binding tag metadata", () => {
    const data = {
      assetKind: "slider",
      title: "Test",
      sourceNode: { id: "tag-1", name: "Tag", path: "/Tag", kind: "tag" },
      bindings: {
        command: [
          {
            id: "tag-1",
            name: "Tag",
            path: "/Tag",
            kind: "tag",
            min: 10,
            max: 40,
          },
        ],
      },
    } as PlantAssetNodeData;

    expect(getBindingNumericRange(data, "command")).toEqual({
      min: 10,
      max: 40,
    });
  });

  it("falls back to default when no metadata is present", () => {
    const data = {
      assetKind: "slider",
      title: "Test",
      sourceNode: { id: "tag-1", name: "Tag", path: "/Tag", kind: "tag" },
      bindings: {
        command: [
          {
            id: "tag-1",
            name: "Tag",
            path: "/Tag",
            kind: "tag",
          },
        ],
      },
    } as PlantAssetNodeData;

    expect(getBindingNumericRange(data, "command")).toEqual({
      min: 0,
      max: 100,
    });
  });
});
