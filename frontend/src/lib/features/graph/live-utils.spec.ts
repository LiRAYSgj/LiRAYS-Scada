import type { Node } from "@xyflow/svelte";
import { describe, expect, it } from "vitest";
import {
  applyLiveValuesToGraphNodes,
  applyLiveValuesToGraphNodesAtIndexes,
  buildGraphLiveDependencyIndex,
  getTrackedTagIds,
  normalizePipeEdges,
} from "./live-utils";
import type { PlantAssetNodeData } from "./assets/types";

function makeNode(id: string, data: Partial<PlantAssetNodeData>): Node {
  return {
    id,
    position: { x: 0, y: 0 },
    data: {
      assetKind: "label",
      title: id,
      sourceNode: { id: `source-${id}`, kind: "folder", name: "", path: "" },
      ...data,
    },
  } as unknown as Node;
}

describe("getTrackedTagIds", () => {
  it("returns empty array when no nodes", () => {
    expect(getTrackedTagIds([])).toEqual([]);
  });

  it("extracts unique tag ids from nodes with sourceNode.kind === tag", () => {
    const nodes = [
      makeNode("n1", {
        sourceNode: { id: "tag-a", kind: "tag", name: "", path: "" },
      }),
      makeNode("n2", {
        sourceNode: { id: "tag-b", kind: "tag", name: "", path: "" },
      }),
      makeNode("n3", {
        sourceNode: { id: "tag-a", kind: "tag", name: "", path: "" },
      }),
    ];
    expect(getTrackedTagIds(nodes)).toEqual(["tag-a", "tag-b"]);
  });

  it("ignores nodes without sourceNode or with kind !== tag", () => {
    const nodes = [
      makeNode("n1", {}),
      makeNode("n2", {
        sourceNode: { id: "f1", kind: "folder", name: "", path: "" },
      }),
      makeNode("n3", {
        sourceNode: { id: "tag-1", kind: "tag", name: "", path: "" },
      }),
    ];
    expect(getTrackedTagIds(nodes)).toEqual(["tag-1"]);
  });
});

describe("applyLiveValuesToGraphNodes", () => {
  it("returns same nodes and changed false when no values apply", () => {
    const nodes = [
      makeNode("n1", {
        sourceNode: { id: "t1", kind: "tag", name: "", path: "" },
        liveValue: 10,
      }),
    ];
    const values: Record<string, number> = {};
    const result = applyLiveValuesToGraphNodes(nodes, values);
    expect(result.changed).toBe(false);
    expect(result.nodes).toStrictEqual(nodes);
  });

  it("updates node data when value differs and sets changed true", () => {
    const nodes = [
      makeNode("n1", {
        sourceNode: { id: "t1", kind: "tag", name: "", path: "" },
        liveValue: 10,
      }),
    ];
    const result = applyLiveValuesToGraphNodes(nodes, { t1: 20 });
    expect(result.changed).toBe(true);
    expect(result.nodes[0].data.liveValue).toBe(20);
  });

  it("leaves node unchanged when value equals current liveValue", () => {
    const nodes = [
      makeNode("n1", {
        sourceNode: { id: "t1", kind: "tag", name: "", path: "" },
        liveValue: 10,
      }),
    ];
    const result = applyLiveValuesToGraphNodes(nodes, { t1: 10 });
    expect(result.changed).toBe(false);
    expect(result.nodes).toStrictEqual(nodes);
  });

  it("skips non-tag nodes", () => {
    const nodes = [
      makeNode("n1", {
        sourceNode: { id: "f1", kind: "folder", name: "", path: "" },
      }),
    ];
    const result = applyLiveValuesToGraphNodes(nodes, { f1: 1 });
    expect(result.changed).toBe(false);
    expect(result.nodes).toStrictEqual(nodes);
  });
});

describe("buildGraphLiveDependencyIndex", () => {
  it("indexes bindings and source nodes by tag id", () => {
    const nodes = [
      makeNode("n1", {
        bindings: {
          primary: [
            { id: "tag-a", kind: "tag", name: "A", path: "A" },
            { id: "tag-b", kind: "tag", name: "B", path: "B" },
          ],
        },
      }),
      makeNode("n2", {
        sourceNode: { id: "tag-b", kind: "tag", name: "", path: "" },
      }),
    ];

    const index = buildGraphLiveDependencyIndex(nodes);

    expect(index.tagToNodeIndexes.get("tag-a")).toEqual([0]);
    expect(index.tagToNodeIndexes.get("tag-b")).toEqual([0, 1]);
  });
});

describe("applyLiveValuesToGraphNodesAtIndexes", () => {
  it("updates only the requested node indexes", () => {
    const nodes = [
      makeNode("n1", {
        sourceNode: { id: "tag-a", kind: "tag", name: "", path: "" },
        liveValue: 1,
      }),
      makeNode("n2", {
        sourceNode: { id: "tag-b", kind: "tag", name: "", path: "" },
        liveValue: 2,
      }),
    ];

    const result = applyLiveValuesToGraphNodesAtIndexes(
      nodes,
      { "tag-a": 9, "tag-b": 8 },
      [1],
    );

    expect(result.changed).toBe(true);
    expect(result.nodes[0]).toBe(nodes[0]);
    expect(result.nodes[1].data.liveValue).toBe(8);
  });

  it("ignores duplicate and out-of-range indexes", () => {
    const nodes = [
      makeNode("n1", {
        sourceNode: { id: "tag-a", kind: "tag", name: "", path: "" },
        liveValue: 1,
      }),
    ];

    const result = applyLiveValuesToGraphNodesAtIndexes(
      nodes,
      { "tag-a": 7 },
      [0, 0, 99, -1],
    );

    expect(result.changed).toBe(true);
    expect(result.nodes[0].data.liveValue).toBe(7);
  });
});

describe("normalizePipeEdges", () => {
  const pipeType = "step";
  const pipeStyle = { stroke: "#333", strokeWidth: 4 };

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
      { id: "e1", type: "default", style: { stroke: "#111" }, animated: true },
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
