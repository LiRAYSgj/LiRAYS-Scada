import type { Node } from "@xyflow/svelte";
import type { TagScalarValue } from "$lib/core/ws/types";
import type { PlantAssetNodeData } from "./assets/types";

export interface GraphLiveDependencyIndex {
  tagToNodeIndexes: Map<string, number[]>;
}

function getBindingValuesForNode(
  data: PlantAssetNodeData,
  values: Record<string, TagScalarValue>,
): Record<string, TagScalarValue | TagScalarValue[] | undefined> {
  const result: Record<string, TagScalarValue | TagScalarValue[] | undefined> =
    {};

  for (const [bindingKey, tags] of Object.entries(data.bindings ?? {})) {
    const tagIds = tags
      .filter((tag) => tag.kind === "tag")
      .map((tag) => tag.id)
      .filter(Boolean);
    if (tagIds.length === 0) {
      result[bindingKey] = undefined;
      continue;
    }
    if (tagIds.length === 1) {
      result[bindingKey] = values[tagIds[0]];
      continue;
    }
    result[bindingKey] = tagIds
      .map((id) => values[id])
      .filter((v): v is TagScalarValue => v !== undefined);
  }

  return result;
}

function isSameArray(
  a: TagScalarValue[] | undefined,
  b: TagScalarValue[] | undefined,
): boolean {
  if (!a && !b) return true;
  if (!a || !b) return false;
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i += 1) {
    if (a[i] !== b[i]) return false;
  }
  return true;
}

function isSameBindingValues(
  a: Record<string, TagScalarValue | TagScalarValue[] | undefined> | undefined,
  b: Record<string, TagScalarValue | TagScalarValue[] | undefined>,
): boolean {
  const aKeys = Object.keys(a ?? {});
  const bKeys = Object.keys(b);
  if (aKeys.length !== bKeys.length) return false;
  for (const key of bKeys) {
    const current = a?.[key];
    const next = b[key];
    if (Array.isArray(current) || Array.isArray(next)) {
      if (
        !isSameArray(
          Array.isArray(current) ? current : undefined,
          Array.isArray(next) ? next : undefined,
        )
      ) {
        return false;
      }
      continue;
    }
    if (current !== next) return false;
  }
  return true;
}

function pickPrimaryLiveValue(
  data: PlantAssetNodeData,
  bindingValues: Record<string, TagScalarValue | TagScalarValue[] | undefined>,
): TagScalarValue | undefined {
  const key = data.primaryBindingKey;
  if (!key) return undefined;
  const value = bindingValues[key];
  if (Array.isArray(value)) return value[0];
  return value;
}

function getNodeTrackedTagIds(node: Node): string[] {
  const unique: Record<string, true> = {};
  const data = node.data as PlantAssetNodeData | undefined;
  const bindings = data?.bindings;

  if (bindings) {
    for (const tags of Object.values(bindings)) {
      for (const tag of tags) {
        if (tag.kind === "tag") {
          unique[tag.id] = true;
        }
      }
    }
  }

  const sourceNode = data?.sourceNode;
  if (sourceNode?.kind === "tag") {
    unique[sourceNode.id] = true;
  }

  return Object.keys(unique);
}

function applyLiveValuesToNode(
  node: Node,
  values: Record<string, TagScalarValue>,
): { node: Node; changed: boolean } {
  const data = node.data as PlantAssetNodeData | undefined;
  if (!data) {
    return { node, changed: false };
  }

  if (data.bindings && Object.keys(data.bindings).length > 0) {
    const nextBindingValues = getBindingValuesForNode(data, values);
    const nextPrimaryLiveValue = pickPrimaryLiveValue(data, nextBindingValues);
    const primaryValueChanged = nextPrimaryLiveValue !== data.liveValue;
    const bindingsChanged = !isSameBindingValues(
      data.liveValues,
      nextBindingValues,
    );

    if (!primaryValueChanged && !bindingsChanged) {
      return { node, changed: false };
    }

    return {
      changed: true,
      node: {
        ...node,
        data: {
          ...data,
          liveValue: nextPrimaryLiveValue,
          liveValues: nextBindingValues,
        },
      },
    };
  }

  if (!data.sourceNode || data.sourceNode.kind !== "tag") {
    return { node, changed: false };
  }

  const incoming = values[data.sourceNode.id];
  if (incoming === undefined || incoming === data.liveValue) {
    return { node, changed: false };
  }

  return {
    changed: true,
    node: {
      ...node,
      data: {
        ...data,
        liveValue: incoming,
      },
    },
  };
}

export function getTrackedTagIds(nodes: Node[]): string[] {
  const unique: Record<string, true> = {};
  for (const node of nodes) {
    for (const tagId of getNodeTrackedTagIds(node)) {
      unique[tagId] = true;
    }
  }
  return Object.keys(unique);
}

export function buildGraphLiveDependencyIndex(
  nodes: Node[],
): GraphLiveDependencyIndex {
  const tagToNodeIndexes = new Map<string, number[]>();

  nodes.forEach((node, index) => {
    for (const tagId of getNodeTrackedTagIds(node)) {
      const indexes = tagToNodeIndexes.get(tagId);
      if (indexes) {
        indexes.push(index);
      } else {
        tagToNodeIndexes.set(tagId, [index]);
      }
    }
  });

  return { tagToNodeIndexes };
}

export function applyLiveValuesToGraphNodes(
  nodes: Node[],
  values: Record<string, TagScalarValue>,
): { nodes: Node[]; changed: boolean } {
  let changed = false;
  const nextNodes = nodes.map((node) => {
    const result = applyLiveValuesToNode(node, values);
    if (result.changed) {
      changed = true;
    }
    return result.node;
  });

  return { nodes: nextNodes, changed };
}

export function applyLiveValuesToGraphNodesAtIndexes(
  nodes: Node[],
  values: Record<string, TagScalarValue>,
  indexes: Iterable<number>,
): { nodes: Node[]; changed: boolean } {
  let nextNodes: Node[] | null = null;
  let changed = false;
  const seen = new Set<number>();

  for (const index of indexes) {
    if (seen.has(index) || index < 0 || index >= nodes.length) {
      continue;
    }
    seen.add(index);

    const result = applyLiveValuesToNode(nodes[index], values);
    if (!result.changed) {
      continue;
    }

    if (!nextNodes) {
      nextNodes = [...nodes];
    }
    nextNodes[index] = result.node;
    changed = true;
  }

  return {
    nodes: nextNodes ?? nodes,
    changed,
  };
}

export function normalizePipeEdges<
  T extends { type?: string; style?: unknown; animated?: boolean },
>(
  edges: T[],
  pipeType: string,
  pipeStyle: unknown,
): { edges: T[]; changed: boolean } {
  const hasSameStyle = (edgeStyle: unknown): boolean => {
    if (typeof pipeStyle === "string") {
      return edgeStyle === pipeStyle;
    }
    if (
      pipeStyle &&
      typeof pipeStyle === "object" &&
      edgeStyle &&
      typeof edgeStyle === "object"
    ) {
      return JSON.stringify(edgeStyle) === JSON.stringify(pipeStyle);
    }
    return edgeStyle === pipeStyle;
  };

  let changed = false;
  const normalized = edges.map((edge) => {
    if (
      edge.type === pipeType &&
      hasSameStyle(edge.style) &&
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
