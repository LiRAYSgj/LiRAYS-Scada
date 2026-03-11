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
});
