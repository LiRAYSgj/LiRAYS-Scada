import { describe, expect, it } from "vitest";
import type { Edge, Node, Viewport } from "@xyflow/svelte";
import {
  buildNextEntityId,
  cloneForHistory,
  createGraphHistorySnapshot,
  resolveInitialEditorViewport,
  snapshotSignature,
} from "./layout-graph-history";

function makePlantNode(id: string, overrides: Partial<Node> = {}): Node {
  return {
    id,
    type: "plantAsset",
    position: { x: 10, y: 20 },
    width: 200,
    height: 80,
    selected: true,
    dragging: true,
    data: {
      symbolId: id,
      assetKind: "pump",
      liveValue: 123,
      liveValues: { a: 1 },
      onWriteValue: () => {},
      onWriteBindingValue: () => {},
      onWidgetEvent: () => {},
      onOpenBindingConfig: () => {},
      graphReadOnly: true,
      interactionMode: "runtime",
      title: "Pump",
    },
    ...overrides,
  } as Node;
}

function makeEdge(id: string): Edge {
  return {
    id,
    source: "a",
    target: "b",
    selected: true,
  } as Edge;
}

describe("layout-graph-history", () => {
  it("strips transient runtime fields from snapshots", () => {
    const snapshot = createGraphHistorySnapshot(
      [makePlantNode("n1")],
      [makeEdge("e1")],
    );
    const node = snapshot.nodes[0];
    const edge = snapshot.edges[0];
    expect(node.selected).toBeUndefined();
    expect((node.data as Record<string, unknown>).liveValue).toBeUndefined();
    expect((node.data as Record<string, unknown>).onWriteValue).toBeUndefined();
    expect(
      (node.data as Record<string, unknown>).graphReadOnly,
    ).toBeUndefined();
    expect(edge.selected).toBeUndefined();
  });

  it("produces deterministic signatures for equivalent snapshots", () => {
    const a = {
      nodes: [makePlantNode("n1", { data: { a: 1, b: 2 } })],
      edges: [makeEdge("e1")],
    };
    const b = {
      nodes: [makePlantNode("n1", { data: { b: 2, a: 1 } })],
      edges: [makeEdge("e1")],
    };
    expect(snapshotSignature(a)).toBe(snapshotSignature(b));
  });

  it("throws when cloning unsupported top-level values", () => {
    expect(() => cloneForHistory(() => "x")).toThrow(
      "Unable to clone graph state for history.",
    );
  });

  it("builds next entity id from base suffix strategy", () => {
    expect(buildNextEntityId("pump-4", 8)).toBe("pump-8");
    expect(buildNextEntityId("pump", 8)).toBe("pump-8");
    expect(buildNextEntityId("-4", 8)).toBe("-4-8");
  });

  it("returns saved viewport when there are no plant nodes", () => {
    const saved: Viewport = { x: 1, y: 2, zoom: 3 };
    const next = resolveInitialEditorViewport(saved, []);
    expect(next).toEqual(saved);
  });

  it("centers bounds and clamps zoom into allowed range", () => {
    const saved: Viewport = { x: 0, y: 0, zoom: 4.5 };
    const node = makePlantNode("n1", {
      position: { x: 100, y: 50 },
      width: 100,
      height: 50,
    });
    const next = resolveInitialEditorViewport(saved, [node], 1000, 500);
    expect(next.zoom).toBe(1.6);
    expect(next.x).toBeCloseTo(260);
    expect(next.y).toBeCloseTo(130);
  });
});
