import { describe, expect, it } from "vitest";
import {
  getLoadedDescendantIds,
  getMinimalAncestorSet,
  hasPartialSelectionInSubtree,
} from "./tree-selection";
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

describe("tree-selection", () => {
  describe("getLoadedDescendantIds", () => {
    it("returns empty when node has no children", () => {
      const nodes: Record<string, TreeNode> = {
        a: node("a", null, []),
      };
      expect(getLoadedDescendantIds("a", nodes)).toEqual([]);
    });

    it("returns empty when node is missing or has no childIds", () => {
      const nodes: Record<string, TreeNode> = {
        a: node("a", null),
      };
      (nodes.a as TreeNode).childIds = null;
      expect(getLoadedDescendantIds("a", nodes)).toEqual([]);
      expect(getLoadedDescendantIds("missing", nodes)).toEqual([]);
    });

    it("returns direct children and nested descendants", () => {
      const nodes: Record<string, TreeNode> = {
        root: node("root", null, ["f1", "f2"]),
        f1: node("f1", "root", ["v1"]),
        f2: node("f2", "root", []),
        v1: node("v1", "f1", null),
      };
      (nodes.v1 as TreeNode).kind = "tag";
      expect(getLoadedDescendantIds("root", nodes)).toEqual(["f1", "v1", "f2"]);
      expect(getLoadedDescendantIds("f1", nodes)).toEqual(["v1"]);
      expect(getLoadedDescendantIds("f2", nodes)).toEqual([]);
    });
  });

  describe("hasPartialSelectionInSubtree", () => {
    it("returns false when node has no loaded descendants", () => {
      const nodes: Record<string, TreeNode> = {
        a: node("a", null, []),
      };
      expect(hasPartialSelectionInSubtree("a", nodes, new Set(["a"]))).toBe(
        false,
      );
    });

    it("returns false when all descendants are selected", () => {
      const nodes: Record<string, TreeNode> = {
        f: node("f", null, ["c1", "c2"]),
        c1: node("c1", "f", null),
        c2: node("c2", "f", null),
      };
      expect(
        hasPartialSelectionInSubtree("f", nodes, new Set(["f", "c1", "c2"])),
      ).toBe(false);
    });

    it("returns false when no descendants are selected", () => {
      const nodes: Record<string, TreeNode> = {
        f: node("f", null, ["c1", "c2"]),
        c1: node("c1", "f", null),
        c2: node("c2", "f", null),
      };
      expect(hasPartialSelectionInSubtree("f", nodes, new Set(["f"]))).toBe(
        false,
      );
    });

    it("returns true when some but not all descendants are selected", () => {
      const nodes: Record<string, TreeNode> = {
        f: node("f", null, ["c1", "c2"]),
        c1: node("c1", "f", null),
        c2: node("c2", "f", null),
      };
      expect(
        hasPartialSelectionInSubtree("f", nodes, new Set(["f", "c1"])),
      ).toBe(true);
      expect(hasPartialSelectionInSubtree("f", nodes, new Set(["c1"]))).toBe(
        true,
      );
    });
  });

  describe("getMinimalAncestorSet", () => {
    it("returns root when only root is selected (roots can be removed)", () => {
      const nodes: Record<string, TreeNode> = {
        root: node("root", null, ["child"]),
        child: node("child", "root", null),
      };
      const selection = new Set(["root"]);
      expect(getMinimalAncestorSet(selection, nodes, null)).toEqual(["root"]);
    });

    it("returns multiple roots when multiple roots selected", () => {
      const nodes: Record<string, TreeNode> = {
        r1: node("r1", null, []),
        r2: node("r2", null, []),
      };
      const selection = new Set(["r1", "r2"]);
      expect(getMinimalAncestorSet(selection, nodes, null)).toEqual([
        "r1",
        "r2",
      ]);
    });

    it("returns only variable when variable selected and parent partially selected", () => {
      const nodes: Record<string, TreeNode> = {
        folder: node("folder", null, ["v1", "v2"]),
        v1: node("v1", "folder", null),
        v2: node("v2", "folder", null),
      };
      (nodes.v1 as TreeNode).kind = "tag";
      (nodes.v2 as TreeNode).kind = "tag";
      const selection = new Set(["folder", "v1"]);
      expect(getMinimalAncestorSet(selection, nodes, null)).toEqual(["v1"]);
    });

    it("returns only folder when folder and all its children selected", () => {
      const nodes: Record<string, TreeNode> = {
        folder: node("folder", null, ["v1", "v2"]),
        v1: node("v1", "folder", null),
        v2: node("v2", "folder", null),
      };
      (nodes.v1 as TreeNode).kind = "tag";
      (nodes.v2 as TreeNode).kind = "tag";
      const selection = new Set(["folder", "v1", "v2"]);
      expect(getMinimalAncestorSet(selection, nodes, null)).toEqual(["folder"]);
    });

    it("returns parent folder when parent and child folder both fully selected", () => {
      const nodes: Record<string, TreeNode> = {
        parent: node("parent", null, ["child"]),
        child: node("child", "parent", ["v1"]),
        v1: node("v1", "child", null),
      };
      (nodes.v1 as TreeNode).kind = "tag";
      const selection = new Set(["parent", "child", "v1"]);
      expect(getMinimalAncestorSet(selection, nodes, null)).toEqual(["parent"]);
    });

    it("returns both when two separate branches fully selected", () => {
      const nodes: Record<string, TreeNode> = {
        r: node("r", null, ["a", "b"]),
        a: node("a", "r", ["v1"]),
        b: node("b", "r", ["v2"]),
        v1: node("v1", "a", null),
        v2: node("v2", "b", null),
      };
      (nodes.v1 as TreeNode).kind = "tag";
      (nodes.v2 as TreeNode).kind = "tag";
      const selection = new Set(["a", "v1", "b", "v2"]);
      expect(getMinimalAncestorSet(selection, nodes, null)).toEqual(["a", "b"]);
    });

    it("excludes partially selected nodes from result", () => {
      const nodes: Record<string, TreeNode> = {
        f: node("f", null, ["c1", "c2"]),
        c1: node("c1", "f", null),
        c2: node("c2", "f", null),
      };
      const selection = new Set(["f", "c1"]);
      expect(getMinimalAncestorSet(selection, nodes, null)).toEqual(["c1"]);
    });
  });
});
