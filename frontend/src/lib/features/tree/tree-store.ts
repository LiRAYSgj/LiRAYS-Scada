import { derived, writable } from "svelte/store";
import type { TreeChanged } from "$lib/proto/namespace/events";
import { flattenVisibleRows } from "./flatten";
import type { TreeNode, TreeState } from "./types";
import {
  applyFolderChangedToState,
  pruneOrphanSelection,
  type ReloadParentRequest,
} from "./tree-remote-reconcile";

interface TreeAdapter {
  fetchChildren: (parent: TreeNode | null) => Promise<TreeNode[]>;
}

const INITIAL_TREE_STATE: TreeState = {
  nodes: {},
  rootIds: [],
  expanded: new Set<string>(),
  selectedId: null,
  loading: new Set<string>(),
  errored: new Set<string>(),
  rootLoading: false,
  hasInitialized: false,
};

function upsertChildren(
  state: TreeState,
  parentId: string | null,
  children: TreeNode[],
): void {
  for (const child of children) {
    const existing = state.nodes[child.id];
    if (!existing) {
      state.nodes[child.id] = child;
      continue;
    }

    state.nodes[child.id] = {
      ...existing,
      ...child,
      // Preserve already loaded descendants for stable refresh behavior.
      childIds: existing.childIds,
      hasChildren:
        child.kind === "folder"
          ? existing.childIds !== null
            ? existing.childIds.length > 0
            : child.hasChildren
          : false,
    };
  }

  const childIds = children.map((child) => child.id);
  if (parentId) {
    const parent = state.nodes[parentId];
    if (parent) {
      parent.childIds = childIds;
      parent.hasChildren = childIds.length > 0;
    }
    return;
  }

  state.rootIds = childIds;
}

export function createTreeStore(adapter: TreeAdapter) {
  const state = writable<TreeState>(structuredClone(INITIAL_TREE_STATE));

  async function initialize(): Promise<void> {
    let shouldFetchRoots = false;
    state.update((current) => {
      if (current.rootLoading || current.rootIds.length > 0) {
        return current;
      }

      current.rootLoading = true;
      shouldFetchRoots = true;
      return current;
    });

    if (!shouldFetchRoots) {
      state.update((current) => {
        current.hasInitialized = true;
        return current;
      });
      return;
    }

    try {
      const roots = await adapter.fetchChildren(null);
      state.update((current) => {
        current.rootLoading = false;
        current.hasInitialized = true;
        upsertChildren(current, null, roots);
        return current;
      });
    } catch {
      state.update((current) => {
        current.rootLoading = false;
        current.hasInitialized = true;
        return current;
      });
    }
  }

  async function loadChildren(nodeId: string): Promise<void> {
    let targetNode: TreeNode | null = null;
    let shouldFetch = false;

    state.update((current) => {
      const node = current.nodes[nodeId];
      if (
        !node ||
        !node.hasChildren ||
        node.childIds !== null ||
        current.loading.has(nodeId)
      ) {
        return current;
      }

      targetNode = node;
      current.loading.add(nodeId);
      current.errored.delete(nodeId);
      shouldFetch = true;
      return current;
    });

    if (!shouldFetch || !targetNode) {
      return;
    }

    try {
      const children = await adapter.fetchChildren(targetNode);
      state.update((current) => {
        current.loading.delete(nodeId);
        upsertChildren(current, nodeId, children);
        return current;
      });
    } catch {
      state.update((current) => {
        current.loading.delete(nodeId);
        current.errored.add(nodeId);
        return current;
      });
    }
  }

  function toggleExpanded(nodeId: string): void {
    let shouldLoad = false;

    state.update((current) => {
      const node = current.nodes[nodeId];
      if (!node) {
        return current;
      }

      if (current.expanded.has(nodeId)) {
        current.expanded.delete(nodeId);
        return current;
      }

      current.expanded.add(nodeId);
      if (node.hasChildren && node.childIds === null) {
        shouldLoad = true;
      }

      return current;
    });

    if (shouldLoad) {
      void loadChildren(nodeId);
    }
  }

  function selectNode(nodeId: string): void {
    state.update((current) => {
      current.selectedId = nodeId;
      return current;
    });
  }

  function collapseNode(nodeId: string): void {
    state.update((current) => {
      current.expanded.delete(nodeId);
      return current;
    });
  }

  async function refreshNode(nodeId: string | null): Promise<void> {
    let targetNode: TreeNode | null = null;
    if (nodeId) {
      state.update((current) => {
        targetNode = current.nodes[nodeId] ?? null;
        if (targetNode) {
          current.loading.add(nodeId);
          current.errored.delete(nodeId);
        }
        return current;
      });
      if (!targetNode) {
        return;
      }
    } else {
      state.update((current) => {
        current.rootLoading = true;
        return current;
      });
    }

    try {
      const children = await adapter.fetchChildren(targetNode);
      state.update((current) => {
        if (nodeId) {
          current.loading.delete(nodeId);
        } else {
          current.rootLoading = false;
        }
        upsertChildren(current, nodeId, children);
        return current;
      });
    } catch {
      state.update((current) => {
        if (nodeId) {
          current.loading.delete(nodeId);
          current.errored.add(nodeId);
        } else {
          current.rootLoading = false;
        }
        return current;
      });
    }
  }

  const visibleRows = derived(state, (current) =>
    flattenVisibleRows({
      nodes: current.nodes,
      rootIds: current.rootIds,
      expanded: current.expanded,
      loading: current.loading,
      errored: current.errored,
    }),
  );

  /**
   * Applies `TreeChanged` pushes from the global WS subscription (other UI instances).
   * Handles reload vs incremental rules for cached children.
   */
  async function applyRemoteTreeChanged(ev: TreeChanged): Promise<void> {
    const reloads: ReloadParentRequest[] = [];
    state.update((current) => {
      for (const fc of ev.folderChangedEvent) {
        reloads.push(...applyFolderChangedToState(current, fc));
      }
      pruneOrphanSelection(current);
      return current;
    });

    const seen = new Set<string>();
    for (const parentId of reloads) {
      const key = parentId === null ? "__root__" : parentId;
      if (seen.has(key)) {
        continue;
      }
      seen.add(key);
      await refreshNode(parentId);
    }
  }

  return {
    state,
    visibleRows,
    initialize,
    toggleExpanded,
    selectNode,
    collapseNode,
    refreshNode,
    applyRemoteTreeChanged,
  };
}
