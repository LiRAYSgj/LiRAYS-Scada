import { describe, expect, it } from "vitest";
import type { Edge } from "@xyflow/svelte";
import {
  createDefaultConnectorEdge,
  normalizeConnectorEdges,
  applyArrowConnectorPatchToEdge,
} from "./connectors";

describe("connectors", () => {
  it("keeps already-normalized connector edge instances", () => {
    const edge = createDefaultConnectorEdge(
      { source: "a", target: "b", sourceHandle: null, targetHandle: null },
      "pipe-1",
    );
    const result = normalizeConnectorEdges([edge]);
    expect(result.changed).toBe(false);
    expect(result.edges[0]).toBe(edge);
  });

  it("normalizes legacy pipe type and reports change", () => {
    const legacy: Edge = {
      id: "pipe-2",
      source: "a",
      target: "b",
      type: "connector-pipe",
      animated: true,
      style: "stroke:#123456;stroke-width:8;",
    };
    const result = normalizeConnectorEdges([legacy]);
    expect(result.changed).toBe(true);
    expect(result.edges[0].animated).toBe(false);
    expect(result.edges[0].type).toBe("connector-pipe");
  });

  it("reports change after arrow config patch", () => {
    const edge = createDefaultConnectorEdge(
      { source: "a", target: "b", sourceHandle: null, targetHandle: null },
      "pipe-3",
    );
    const patched = applyArrowConnectorPatchToEdge(edge, { color: "#112233" });
    const result = normalizeConnectorEdges([patched]);
    expect(result.changed).toBe(false);
    expect(result.edges[0].style).toContain("#112233");
  });
});
