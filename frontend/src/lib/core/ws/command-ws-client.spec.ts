import { describe, expect, it } from "vitest";
import { ItemType, VarDataType } from "@lirays/scada-proto";
import {
  createAddBulkCommand,
  createAddCommand,
  createCommandId,
  createDelCommand,
  createGetCommand,
  createListCommand,
  createSetCommand,
  createSingleItemMeta,
  createEditMetaCommand,
  fromBackendValue,
  namespaceJsonToSchema,
  toBackendValue,
} from "./command-ws-client";

describe("command-ws-client", () => {
  describe("createCommandId", () => {
    it("returns string with prefix and unique suffix", () => {
      const id = createCommandId("list");
      expect(id).toMatch(/^list-\d+-[a-f0-9]+$/);
    });
  });

  describe("toBackendValue", () => {
    it("maps string to textValue", () => {
      expect(toBackendValue("hello")).toEqual({ textValue: "hello" });
    });
    it("maps boolean to booleanValue", () => {
      expect(toBackendValue(true)).toEqual({ booleanValue: true });
    });
    it("maps integer to integerValue", () => {
      expect(toBackendValue(42)).toEqual({ integerValue: 42 });
    });
    it("maps float to floatValue", () => {
      expect(toBackendValue(3.14)).toEqual({ floatValue: 3.14 });
    });
  });

  describe("fromBackendValue", () => {
    it("returns undefined for null or undefined", () => {
      expect(fromBackendValue(null)).toBeUndefined();
      expect(fromBackendValue(undefined)).toBeUndefined();
    });
    it("extracts integerValue", () => {
      expect(fromBackendValue({ integerValue: 10 })).toBe(10);
    });
    it("extracts floatValue", () => {
      expect(fromBackendValue({ floatValue: 1.5 })).toBe(1.5);
    });
    it("extracts textValue", () => {
      expect(fromBackendValue({ textValue: "x" })).toBe("x");
    });
    it("extracts booleanValue", () => {
      expect(fromBackendValue({ booleanValue: false })).toBe(false);
    });
  });

  describe("createListCommand", () => {
    it("includes folderId when provided", () => {
      const { cmdId, command } = createListCommand("folder-1");
      expect(command.list?.folderId).toBe("folder-1");
      expect(cmdId).toBeDefined();
    });
    it("omits folderId when undefined (roots)", () => {
      const { command } = createListCommand(undefined);
      expect(command.list).toBeDefined();
      expect(command.list?.folderId).toBeUndefined();
    });
  });

  describe("createAddCommand", () => {
    it("sets parentId and itemsMeta", () => {
      const meta = createSingleItemMeta(
        "x",
        ItemType.ITEM_TYPE_FOLDER,
        undefined,
      );
      const { command } = createAddCommand("parent-id", [meta]);
      expect(command.add?.parentId).toBe("parent-id");
      expect(command.add?.itemsMeta).toHaveLength(1);
      expect(command.add?.itemsMeta?.[0].name).toBe("x");
    });
  });

  describe("createDelCommand", () => {
    it("sets itemIds array", () => {
      const { command } = createDelCommand(["id1", "id2"]);
      expect(command.del?.itemIds).toEqual(["id1", "id2"]);
    });
  });

  describe("createGetCommand", () => {
    it("sets varIds", () => {
      const { command } = createGetCommand(["a", "b"]);
      expect(command.get?.varIds).toEqual(["a", "b"]);
    });
  });

  describe("createSetCommand", () => {
    it("sets varIdsValues with toBackendValue", () => {
      const { command } = createSetCommand("tag-1", 100);
      expect(command.set?.varIdsValues).toHaveLength(1);
      expect(command.set?.varIdsValues?.[0].varId).toBe("tag-1");
      expect(command.set?.varIdsValues?.[0].value?.integerValue).toBe(100);
    });
  });

  describe("createAddBulkCommand", () => {
    it("sets parentId and schema", () => {
      const schema = namespaceJsonToSchema({ Area: { P: "Float" } });
      const { command } = createAddBulkCommand("", schema);
      expect(command.addBulk?.parentId).toBe("");
      expect(command.addBulk?.schema).toBe(schema);
    });
  });

  describe("namespaceJsonToSchema", () => {
    it("converts leaf string to variableType", () => {
      const schema = namespaceJsonToSchema({ P: "Float" });
      expect(schema.roots.P).toEqual({
        variable: {
          varDType: VarDataType.VAR_DATA_TYPE_FLOAT,
          unit: undefined,
          min: undefined,
          max: undefined,
          options: [],
          maxLen: undefined,
        },
      });
    });
    it("converts nested object to folder with children", () => {
      const schema = namespaceJsonToSchema({ Area: { P: "Float" } });
      expect(schema.roots.Area).toEqual({
        folder: {
          children: {
            P: {
              variable: {
                varDType: VarDataType.VAR_DATA_TYPE_FLOAT,
                unit: undefined,
                min: undefined,
                max: undefined,
                options: [],
                maxLen: undefined,
              },
            },
          },
        },
      });
    });
    it("converts variable metadata object to NamespaceVariable", () => {
      const schema = namespaceJsonToSchema({
        Temperature: {
          type: "Float",
          unit: "kW",
          min: 0,
          max: 100,
        },
        Label: {
          type: "Text",
          maxLength: 255,
          options: ["RUNNING", "STOPPED"],
        },
      });
      expect(schema.roots.Temperature).toEqual({
        variable: {
          varDType: VarDataType.VAR_DATA_TYPE_FLOAT,
          unit: "kW",
          min: 0,
          max: 100,
          options: [],
          maxLen: undefined,
        },
      });
      expect(schema.roots.Label).toEqual({
        variable: {
          varDType: VarDataType.VAR_DATA_TYPE_TEXT,
          unit: undefined,
          min: undefined,
          max: undefined,
          options: ["RUNNING", "STOPPED"],
          maxLen: 255,
        },
      });
    });
    it("throws on invalid metadata field types", () => {
      expect(() =>
        namespaceJsonToSchema({
          Temperature: { type: "Float", min: "0" as unknown as number },
        }),
      ).toThrow(/expected number/);
    });
    it("throws on invalid node type", () => {
      expect(() => namespaceJsonToSchema({ x: 123 } as any)).toThrow(
        /Invalid namespace node/,
      );
    });
  });

  describe("createSingleItemMeta", () => {
    it("returns meta with name, item type, and optional var type", () => {
      const meta = createSingleItemMeta(
        "node1",
        ItemType.ITEM_TYPE_VARIABLE,
        VarDataType.VAR_DATA_TYPE_FLOAT,
        { min: 0, max: 10, unit: "°C", maxLen: 4, options: ["x"] },
      );
      expect(meta.name).toBe("node1");
      expect(meta.iType).toBe(ItemType.ITEM_TYPE_VARIABLE);
      expect(meta.varDType).toBe(VarDataType.VAR_DATA_TYPE_FLOAT);
      expect(meta.min).toBe(0);
      expect(meta.max).toBe(10);
      expect(meta.unit).toBe("°C");
      expect(meta.maxLen).toEqual(4);
      expect(meta.options).toEqual(["x"]);
    });
  });

  describe("createEditMetaCommand", () => {
    it("builds editMeta envelope with optional fields", () => {
      const { command, cmdId } = createEditMetaCommand("var-1", {
        unit: "kPa",
        min: 1,
        max: 5,
        options: ["A", "B"],
        maxLen: 8,
      });
      expect(command.editMeta?.varId).toBe("var-1");
      expect(command.editMeta?.unit).toBe("kPa");
      expect(command.editMeta?.min).toBe(1);
      expect(command.editMeta?.max).toBe(5);
      expect(command.editMeta?.options).toEqual(["A", "B"]);
      expect(command.editMeta?.maxLen).toEqual(8);
      expect(cmdId).toMatch(/^edit-meta-/);
    });
  });
});
