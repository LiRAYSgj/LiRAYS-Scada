import type { Node } from "@xyflow/svelte";

export function getNodeZIndex(node: Node, fallbackIndex: number): number {
  if (typeof node.zIndex === "number" && Number.isFinite(node.zIndex)) {
    return node.zIndex;
  }
  return fallbackIndex + 1;
}

export function getOrderedNodeIdsByZ(nodes: Node[]): string[] {
  return nodes
    .map((node, index) => ({
      id: node.id,
      zIndex: getNodeZIndex(node, index),
      index,
    }))
    .sort((a, b) => a.zIndex - b.zIndex || a.index - b.index)
    .map((entry) => entry.id);
}

export function normalizeNodeZIndexes(nodes: Node[]): {
  nodes: Node[];
  changed: boolean;
} {
  const orderedIds = getOrderedNodeIdsByZ(nodes);
  const zIndexById = new Map<string, number>();
  orderedIds.forEach((id, index) => {
    zIndexById.set(id, index + 1);
  });

  let changed = false;
  const normalized = nodes.map((node, index) => {
    const nextZIndex = zIndexById.get(node.id) ?? index + 1;
    const currentZIndex = getNodeZIndex(node, index);
    if (currentZIndex === nextZIndex) {
      return node;
    }
    changed = true;
    return {
      ...node,
      zIndex: nextZIndex,
    };
  });
  return { nodes: changed ? normalized : nodes, changed };
}

export function applyNodeOrderByIds(
  nodes: Node[],
  orderedIds: string[],
): { nodes: Node[]; changed: boolean } {
  if (orderedIds.length === 0) {
    return { nodes, changed: false };
  }
  const zIndexById = new Map<string, number>();
  orderedIds.forEach((id, index) => {
    zIndexById.set(id, index + 1);
  });

  let changed = false;
  const nextNodes = nodes.map((node, index) => {
    const nextZIndex = zIndexById.get(node.id);
    if (nextZIndex === undefined) {
      return node;
    }
    const currentZIndex = getNodeZIndex(node, index);
    if (currentZIndex === nextZIndex) {
      return node;
    }
    changed = true;
    return {
      ...node,
      zIndex: nextZIndex,
    };
  });

  return { nodes: nextNodes, changed };
}

export function getNextNodeZIndex(nodes: Node[]): number {
  if (nodes.length === 0) {
    return 1;
  }
  const max = nodes.reduce(
    (acc, node, index) => Math.max(acc, getNodeZIndex(node, index)),
    0,
  );
  return max + 1;
}
