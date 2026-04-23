import { describe, expect, it } from "vitest";
import { ItemType, VarDataType } from "@lirays/scada-proto";
import { addTreeItemSchema, editTreeMetaSchema } from "./tree-schemas";

describe("tree-schemas min/max validation", () => {
  it("rejects Add Variable payload when max is lower than min for numeric types", () => {
    const result = addTreeItemSchema.safeParse({
      name: "Pressure",
      kind: ItemType.ITEM_TYPE_VARIABLE,
      dataType: VarDataType.VAR_DATA_TYPE_FLOAT,
      unit: "bar",
      min: 20,
      max: 10,
      options: "",
      maxLen: undefined,
    });

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.issues.some((issue) => issue.path[0] === "max")).toBe(
        true,
      );
    }
  });

  it("rejects Edit Variable Metadata payload when max is lower than min for numeric types", () => {
    const result = editTreeMetaSchema.safeParse({
      varId: "Area.Line.Pressure",
      dataType: "VAR_DATA_TYPE_INTEGER",
      unit: "bar",
      min: 50,
      max: 10,
      options: "",
      maxLen: undefined,
    });

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.issues.some((issue) => issue.path[0] === "max")).toBe(
        true,
      );
    }
  });

  it("allows equal min and max", () => {
    const result = addTreeItemSchema.safeParse({
      name: "Setpoint",
      kind: ItemType.ITEM_TYPE_VARIABLE,
      dataType: VarDataType.VAR_DATA_TYPE_INTEGER,
      unit: "",
      min: 42,
      max: 42,
      options: "",
      maxLen: undefined,
    });

    expect(result.success).toBe(true);
  });
});
