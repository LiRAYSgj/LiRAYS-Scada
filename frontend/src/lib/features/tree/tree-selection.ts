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
 * True if this node has both selected and unselected loaded descendants (partial selection).
 * Used so indeterminate state propagates up: ancestors show indeterminate when any descendant does.
 */
export function hasPartialSelectionInSubtree(
  nodeId: string,
  nodes: Record<string, TreeNode>,
  selection: Set<string>,
): boolean {
  const descendantIds = getLoadedDescendantIds(nodeId, nodes);
  if (descendantIds.length === 0) return false;
  const someSelected = descendantIds.some((id) => selection.has(id));
  const someUnselected = descendantIds.some((id) => !selection.has(id));
  return someSelected && someUnselected;
}

/** True if node is fully selected (in selection and not partially selected). */
function isFullySelected(
  nodeId: string,
  nodes: Record<string, TreeNode>,
  selection: Set<string>,
): boolean {
  return (
    selection.has(nodeId) &&
    !hasPartialSelectionInSubtree(nodeId, nodes, selection)
  );
}

/**
 * Ids to send to delete: only superior/higher-level selected nodes.
 * - Root nodes can be removed and count as superior (no parent).
 * - Partially selected nodes are not sent (only their fully-selected descendants are).
 * - A fully selected node is sent only if it has no fully selected ancestor.
 * So: variable selected + parent partially selected → send variable only; folder + all children selected → send folder only unless its parent is also fully selected.
 */
export function getMinimalAncestorSet(
  selection: Set<string>,
  nodes: Record<string, TreeNode>,
  _rootId: string | null,
): string[] {
  const result: string[] = [];
  for (const id of selection) {
    if (!isFullySelected(id, nodes, selection)) continue;
    let current: TreeNode | undefined = nodes[id];
    let hasFullySelectedAncestor = false;
    while (current?.parentId) {
      if (isFullySelected(current.parentId, nodes, selection)) {
        hasFullySelectedAncestor = true;
        break;
      }
      current = nodes[current.parentId];
    }
    if (!hasFullySelectedAncestor) result.push(id);
  }
  return result;
}
