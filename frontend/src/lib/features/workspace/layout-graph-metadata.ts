import { SvelteMap } from "svelte/reactivity";
import type { Node } from "@xyflow/svelte";
import type { VarMetaChanged } from "@lirays/scada-proto";
import type { TreeNode } from "$lib/features/tree/types";
import type { PlantAssetNodeData } from "$lib/features/graph/assets/types";

export interface BoundTagMetadata {
  min?: number;
  max?: number;
  unit?: string;
  dataType?: string;
  options?: string[];
}

function isStringArrayEqual(a?: string[], b?: string[]): boolean {
  if (a === b) {
    return true;
  }
  const left = a ?? [];
  const right = b ?? [];
  if (left.length !== right.length) {
    return false;
  }
  for (let index = 0; index < left.length; index += 1) {
    if (left[index] !== right[index]) {
      return false;
    }
  }
  return true;
}

export function applyVarMetaChanged(
  nodes: Node[],
  changes: VarMetaChanged[],
): { nodes: Node[]; changed: boolean } {
  if (changes.length === 0 || nodes.length === 0) {
    return { nodes, changed: false };
  }

  const byVarId = new SvelteMap<string, VarMetaChanged>();
  for (const change of changes) {
    byVarId.set(change.varId, change);
  }

  let changed = false;
  const nextNodes = nodes.map((node) => {
    if (node.type !== "plantAsset") {
      return node;
    }
    const data = node.data as PlantAssetNodeData;
    let nodeChanged = false;

    let nextBindings: PlantAssetNodeData["bindings"] = data.bindings;
    if (data.bindings) {
      const updatedBindings = Object.fromEntries(
        Object.entries(data.bindings).map(([bindingKey, tags]) => {
          const updatedTags = tags.map((tag) => {
            if (tag.kind !== "tag") {
              return tag;
            }
            const change = byVarId.get(tag.id);
            if (!change) {
              return tag;
            }
            const nextMin = change.min ?? undefined;
            const nextMax = change.max ?? undefined;
            if (tag.min === nextMin && tag.max === nextMax) {
              return tag;
            }
            nodeChanged = true;
            return {
              ...tag,
              min: nextMin,
              max: nextMax,
            };
          });
          return [bindingKey, updatedTags];
        }),
      );
      if (nodeChanged) {
        nextBindings = updatedBindings;
      }
    }

    const source = data.sourceNode;
    const sourceChange =
      source?.kind === "tag" ? byVarId.get(source.id) : undefined;
    let nextSource = source;
    if (sourceChange) {
      const nextMin = sourceChange.min ?? undefined;
      const nextMax = sourceChange.max ?? undefined;
      if (source.min !== nextMin || source.max !== nextMax) {
        nextSource = {
          ...source,
          min: nextMin,
          max: nextMax,
        };
        nodeChanged = true;
      }
    }

    if (!nodeChanged) {
      return node;
    }

    changed = true;
    return {
      ...node,
      data: {
        ...data,
        bindings: nextBindings,
        sourceNode: nextSource,
      },
    };
  });

  return { nodes: changed ? nextNodes : nodes, changed };
}

export function applyTreeSnapshotMetadata(
  nodes: Node[],
  treeNodes: Record<string, TreeNode>,
): { nodes: Node[]; changed: boolean } {
  if (nodes.length === 0 || Object.keys(treeNodes).length === 0) {
    return { nodes, changed: false };
  }

  let changed = false;
  const nextNodes = nodes.map((node) => {
    if (node.type !== "plantAsset") {
      return node;
    }
    const data = node.data as PlantAssetNodeData;
    let nodeChanged = false;

    let nextBindings: PlantAssetNodeData["bindings"] = data.bindings;
    if (data.bindings) {
      const updatedBindings = Object.fromEntries(
        Object.entries(data.bindings).map(([bindingKey, tags]) => {
          const updatedTags = tags.map((tag) => {
            if (tag.kind !== "tag") {
              return tag;
            }
            const latest = treeNodes[tag.id];
            if (!latest || latest.kind !== "tag") {
              return tag;
            }
            const nextMin = latest.min ?? undefined;
            const nextMax = latest.max ?? undefined;
            const nextUnit = latest.unit ?? undefined;
            const nextDataType = latest.dataType ?? undefined;
            const nextOptions = latest.options;
            const sameOptions = isStringArrayEqual(tag.options, nextOptions);
            if (
              tag.min === nextMin &&
              tag.max === nextMax &&
              tag.unit === nextUnit &&
              tag.dataType === nextDataType &&
              sameOptions
            ) {
              return tag;
            }
            nodeChanged = true;
            return {
              ...tag,
              min: nextMin,
              max: nextMax,
              unit: nextUnit,
              dataType: nextDataType,
              options: nextOptions,
            };
          });
          return [bindingKey, updatedTags];
        }),
      );
      if (nodeChanged) {
        nextBindings = updatedBindings;
      }
    }

    const source = data.sourceNode;
    let nextSource = source;
    if (source?.kind === "tag") {
      const latest = treeNodes[source.id];
      if (latest?.kind === "tag") {
        const nextMin = latest.min ?? undefined;
        const nextMax = latest.max ?? undefined;
        const nextUnit = latest.unit ?? undefined;
        const nextDataType = latest.dataType ?? undefined;
        const nextOptions = latest.options;
        const sameOptions = isStringArrayEqual(source.options, nextOptions);
        if (
          source.min !== nextMin ||
          source.max !== nextMax ||
          source.unit !== nextUnit ||
          source.dataType !== nextDataType ||
          !sameOptions
        ) {
          nextSource = {
            ...source,
            min: nextMin,
            max: nextMax,
            unit: nextUnit,
            dataType: nextDataType,
            options: nextOptions,
          };
          nodeChanged = true;
        }
      }
    }

    if (!nodeChanged) {
      return node;
    }

    changed = true;
    return {
      ...node,
      data: {
        ...data,
        bindings: nextBindings,
        sourceNode: nextSource,
      },
    };
  });

  return { nodes: changed ? nextNodes : nodes, changed };
}

export function applyBoundMetadata(
  nodes: Node[],
  byTagId: Map<string, BoundTagMetadata>,
): { nodes: Node[]; changed: boolean } {
  if (byTagId.size === 0 || nodes.length === 0) {
    return { nodes, changed: false };
  }

  let changed = false;
  const nextNodes = nodes.map((node) => {
    if (node.type !== "plantAsset") {
      return node;
    }
    const data = node.data as PlantAssetNodeData;
    let nodeChanged = false;

    let nextBindings: PlantAssetNodeData["bindings"] = data.bindings;
    if (data.bindings) {
      const updatedBindings = Object.fromEntries(
        Object.entries(data.bindings).map(([bindingKey, tags]) => {
          const updatedTags = tags.map((tag) => {
            if (tag.kind !== "tag") {
              return tag;
            }
            const latest = byTagId.get(tag.id);
            if (!latest) {
              return tag;
            }
            const sameOptions = isStringArrayEqual(tag.options, latest.options);
            if (
              tag.min === latest.min &&
              tag.max === latest.max &&
              tag.unit === latest.unit &&
              sameOptions
            ) {
              return tag;
            }
            nodeChanged = true;
            return {
              ...tag,
              min: latest.min,
              max: latest.max,
              unit: latest.unit,
              options: latest.options,
            };
          });
          return [bindingKey, updatedTags];
        }),
      );
      if (nodeChanged) {
        nextBindings = updatedBindings;
      }
    }

    const source = data.sourceNode;
    let nextSource = source;
    if (source?.kind === "tag") {
      const latest = byTagId.get(source.id);
      if (latest) {
        const sameOptions = isStringArrayEqual(source.options, latest.options);
        if (
          source.min !== latest.min ||
          source.max !== latest.max ||
          source.unit !== latest.unit ||
          !sameOptions
        ) {
          nextSource = {
            ...source,
            min: latest.min,
            max: latest.max,
            unit: latest.unit,
            options: latest.options,
          };
          nodeChanged = true;
        }
      }
    }

    if (!nodeChanged) {
      return node;
    }

    changed = true;
    return {
      ...node,
      data: {
        ...data,
        bindings: nextBindings,
        sourceNode: nextSource,
      },
    };
  });

  return { nodes: changed ? nextNodes : nodes, changed };
}

export function getParentIdFromNodePath(pathOrId: string): string | null {
  if (!pathOrId || pathOrId === "/") {
    return null;
  }
  const trimmed = pathOrId.endsWith("/") ? pathOrId.slice(0, -1) : pathOrId;
  const idx = trimmed.lastIndexOf("/");
  if (idx <= 0) {
    return null;
  }
  return trimmed.slice(0, idx);
}
