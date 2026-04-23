import { describe, expect, it } from "vitest";
import {
  clearWidgetHandlers,
  getWidgetHandlers,
  registerWidgetHandlers,
  unregisterWidgetHandlers,
} from "./widget-handlers";

describe("widget-handlers", () => {
  it("unregisters handlers by symbol id", () => {
    clearWidgetHandlers();
    registerWidgetHandlers("asset-1", {
      onWriteValue: () => {},
    });
    expect(getWidgetHandlers("asset-1")).toBeDefined();
    unregisterWidgetHandlers("asset-1");
    expect(getWidgetHandlers("asset-1")).toBeUndefined();
  });
});
