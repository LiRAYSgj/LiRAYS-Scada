import { describe, expect, it } from "vitest";
import {
  flatten,
  findRowById,
  isDropAllowed,
  findNodeLocation,
} from "./namespace-tree-helpers.js";
import type { NamespaceNode } from "../types.js";

function node(
  id: string,
  name: string,
  children: NamespaceNode[] = [],
): NamespaceNode {
  return {
    id,
    name,
    nameSuffix: "",
    seriesMode: "none",
    seriesValues: "",
    kind: children.length ? "folder" : "variable",
    dataType: "Float",
    unit: "",
    min: "",
    max: "",
    maxLength: "",
    options: [],
    rangeStart: "",
    rangeEnd: "",
    rangeStep: "",
    children,
  };
}

describe("namespace-tree-helpers", () => {
  it("flatten preserves depth and parentId", () => {
    const ast = [node("a", "A", [node("b", "B")])];
    const rows = flatten(ast);
    expect(rows).toHaveLength(2);
    expect(rows[0].depth).toBe(0);
    expect(rows[1].depth).toBe(1);
    expect(rows[1].parentId).toBe("a");
  });

  it("isDropAllowed rejects descendant target", () => {
    const ast = [node("p", "P", [node("c", "C")])];
    expect(isDropAllowed(ast, "p", "c")).toBe(false);
    expect(isDropAllowed(ast, "c", "p")).toBe(true);
  });

  it("findNodeLocation returns index", () => {
    const ast = [node("x", "X"), node("y", "Y")];
    const loc = findNodeLocation(ast, "y");
    expect(loc?.index).toBe(1);
    expect(findRowById(ast, "y")?.node.name).toBe("Y");
  });
});
