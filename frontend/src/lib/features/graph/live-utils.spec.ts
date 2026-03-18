import { describe, expect, it } from "vitest";
import {
  applyLiveValuesToGraphNodes,
  getTrackedTagIds,
  normalizePipeEdges,
} from "./live-utils";

describe("getTrackedTagIds", () => {
  it("returns empty array when no nodes", () => {
    expect(getTrackedTagIds([])).toEqual([]);
  });

  it("extracts unique tag ids from nodes with sourceNode.kind === tag", () => {
    const nodes = [
      { id: "n1", data: { sourceNode: { id: "tag-a", kind: "tag" } } },
      { id: "n2", data: { sourceNode: { id: "tag-b", kind: "tag" } } },
      { id: "n3", data: { sourceNode: { id: "tag-a", kind: "tag" } } },
    ] as any;
    expect(getTrackedTagIds(nodes)).toEqual(["tag-a", "tag-b"]);
  });

  it("ignores nodes without sourceNode or with kind !== tag", () => {
    const nodes = [
      { id: "n1", data: {} },
      { id: "n2", data: { sourceNode: { id: "f1", kind: "folder" } } },
      { id: "n3", data: { sourceNode: { id: "tag-1", kind: "tag" } } },
    ] as any;
    expect(getTrackedTagIds(nodes)).toEqual(["tag-1"]);
  });
});

describe("applyLiveValuesToGraphNodes", () => {
  it("returns same nodes and changed false when no values apply", () => {
    const nodes = [
      {
        id: "n1",
        data: {
          sourceNode: { id: "t1", kind: "tag" as const },
          liveValue: 10,
        },
      },
    ] as any;
    const values: Record<string, number> = {};
    const result = applyLiveValuesToGraphNodes(nodes, values);
    expect(result.changed).toBe(false);
    expect(result.nodes).toStrictEqual(nodes);
  });

  it("updates node data when value differs and sets changed true", () => {
    const nodes = [
      {
        id: "n1",
        data: {
          sourceNode: { id: "t1", kind: "tag" as const },
          liveValue: 10,
        },
      },
    ] as any;
    const result = applyLiveValuesToGraphNodes(nodes, { t1: 20 });
    expect(result.changed).toBe(true);
    expect(result.nodes[0].data.liveValue).toBe(20);
  });

  it("leaves node unchanged when value equals current liveValue", () => {
    const nodes = [
      {
        id: "n1",
        data: {
          sourceNode: { id: "t1", kind: "tag" as const },
          liveValue: 10,
        },
      },
    ] as any;
    const result = applyLiveValuesToGraphNodes(nodes, { t1: 10 });
    expect(result.changed).toBe(false);
    expect(result.nodes).toStrictEqual(nodes);
  });

  it("skips non-tag nodes", () => {
    const nodes = [
      { id: "n1", data: { sourceNode: { id: "f1", kind: "folder" } } },
    ] as any;
    const result = applyLiveValuesToGraphNodes(nodes, { f1: 1 });
    expect(result.changed).toBe(false);
    expect(result.nodes).toStrictEqual(nodes);
  });
});

describe("normalizePipeEdges", () => {
  const pipeType = "step";
  const pipeStyle = "stroke:#333";

  it("returns same edges and changed false when all already normalized", () => {
    const edges = [
      { id: "e1", type: pipeType, style: pipeStyle, animated: false },
    ];
    const result = normalizePipeEdges(edges, pipeType, pipeStyle);
    expect(result.changed).toBe(false);
    expect(result.edges).toStrictEqual(edges);
  });

  it("normalizes edges and sets changed true when type or style differ", () => {
    const edges = [
      { id: "e1", type: "default", style: "other", animated: true },
    ];
    const result = normalizePipeEdges(edges, pipeType, pipeStyle);
    expect(result.changed).toBe(true);
    expect(result.edges[0]).toEqual({
      id: "e1",
      type: pipeType,
      style: pipeStyle,
      animated: false,
    });
  });

  it("handles empty edges array", () => {
    const result = normalizePipeEdges([], pipeType, pipeStyle);
    expect(result.changed).toBe(false);
    expect(result.edges).toEqual([]);
  });
});
