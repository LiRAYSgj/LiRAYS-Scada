import { describe, expect, it } from "vitest";
import {
  duplicateThresholdIndexes,
  nextThresholdValue,
  readThresholds,
  resolveThreshold,
  thresholdBands,
} from "$lib/scada/plugins/thresholds";
import type { WidgetConfigThresholds } from "$lib/scada/plugins/types";

const fallback: WidgetConfigThresholds = {
  default: { color: "#ef4444", label: "" },
  levels: [
    { value: 25, color: "#3b82f6", label: "" },
    { value: 60, color: "#546a0e", label: "" },
  ],
};

describe("resolveThreshold", () => {
  const config: WidgetConfigThresholds = {
    default: { color: "#ff0000", label: "High" },
    levels: [
      { value: 60, color: "#00ff00", label: "Normal" },
      { value: 25, color: "#0000ff", label: "Low" },
    ],
  };

  it("uses the lowest threshold the value is <= to, regardless of order", () => {
    expect(resolveThreshold(-5, config)?.color).toBe("#0000ff");
    expect(resolveThreshold(25, config)?.color).toBe("#0000ff");
    expect(resolveThreshold(25.1, config)?.color).toBe("#00ff00");
    expect(resolveThreshold(60, config)?.label).toBe("Normal");
  });

  it("uses the default color above the highest threshold", () => {
    expect(resolveThreshold(60.5, config)).toEqual({
      color: "#ff0000",
      label: "High",
    });
  });

  it("uses the default color when there are no thresholds", () => {
    expect(
      resolveThreshold(1, { default: { color: "#123456" }, levels: [] })?.color,
    ).toBe("#123456");
  });

  it("returns null without a value", () => {
    expect(resolveThreshold(null, config)).toBeNull();
  });
});

describe("readThresholds", () => {
  it("returns the fallback for missing config", () => {
    expect(readThresholds(undefined, fallback)).toEqual(fallback);
  });

  it("parses and sorts the current format, dropping invalid levels", () => {
    expect(
      readThresholds(
        {
          default: { color: "#111111" },
          levels: [
            { value: 80, color: "#222222", label: "b" },
            { value: "x", color: "#333333" },
            { value: 10, color: "#444444" },
          ],
        },
        fallback,
      ),
    ).toEqual({
      default: { color: "#111111", label: "" },
      levels: [
        { value: 10, color: "#444444", label: "" },
        { value: 80, color: "#222222", label: "b" },
      ],
    });
  });

  it("migrates legacy min/max ranges", () => {
    expect(
      readThresholds(
        [
          { max: 25, color: "#3b82f6", label: "" },
          { min: 60, max: 80, color: "#f59e0b", label: "Warn" },
          { min: 25, max: 60, color: "#546a0e", label: "" },
          { min: 80, color: "#ef4444", label: "Alarm" },
        ],
        fallback,
      ),
    ).toEqual({
      default: { color: "#ef4444", label: "Alarm" },
      levels: [
        { value: 25, color: "#3b82f6", label: "" },
        { value: 60, color: "#546a0e", label: "" },
        { value: 80, color: "#f59e0b", label: "Warn" },
      ],
    });
  });
});

describe("editor helpers", () => {
  it("flags repeated values after the first occurrence", () => {
    expect([
      ...duplicateThresholdIndexes([
        { value: 1, color: "" },
        { value: 2, color: "" },
        { value: 1, color: "" },
      ]),
    ]).toEqual([2]);
  });

  it("proposes a value above the highest threshold", () => {
    expect(nextThresholdValue([])).toBe(0);
    expect(nextThresholdValue(fallback.levels)).toBe(70);
  });
});

describe("thresholdBands", () => {
  const config: WidgetConfigThresholds = {
    default: { color: "#ff0000", label: "High" },
    levels: [
      { value: 75, color: "#00ff00", label: "Normal" },
      { value: 20, color: "#0000ff", label: "Low" },
    ],
  };

  it("splits the range in ascending bands ending with the default", () => {
    expect(thresholdBands(config, { min: 0, max: 100 })).toEqual([
      { from: 0, to: 20, color: "#0000ff", label: "Low" },
      { from: 20, to: 75, color: "#00ff00", label: "Normal" },
      { from: 75, to: 100, color: "#ff0000", label: "High" },
    ]);
  });

  it("clips levels outside the range and drops empty bands", () => {
    expect(thresholdBands(config, { min: 30, max: 60 })).toEqual([
      { from: 30, to: 60, color: "#00ff00", label: "Normal" },
    ]);
    expect(thresholdBands(config, { min: 80, max: 90 })).toEqual([
      { from: 80, to: 90, color: "#ff0000", label: "High" },
    ]);
    expect(thresholdBands(config, { min: -10, max: 20 })).toEqual([
      { from: -10, to: 20, color: "#0000ff", label: "Low" },
    ]);
  });

  it("returns no bands without thresholds", () => {
    expect(
      thresholdBands({ ...config, levels: [] }, { min: 0, max: 100 }),
    ).toEqual([]);
  });
});
