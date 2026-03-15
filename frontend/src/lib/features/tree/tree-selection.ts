/**
 * Multi-selection state for the variable tree.
 * Selection is a Set of node ids; only nodes in this set are considered selected.
 */

import type { TreeNode } from "./types";

/** Collect all loaded descendant ids of a node (recursive, uses nodes map). */
export function getLoadedDescendantIds(
  nodeId: string,
  nodes: Record<string, TreeNode>,
): string[] {
  const node = nodes[nodeId];
  if (!node?.childIds) return [];
  const ids: string[] = [];
  for (const childId of node.childIds) {
    ids.push(childId);
    ids.push(...getLoadedDescendantIds(childId, nodes));
  }
  return ids;
}

/**
 * From a selection set, compute the minimal set of "greatest ancestors" to send to delete:
 * selected nodes that have no other selected ancestor (root is never sent and is not
 * counted as an ancestor for this purpose, so the root's selected children are included).
 */
export function getMinimalAncestorSet(
  selection: Set<string>,
  nodes: Record<string, TreeNode>,
  rootId: string | null,
): string[] {
  const result: string[] = [];
  for (const id of selection) {
    if (rootId !== null && id === rootId) continue;
    let current: TreeNode | undefined = nodes[id];
    let hasSelectedAncestorOtherThanRoot = false;
    while (current?.parentId) {
      const parentId = current.parentId;
      if (parentId !== rootId && selection.has(parentId)) {
        hasSelectedAncestorOtherThanRoot = true;
        break;
      }
      current = nodes[parentId];
    }
    if (!hasSelectedAncestorOtherThanRoot) result.push(id);
  }
  return result;
}
