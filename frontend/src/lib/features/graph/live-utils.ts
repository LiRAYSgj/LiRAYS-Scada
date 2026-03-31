import type { Node } from "@xyflow/svelte";
import type { TagScalarValue } from "$lib/core/ws/types";
import type { PlantAssetNodeData } from "./assets/types";

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

export function getTrackedTagIds(nodes: Node[]): string[] {
  const unique: Record<string, true> = {};
  for (const node of nodes) {
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
    if (!data) return node;

    if (data.bindings && Object.keys(data.bindings).length > 0) {
      const nextBindingValues = getBindingValuesForNode(data, values);
      const nextPrimaryLiveValue = pickPrimaryLiveValue(
        data,
        nextBindingValues,
      );
      const primaryValueChanged = nextPrimaryLiveValue !== data.liveValue;
      const bindingsChanged = !isSameBindingValues(
        data.liveValues,
        nextBindingValues,
      );

      if (!primaryValueChanged && !bindingsChanged) {
        return node;
      }

      changed = true;
      return {
        ...node,
        data: {
          ...data,
          liveValue: nextPrimaryLiveValue,
          liveValues: nextBindingValues,
        },
      };
    }

    if (!data.sourceNode || data.sourceNode.kind !== "tag") return node;
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
