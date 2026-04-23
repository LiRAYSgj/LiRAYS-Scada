import { describe, expect, it } from "vitest";
import type { TreeNode } from "$lib/features/tree/types";
import type { GraphAssetDefinition } from "./assets/types";
import {
  acceptsPrimaryBindingType,
  filterAssetDefinitionsByPrimaryType,
  isNodeCompatibleWithBinding,
  normalizeVarDataType,
} from "./widget-type-compat";

const tagNode = (dataType?: string): TreeNode => ({
  id: "tag-1",
  parentId: "root",
  name: "Tag 1",
  path: "/root/tag-1",
  kind: "tag",
  hasChildren: false,
  childIds: null,
  dataType,
});

const folderNode = (): TreeNode => ({
  id: "folder-1",
  parentId: "root",
  name: "Folder 1",
  path: "/root/folder-1",
  kind: "folder",
  hasChildren: true,
  childIds: [],
});

const numericWidget: GraphAssetDefinition = {
  name: "numeric",
  pluginId: "test",
  label: "Numeric",
  runtime: { kind: "custom-element", tagName: "x-numeric" },
  primaryBindingKey: "value",
  bindings: [
    {
      key: "value",
      label: "Value",
      access: "read",
      dataTypes: ["VAR_DATA_TYPE_INTEGER", "VAR_DATA_TYPE_FLOAT"],
    },
  ],
};

const boolWidget: GraphAssetDefinition = {
  name: "bool",
  pluginId: "test",
  label: "Bool",
  runtime: { kind: "custom-element", tagName: "x-bool" },
  primaryBindingKey: "state",
  bindings: [
    {
      key: "state",
      label: "State",
      access: "read",
      dataTypes: ["VAR_DATA_TYPE_BOOLEAN"],
    },
  ],
};

const untypedWidget: GraphAssetDefinition = {
  name: "untyped",
  pluginId: "test",
  label: "Untyped",
  runtime: { kind: "custom-element", tagName: "x-untyped" },
  primaryBindingKey: "value",
  bindings: [{ key: "value", label: "Value", access: "read" }],
};

const nonBindableWidget: GraphAssetDefinition = {
  name: "non-bindable",
  pluginId: "test",
  label: "Non-bindable",
  runtime: { kind: "custom-element", tagName: "x-non-bindable" },
  primaryBindingKey: "",
  bindings: [],
};

describe("normalizeVarDataType", () => {
  it("normalizes compact enum suffixes", () => {
    expect(normalizeVarDataType("float")).toBe("VAR_DATA_TYPE_FLOAT");
    expect(normalizeVarDataType("BOOLEAN")).toBe("VAR_DATA_TYPE_BOOLEAN");
  });

  it("accepts already-prefixed values", () => {
    expect(normalizeVarDataType("VAR_DATA_TYPE_TEXT")).toBe(
      "VAR_DATA_TYPE_TEXT",
    );
  });

  it("rejects unknown values", () => {
    expect(normalizeVarDataType("UNRECOGNIZED")).toBeUndefined();
    expect(normalizeVarDataType(undefined)).toBeUndefined();
  });
});

describe("isNodeCompatibleWithBinding", () => {
  it("allows folder nodes regardless of binding types", () => {
    expect(
      isNodeCompatibleWithBinding(folderNode(), numericWidget.bindings[0]),
    ).toBe(true);
  });

  it("allows any tag type when binding has no dataTypes", () => {
    expect(
      isNodeCompatibleWithBinding(tagNode("VAR_DATA_TYPE_TEXT"), undefined),
    ).toBe(true);
    expect(
      isNodeCompatibleWithBinding(
        tagNode("VAR_DATA_TYPE_TEXT"),
        untypedWidget.bindings[0],
      ),
    ).toBe(true);
  });

  it("matches tag type against binding dataTypes", () => {
    expect(
      isNodeCompatibleWithBinding(
        tagNode("VAR_DATA_TYPE_FLOAT"),
        numericWidget.bindings[0],
      ),
    ).toBe(true);
    expect(
      isNodeCompatibleWithBinding(
        tagNode("VAR_DATA_TYPE_BOOLEAN"),
        numericWidget.bindings[0],
      ),
    ).toBe(false);
  });
});

describe("primary binding filtering", () => {
  it("filters assets by primary binding compatibility for tags", () => {
    const filtered = filterAssetDefinitionsByPrimaryType(
      [numericWidget, boolWidget, untypedWidget, nonBindableWidget],
      tagNode("VAR_DATA_TYPE_BOOLEAN"),
    );
    expect(filtered.map((item) => item.name)).toEqual(["bool", "untyped"]);
  });

  it("accepts all assets for folder drops", () => {
    const filtered = filterAssetDefinitionsByPrimaryType(
      [numericWidget, boolWidget, untypedWidget],
      folderNode(),
    );
    expect(filtered.map((item) => item.name)).toEqual([
      "numeric",
      "bool",
      "untyped",
    ]);
  });

  it("exposes single-definition check via acceptsPrimaryBindingType", () => {
    expect(
      acceptsPrimaryBindingType(boolWidget, tagNode("VAR_DATA_TYPE_BOOLEAN")),
    ).toBe(true);
    expect(
      acceptsPrimaryBindingType(boolWidget, tagNode("VAR_DATA_TYPE_FLOAT")),
    ).toBe(false);
    expect(
      acceptsPrimaryBindingType(
        nonBindableWidget,
        tagNode("VAR_DATA_TYPE_FLOAT"),
      ),
    ).toBe(false);
  });
});
