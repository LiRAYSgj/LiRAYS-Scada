import type { TreeNode } from "./types";

export interface TreeSelectionSummary {
  loadedDescendantCountById: Map<string, number>;
  selectedDescendantCountById: Map<string, number>;
  indeterminateIds: Set<string>;
}

export function buildTreeSelectionSummary(
  nodes: Record<string, TreeNode>,
  rootIds: string[],
  selection: Set<string>,
): TreeSelectionSummary {
  const loadedDescendantCountById = new Map<string, number>();
  const selectedDescendantCountById = new Map<string, number>();
  const indeterminateIds = new Set<string>();
  const visited = new Set<string>();
  const postOrder: string[] = [];
  const stack: string[] = [...rootIds];

  while (stack.length > 0) {
    const nodeId = stack.pop();
    if (!nodeId || visited.has(nodeId)) {
      continue;
    }
    visited.add(nodeId);
    postOrder.push(nodeId);
    const node = nodes[nodeId];
    if (!node?.childIds || node.childIds.length === 0) {
      continue;
    }
    for (const childId of node.childIds) {
      if (!visited.has(childId) && nodes[childId]) {
        stack.push(childId);
      }
    }
  }

  for (let index = postOrder.length - 1; index >= 0; index -= 1) {
    const nodeId = postOrder[index];
    const node = nodes[nodeId];
    if (!node) {
      continue;
    }
    let loadedDescendants = 0;
    let selectedDescendants = 0;

    for (const childId of node.childIds ?? []) {
      if (!nodes[childId]) {
        continue;
      }
      loadedDescendants += 1;
      loadedDescendants += loadedDescendantCountById.get(childId) ?? 0;

      if (selection.has(childId)) {
        selectedDescendants += 1;
      }
      selectedDescendants += selectedDescendantCountById.get(childId) ?? 0;
    }

    loadedDescendantCountById.set(nodeId, loadedDescendants);
    selectedDescendantCountById.set(nodeId, selectedDescendants);
    if (
      loadedDescendants > 0 &&
      selectedDescendants > 0 &&
      selectedDescendants < loadedDescendants
    ) {
      indeterminateIds.add(nodeId);
    }
  }

  return {
    loadedDescendantCountById,
    selectedDescendantCountById,
    indeterminateIds,
  };
}
