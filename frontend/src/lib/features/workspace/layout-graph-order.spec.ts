import { describe, expect, it } from "vitest";
import type { Node } from "@xyflow/svelte";
import {
  applyNodeOrderByIds,
  getNextNodeZIndex,
  getNodeZIndex,
  getOrderedNodeIdsByZ,
  normalizeNodeZIndexes,
} from "./layout-graph-order";

function makeNode(id: string, zIndex?: number): Node {
  return {
    id,
    type: "plantAsset",
    position: { x: 0, y: 0 },
    data: {},
    zIndex,
  } as Node;
}

describe("layout-graph-order", () => {
  it("uses explicit zIndex when finite and falls back to index otherwise", () => {
    expect(getNodeZIndex(makeNode("a", 5), 0)).toBe(5);
    expect(getNodeZIndex(makeNode("a"), 2)).toBe(3);
    expect(getNodeZIndex(makeNode("a", Number.NaN), 2)).toBe(3);
  });

  it("orders ids by zIndex and original index as tie-breaker", () => {
    const nodes = [makeNode("a", 2), makeNode("b", 1), makeNode("c", 2)];
    expect(getOrderedNodeIdsByZ(nodes)).toEqual(["b", "a", "c"]);
  });

  it("normalizes zIndex values to a contiguous ordering", () => {
    const nodes = [makeNode("a", 9), makeNode("b", 3), makeNode("c", 3)];
    const result = normalizeNodeZIndexes(nodes);
    expect(result.changed).toBe(true);
    expect(result.nodes.map((node) => node.zIndex)).toEqual([3, 1, 2]);
  });

  it("returns unchanged when nodes are already normalized", () => {
    const nodes = [makeNode("a", 1), makeNode("b", 2), makeNode("c", 3)];
    const result = normalizeNodeZIndexes(nodes);
    expect(result.changed).toBe(false);
    expect(result.nodes).toBe(nodes);
  });

  it("applies explicit order ids to node zIndex values", () => {
    const nodes = [makeNode("a", 1), makeNode("b", 2), makeNode("c", 3)];
    const result = applyNodeOrderByIds(nodes, ["c", "a", "b"]);
    expect(result.changed).toBe(true);
    expect(result.nodes.map((node) => [node.id, node.zIndex])).toEqual([
      ["a", 2],
      ["b", 3],
      ["c", 1],
    ]);
  });

  it("respects empty order list and leaves nodes unchanged", () => {
    const nodes = [makeNode("a", 1)];
    const result = applyNodeOrderByIds(nodes, []);
    expect(result.changed).toBe(false);
    expect(result.nodes).toBe(nodes);
  });

  it("computes the next zIndex from the current max", () => {
    expect(getNextNodeZIndex([])).toBe(1);
    expect(getNextNodeZIndex([makeNode("a", 7), makeNode("b", 3)])).toBe(8);
  });
});
