import { describe, expect, it } from "vitest";
import type { Node } from "@xyflow/svelte";
import { deserializeCanvasState, serializeCanvasState } from "./types";

describe("views/types metadata persistence", () => {
  it("strips min/max from bound tags and sourceNode on serialize", () => {
    const nodes: Node[] = [
      {
        id: "n1",
        type: "plantAsset",
        position: { x: 0, y: 0 },
        data: {
          assetKind: "tank",
          title: "Tank 1",
          bindings: {
            level: [
              {
                id: "Area.Line.Level",
                name: "Level",
                path: "/Area/Line/Level",
                kind: "tag",
                min: 10,
                max: 90,
              },
            ],
          },
          sourceNode: {
            id: "Area.Line.Level",
            name: "Level",
            path: "/Area/Line/Level",
            kind: "tag",
            min: 10,
            max: 90,
          },
        },
      },
    ];

    const json = serializeCanvasState(nodes, [], { x: 0, y: 0, zoom: 1 });
    const parsed = JSON.parse(json) as { nodes: Node[] };
    const data = parsed.nodes[0].data as Record<string, unknown>;
    const boundTag = (
      (data.bindings as Record<string, unknown>).level as Array<
        Record<string, unknown>
      >
    )[0];
    const sourceNode = data.sourceNode as Record<string, unknown>;

    expect(boundTag.min).toBeUndefined();
    expect(boundTag.max).toBeUndefined();
    expect(sourceNode.min).toBeUndefined();
    expect(sourceNode.max).toBeUndefined();
  });

  it("strips runtime live values from plant assets on serialize", () => {
    const nodes: Node[] = [
      {
        id: "n1",
        type: "plantAsset",
        position: { x: 0, y: 0 },
        data: {
          assetKind: "light",
          title: "Light 1",
          liveValue: true,
          liveValues: {
            state: true,
          },
        },
      },
    ];

    const json = serializeCanvasState(nodes, [], { x: 0, y: 0, zoom: 1 });
    const parsed = JSON.parse(json) as { nodes: Node[] };
    const data = parsed.nodes[0].data as Record<string, unknown>;

    expect(data.liveValue).toBeUndefined();
    expect(data.liveValues).toBeUndefined();
  });

  it("strips runtime live values from legacy canvas json on deserialize", () => {
    const parsed = deserializeCanvasState(
      JSON.stringify({
        version: 1,
        nodes: [
          {
            id: "n1",
            type: "plantAsset",
            position: { x: 0, y: 0 },
            data: {
              assetKind: "light",
              title: "Light 1",
              liveValue: 1,
              liveValues: { state: 1 },
            },
          },
        ],
        edges: [],
        viewport: { x: 0, y: 0, zoom: 1 },
      }),
    );
    const data = parsed.nodes[0].data as Record<string, unknown>;

    expect(data.liveValue).toBeUndefined();
    expect(data.liveValues).toBeUndefined();
  });
});
