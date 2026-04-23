import { describe, expect, it } from "vitest";
import { buildTreeSelectionSummary } from "./tree-selection-summary";
import type { TreeNode } from "./types";

function node(
  id: string,
  parentId: string | null,
  childIds: string[] | null = null,
): TreeNode {
  return {
    id,
    parentId,
    name: id,
    path: id,
    kind: "folder",
    hasChildren: (childIds?.length ?? 0) > 0,
    childIds,
  };
}

describe("tree-selection-summary", () => {
  it("marks node as indeterminate when only part of loaded descendants are selected", () => {
    const nodes: Record<string, TreeNode> = {
      root: node("root", null, ["a", "b"]),
      a: node("a", "root", null),
      b: node("b", "root", null),
    };
    const summary = buildTreeSelectionSummary(nodes, ["root"], new Set(["a"]));
    expect(summary.indeterminateIds.has("root")).toBe(true);
  });

  it("does not mark node as indeterminate when all descendants are selected", () => {
    const nodes: Record<string, TreeNode> = {
      root: node("root", null, ["a", "b"]),
      a: node("a", "root", null),
      b: node("b", "root", null),
    };
    const summary = buildTreeSelectionSummary(
      nodes,
      ["root"],
      new Set(["a", "b", "root"]),
    );
    expect(summary.indeterminateIds.has("root")).toBe(false);
  });

  it("counts loaded descendants and selected descendants for nested branches", () => {
    const nodes: Record<string, TreeNode> = {
      root: node("root", null, ["folder"]),
      folder: node("folder", "root", ["v1", "v2"]),
      v1: node("v1", "folder", null),
      v2: node("v2", "folder", null),
    };
    const summary = buildTreeSelectionSummary(
      nodes,
      ["root"],
      new Set(["folder", "v1"]),
    );
    expect(summary.loadedDescendantCountById.get("root")).toBe(3);
    expect(summary.selectedDescendantCountById.get("root")).toBe(2);
    expect(summary.indeterminateIds.has("root")).toBe(true);
    expect(summary.indeterminateIds.has("folder")).toBe(true);
  });
});
