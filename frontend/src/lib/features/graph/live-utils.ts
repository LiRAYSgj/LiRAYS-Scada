import type { Node } from "@xyflow/svelte";
import type { TagScalarValue } from "$lib/core/ws/types";
import type { PlantAssetNodeData } from "./assets/types";

export function getTrackedTagIds(nodes: Node[]): string[] {
  const unique: Record<string, true> = {};
  for (const node of nodes) {
    const data = node.data as PlantAssetNodeData | undefined;
    const sourceNode = data?.sourceNode;
    if (sourceNode?.kind === "tag") {
      unique[sourceNode.id] = true;
    }
  }
  return Object.keys(unique);
}

export function applyLiveValuesToGraphNodes(
  nodes: Node[],
  values: Record<string, TagScalarValue>,
): { nodes: Node[]; changed: boolean } {
  let changed = false;
  const nextNodes = nodes.map((node) => {
    const data = node.data as PlantAssetNodeData | undefined;
    if (!data?.sourceNode || data.sourceNode.kind !== "tag") {
      return node;
    }
    const incoming = values[data.sourceNode.id];
    if (incoming === undefined || incoming === data.liveValue) {
      return node;
    }

    changed = true;
    return {
      ...node,
      data: {
        ...data,
        liveValue: incoming,
      },
    };
  });

  return { nodes: nextNodes, changed };
}

export function normalizePipeEdges<
  T extends { type?: string; style?: unknown; animated?: boolean },
>(
  edges: T[],
  pipeType: string,
  pipeStyle: string,
): { edges: T[]; changed: boolean } {
  let changed = false;
  const normalized = edges.map((edge) => {
    const edgeStyle = typeof edge.style === "string" ? edge.style : "";
    if (
      edge.type === pipeType &&
      edgeStyle === pipeStyle &&
      edge.animated === false
    ) {
      return edge;
    }
    changed = true;
    return {
      ...edge,
      type: pipeType,
      animated: false,
      style: pipeStyle,
    };
  });
  return { edges: normalized, changed };
}
