import { describe, expect, it } from "vitest";
import { flattenVisibleRows } from "./flatten";
import type { TreeNode } from "./types";

describe("flattenVisibleRows", () => {
  const root: TreeNode = {
    id: "root",
    parentId: null,
    name: "root",
    path: "root",
    kind: "folder",
    hasChildren: true,
    childIds: ["child-a", "child-b"],
  };

  const childA: TreeNode = {
    id: "child-a",
    parentId: "root",
    name: "child-a",
    path: "root/child-a",
    kind: "tag",
    hasChildren: false,
    childIds: [],
  };

  const childB: TreeNode = {
    id: "child-b",
    parentId: "root",
    name: "child-b",
    path: "root/child-b",
    kind: "tag",
    hasChildren: false,
    childIds: [],
  };

  it("returns only roots when collapsed", () => {
    const rows = flattenVisibleRows({
      nodes: { root, "child-a": childA, "child-b": childB },
      rootIds: ["root"],
      expanded: new Set<string>(),
      loading: new Set<string>(),
      errored: new Set<string>(),
    });

    expect(rows.map((row) => row.id)).toEqual(["root"]);
  });

  it("returns children directly after expanded parent", () => {
    const rows = flattenVisibleRows({
      nodes: { root, "child-a": childA, "child-b": childB },
      rootIds: ["root"],
      expanded: new Set<string>(["root"]),
      loading: new Set<string>(),
      errored: new Set<string>(),
    });

    expect(rows.map((row) => row.id)).toEqual(["root", "child-a", "child-b"]);
    expect(rows[1].depth).toBe(2);
    expect(rows[2].depth).toBe(2);
  });

  it("sets isLoading and isErrored from state", () => {
    const rows = flattenVisibleRows({
      nodes: { root, "child-a": childA, "child-b": childB },
      rootIds: ["root"],
      expanded: new Set<string>(["root"]),
      loading: new Set<string>(["child-a"]),
      errored: new Set<string>(["child-b"]),
    });
    expect(rows).toHaveLength(3);
    expect(rows[0].isLoading).toBe(false);
    expect(rows[0].isErrored).toBe(false);
    expect(rows[1].isLoading).toBe(true);
    expect(rows[1].isErrored).toBe(false);
    expect(rows[2].isLoading).toBe(false);
    expect(rows[2].isErrored).toBe(true);
  });

  it("returns empty when rootIds empty", () => {
    const rows = flattenVisibleRows({
      nodes: { root },
      rootIds: [],
      expanded: new Set(),
      loading: new Set(),
      errored: new Set(),
    });
    expect(rows).toEqual([]);
  });
});
