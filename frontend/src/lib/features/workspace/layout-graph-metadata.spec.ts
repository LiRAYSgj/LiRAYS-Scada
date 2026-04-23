import { describe, expect, it } from "vitest";
import type { Node } from "@xyflow/svelte";
import type { TreeNode } from "$lib/features/tree/types";
import {
  applyBoundMetadata,
  applyTreeSnapshotMetadata,
  applyVarMetaChanged,
  getParentIdFromNodePath,
} from "./layout-graph-metadata";

function makeTagNode(
  id: string,
  overrides: Record<string, unknown> = {},
): TreeNode {
  return {
    id,
    parentId: "/Area",
    name: id,
    path: `/Area/${id}`,
    kind: "tag",
    hasChildren: false,
    childIds: null,
    ...overrides,
  } as TreeNode;
}

function makeGraphNode(): Node {
  return {
    id: "asset-1",
    type: "plantAsset",
    position: { x: 0, y: 0 },
    data: {
      title: "Asset",
      assetKind: "pump",
      sourceNode: {
        id: "tag-src",
        name: "Source",
        path: "/Area/tag-src",
        kind: "tag",
        min: 0,
        max: 100,
        unit: "psi",
        options: ["A"],
      },
      bindings: {
        command: [
          {
            id: "tag-1",
            name: "Tag 1",
            path: "/Area/tag-1",
            kind: "tag",
            min: 1,
            max: 2,
            unit: "bar",
            dataType: "float",
            options: ["X"],
          },
        ],
      },
    },
  } as Node;
}

describe("layout-graph-metadata", () => {
  it("applies var meta changes to both bindings and source node", () => {
    const node = makeGraphNode();
    const result = applyVarMetaChanged([node], [
      { varId: "tag-1", min: 10, max: 20 },
      { varId: "tag-src", min: -5, max: 5 },
    ] as never[]);
    expect(result.changed).toBe(true);
    const updated = result.nodes[0].data as Record<string, any>;
    expect(updated.bindings.command[0].min).toBe(10);
    expect(updated.bindings.command[0].max).toBe(20);
    expect(updated.sourceNode.min).toBe(-5);
    expect(updated.sourceNode.max).toBe(5);
  });

  it("returns unchanged when no relevant var meta is provided", () => {
    const node = makeGraphNode();
    const result = applyVarMetaChanged([node], [
      { varId: "unrelated", min: 2, max: 3 },
    ] as never[]);
    expect(result.changed).toBe(false);
    expect(result.nodes).toBeTruthy();
  });

  it("applies metadata from tree snapshot including options/dataType", () => {
    const node = makeGraphNode();
    const treeNodes: Record<string, TreeNode> = {
      "tag-1": makeTagNode("tag-1", {
        min: 7,
        max: 9,
        unit: "kPa",
        dataType: "int",
        options: ["ON", "OFF"],
      }),
      "tag-src": makeTagNode("tag-src", {
        min: -1,
        max: 1,
        unit: "%",
        dataType: "float",
        options: ["LOW", "HIGH"],
      }),
    };
    const result = applyTreeSnapshotMetadata([node], treeNodes);
    expect(result.changed).toBe(true);
    const updated = result.nodes[0].data as Record<string, any>;
    expect(updated.bindings.command[0].unit).toBe("kPa");
    expect(updated.bindings.command[0].options).toEqual(["ON", "OFF"]);
    expect(updated.sourceNode.unit).toBe("%");
    expect(updated.sourceNode.options).toEqual(["LOW", "HIGH"]);
  });

  it("applies backend metadata map updates", () => {
    const node = makeGraphNode();
    const map = new Map([
      ["tag-1", { min: 3, max: 4, unit: "amp", options: ["A", "B"] }],
      ["tag-src", { min: 8, max: 9, unit: "v", options: ["H"] }],
    ]);
    const result = applyBoundMetadata([node], map);
    expect(result.changed).toBe(true);
    const updated = result.nodes[0].data as Record<string, any>;
    expect(updated.bindings.command[0].min).toBe(3);
    expect(updated.bindings.command[0].unit).toBe("amp");
    expect(updated.sourceNode.max).toBe(9);
    expect(updated.sourceNode.options).toEqual(["H"]);
  });

  it("resolves parent id from path-like ids", () => {
    expect(getParentIdFromNodePath("/Area/Line/Tag")).toBe("/Area/Line");
    expect(getParentIdFromNodePath("/Tag")).toBeNull();
    expect(getParentIdFromNodePath("/")).toBeNull();
    expect(getParentIdFromNodePath("")).toBeNull();
    expect(getParentIdFromNodePath("/Area/Line/")).toBe("/Area");
  });
});
