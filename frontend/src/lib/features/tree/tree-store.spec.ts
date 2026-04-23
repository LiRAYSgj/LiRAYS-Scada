import { get } from "svelte/store";
import { describe, expect, it, vi } from "vitest";
import { createTreeStore } from "./tree-store";
import type { TreeNode } from "./types";

function makeNode(
  id: string,
  parentId: string | null,
  hasChildren: boolean,
): TreeNode {
  return {
    id,
    parentId,
    name: id,
    path: id,
    kind: hasChildren ? "folder" : "tag",
    hasChildren,
    childIds: null,
  };
}

describe("createTreeStore", () => {
  it("initializes root nodes and marks state initialized", async () => {
    const roots = [
      makeNode("root-a", null, true),
      makeNode("root-b", null, false),
    ];
    const fetchChildren = vi.fn(async (parent: TreeNode | null) =>
      parent ? [] : roots,
    );
    const tree = createTreeStore({ fetchChildren });

    await tree.initialize();

    const state = get(tree.state);
    expect(fetchChildren).toHaveBeenCalledWith(null);
    expect(state.hasInitialized).toBe(true);
    expect(state.rootLoading).toBe(false);
    expect(state.rootIds).toEqual(["root-a", "root-b"]);
    expect(Object.keys(state.nodes)).toEqual(["root-a", "root-b"]);
  });

  it("loads children once on first expand and caches them", async () => {
    const child = makeNode("child-a", "root-a", false);
    const fetchChildren = vi.fn(async (parent: TreeNode | null) => {
      if (!parent) {
        return [makeNode("root-a", null, true)];
      }
      if (parent.id === "root-a") {
        return [child];
      }
      return [];
    });
    const tree = createTreeStore({ fetchChildren });
    await tree.initialize();

    tree.toggleExpanded("root-a");
    await Promise.resolve();
    await Promise.resolve();

    tree.toggleExpanded("root-a"); // collapse
    tree.toggleExpanded("root-a"); // expand again, should be instant/cached
    await Promise.resolve();

    expect(fetchChildren).toHaveBeenCalledTimes(2); // null + root-a once
    const rows = get(tree.visibleRows);
    expect(rows.map((r) => r.id)).toEqual(["root-a", "child-a"]);
  });

  it("updates selected and collapsed states", async () => {
    const tree = createTreeStore({
      fetchChildren: async (parent) =>
        parent ? [] : [makeNode("root-a", null, true)],
    });
    await tree.initialize();

    tree.selectNode("root-a");
    tree.toggleExpanded("root-a");
    tree.collapseNode("root-a");

    const state = get(tree.state);
    expect(state.selectedId).toBe("root-a");
    expect(state.expanded.has("root-a")).toBe(false);
  });

  it("preserves expanded descendants when parent is refreshed", async () => {
    const fetchChildren = vi.fn(async (parent: TreeNode | null) => {
      if (!parent) {
        return [makeNode("root-a", null, true)];
      }
      if (parent.id === "root-a") {
        return [makeNode("child-a", "root-a", true)];
      }
      if (parent.id === "child-a") {
        return [makeNode("leaf-a", "child-a", false)];
      }
      return [];
    });

    const tree = createTreeStore({ fetchChildren });
    await tree.initialize();

    tree.toggleExpanded("root-a");
    await Promise.resolve();
    await Promise.resolve();

    tree.toggleExpanded("child-a");
    await Promise.resolve();
    await Promise.resolve();

    await tree.refreshNode("root-a");

    const rows = get(tree.visibleRows);
    expect(rows.map((row) => row.id)).toEqual(["root-a", "child-a", "leaf-a"]);
    const state = get(tree.state);
    expect(state.expanded.has("child-a")).toBe(true);
    expect(state.nodes["child-a"]?.childIds).toEqual(["leaf-a"]);
  });
});
