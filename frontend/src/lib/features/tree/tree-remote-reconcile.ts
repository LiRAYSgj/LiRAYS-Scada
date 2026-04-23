import type { FolderChanged, VarMetaChanged } from "@lirays/scada-proto";
import { varDataTypeToJSON } from "@lirays/scada-proto";
import type { FolderInfo, VarInfo } from "@lirays/scada-proto";
import type { TreeNode, TreeState } from "./types";

/** `null` means refresh virtual root (same as `refreshNode(null)`). */
export type ReloadParentRequest = string | null;

export function normalizeFolderParentKey(folderId: string): string | null {
  if (!folderId || folderId === "/") {
    return null;
  }
  return folderId;
}

function collectSubtreeIds(state: TreeState, rootId: string): Set<string> {
  const acc = new Set<string>();
  const stack = [rootId];
  while (stack.length) {
    const id = stack.pop()!;
    if (acc.has(id)) {
      continue;
    }
    acc.add(id);
    const n = state.nodes[id];
    if (n?.childIds) {
      stack.push(...n.childIds);
    }
  }
  return acc;
}

function collectIdsToRemove(
  state: TreeState,
  removedItems: string[],
): Set<string> {
  const all = new Set<string>();
  for (const fid of removedItems) {
    for (const id of collectSubtreeIds(state, fid)) {
      all.add(id);
    }
  }
  // for (const vid of varIds) {
  //   all.add(vid);
  // }
  return all;
}

/** Remove nodes and strip references from parents / rootIds. */
export function purgeNodes(state: TreeState, ids: Set<string>): void {
  for (const id of ids) {
    delete state.nodes[id];
    state.expanded.delete(id);
    state.loading.delete(id);
    state.errored.delete(id);
  }
  state.rootIds = state.rootIds.filter((id) => !ids.has(id));
  for (const node of Object.values(state.nodes)) {
    if (!node.childIds) {
      continue;
    }
    const next = node.childIds.filter((id) => !ids.has(id));
    if (next.length !== node.childIds.length) {
      node.childIds = next;
      if (node.kind === "folder") {
        node.hasChildren = next.length > 0;
      }
    }
  }
}

function folderToNode(f: FolderInfo, parentId: string | null): TreeNode {
  return {
    id: f.id,
    parentId,
    name: f.name,
    path: f.id,
    kind: "folder",
    hasChildren: true,
    childIds: null,
  };
}

function varToNode(v: VarInfo, parentId: string | null): TreeNode {
  const toNumberArray = (value: unknown): number[] | undefined => {
    if (Array.isArray(value)) {
      return value.filter((item): item is number => typeof item === "number");
    }
    if (typeof value === "number") {
      return [value];
    }
    return undefined;
  };

  return {
    id: v.id,
    parentId,
    name: v.name,
    path: v.id,
    kind: "tag",
    hasChildren: false,
    childIds: null,
    dataType:
      v.varDType !== undefined ? varDataTypeToJSON(v.varDType) : undefined,
    unit: v.unit ?? undefined,
    min: v.min ?? undefined,
    max: v.max ?? undefined,
    options: v.options ?? [],
    maxLen: toNumberArray(v.maxLen),
  };
}

/**
 * Applies variable metadata updates without requiring full folder reload.
 * Returns ids of tag nodes that were updated.
 */
export function applyVarMetaChangedToState(
  state: TreeState,
  changes: VarMetaChanged[],
): string[] {
  const updatedIds: string[] = [];
  for (const change of changes) {
    const node = state.nodes[change.varId];
    if (!node || node.kind !== "tag") {
      continue;
    }
    node.unit = change.unit ?? undefined;
    node.min = change.min ?? undefined;
    node.max = change.max ?? undefined;
    node.options = [...(change.options ?? [])];
    node.maxLen = change.maxLen === undefined ? undefined : [change.maxLen];
    updatedIds.push(node.id);
  }
  return updatedIds;
}

function mergeNewChildNodes(
  state: TreeState,
  parentKey: string,
  newFolders: FolderInfo[],
  newVariables: VarInfo[],
): void {
  const parentRef = state.nodes[parentKey];
  if (!parentRef || parentRef.childIds === null) {
    return;
  }

  const newNodes: TreeNode[] = [
    ...newFolders.map((f) => folderToNode(f, parentKey)),
    ...newVariables.map((v) => varToNode(v, parentKey)),
  ];

  const existing = new Set(parentRef.childIds);
  for (const cn of newNodes) {
    const prev = state.nodes[cn.id];
    if (prev) {
      state.nodes[cn.id] = {
        ...prev,
        ...cn,
        childIds: prev.childIds,
        hasChildren:
          cn.kind === "folder"
            ? prev.childIds !== null
              ? prev.childIds.length > 0
              : cn.hasChildren
            : false,
      };
    } else {
      state.nodes[cn.id] = cn;
    }
    if (!existing.has(cn.id)) {
      parentRef.childIds.push(cn.id);
      existing.add(cn.id);
    }
  }
  parentRef.hasChildren = parentRef.childIds.length > 0;
}

function seedExpandedEmptyFolder(
  state: TreeState,
  parentKey: string,
  newFolders: FolderInfo[],
  newVariables: VarInfo[],
): void {
  const parentRef = state.nodes[parentKey];
  if (!parentRef) {
    return;
  }

  const children: TreeNode[] = [
    ...newFolders.map((f) => folderToNode(f, parentKey)),
    ...newVariables.map((v) => varToNode(v, parentKey)),
  ];

  for (const cn of children) {
    const prev = state.nodes[cn.id];
    if (prev) {
      state.nodes[cn.id] = {
        ...prev,
        ...cn,
        childIds: prev.childIds,
        hasChildren:
          cn.kind === "folder"
            ? prev.childIds !== null
              ? prev.childIds.length > 0
              : cn.hasChildren
            : false,
      };
    } else {
      state.nodes[cn.id] = cn;
    }
  }

  parentRef.childIds = children.map((c) => c.id);
  parentRef.hasChildren = children.length > 0;
}

function applyIncrementalVirtualRoot(
  state: TreeState,
  fc: FolderChanged,
): void {
  const toRemove = collectIdsToRemove(state, fc.removedItems);
  if (toRemove.size > 0) {
    purgeNodes(state, toRemove);
  }

  const newNodes: TreeNode[] = [
    ...fc.newFolders.map((f) => folderToNode(f, null)),
    ...fc.newVariables.map((v) => varToNode(v, null)),
  ];

  const rootSet = new Set(state.rootIds);
  for (const cn of newNodes) {
    const prev = state.nodes[cn.id];
    if (prev) {
      state.nodes[cn.id] = {
        ...prev,
        ...cn,
        childIds: prev.childIds,
        hasChildren:
          cn.kind === "folder"
            ? prev.childIds !== null
              ? prev.childIds.length > 0
              : cn.hasChildren
            : false,
      };
    } else {
      state.nodes[cn.id] = cn;
    }
    if (!rootSet.has(cn.id)) {
      state.rootIds.push(cn.id);
      rootSet.add(cn.id);
    }
  }
}

/**
 * Applies a single `FolderChanged` in place. Returns folders whose children should be re-fetched
 * (`reload === true` and folder expanded). `null` means refresh the virtual root.
 */
export function applyFolderChangedToState(
  state: TreeState,
  fc: FolderChanged,
): ReloadParentRequest[] {
  const reloads: ReloadParentRequest[] = [];
  const parentKey = normalizeFolderParentKey(fc.folderId);

  if (fc.reload) {
    if (parentKey === null) {
      reloads.push(null);
      return reloads;
    }
    const parent = state.nodes[parentKey];
    if (!parent) {
      return reloads;
    }
    if (state.expanded.has(parentKey)) {
      reloads.push(parentKey);
    } else {
      parent.childIds = null;
    }
    return reloads;
  }

  if (!state.hasInitialized) {
    return reloads;
  }

  if (parentKey === null) {
    applyIncrementalVirtualRoot(state, fc);
    return reloads;
  }

  const parent = state.nodes[parentKey];
  if (!parent) {
    return reloads;
  }

  const loaded = parent.childIds !== null;
  const isExpanded = state.expanded.has(parentKey);

  if (loaded) {
    const toRemove = collectIdsToRemove(state, fc.removedItems);
    if (toRemove.size > 0) {
      purgeNodes(state, toRemove);
    }
    if (!state.nodes[parentKey]) {
      return reloads;
    }
    mergeNewChildNodes(state, parentKey, fc.newFolders, fc.newVariables);
    return reloads;
  }

  if (isExpanded) {
    seedExpandedEmptyFolder(state, parentKey, fc.newFolders, fc.newVariables);
  }

  return reloads;
}

export function pruneOrphanSelection(state: TreeState): void {
  if (state.selectedId && !state.nodes[state.selectedId]) {
    state.selectedId = null;
  }
}
