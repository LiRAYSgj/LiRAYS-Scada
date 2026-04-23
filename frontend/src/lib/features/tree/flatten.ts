import type { TreeNode, VisibleTreeRow } from "./types";

interface FlattenParams {
  nodes: Record<string, TreeNode>;
  rootIds: string[];
  expanded: Set<string>;
  loading: Set<string>;
  errored: Set<string>;
}

interface StackFrame {
  id: string;
  depth: number;
}

export function flattenVisibleRows({
  nodes,
  rootIds,
  expanded,
  loading,
  errored,
}: FlattenParams): VisibleTreeRow[] {
  const rows: VisibleTreeRow[] = [];
  const stack: StackFrame[] = rootIds
    .slice()
    .reverse()
    .map((id) => ({ id, depth: 1 }));

  while (stack.length > 0) {
    const frame = stack.pop();
    if (!frame) {
      continue;
    }

    const node = nodes[frame.id];
    if (!node) {
      continue;
    }

    const isExpanded = expanded.has(node.id);
    rows.push({
      id: node.id,
      node,
      depth: frame.depth,
      isExpanded,
      isLoading: loading.has(node.id),
      isErrored: errored.has(node.id),
    });

    if (!isExpanded || !node.childIds || node.childIds.length === 0) {
      continue;
    }

    for (let index = node.childIds.length - 1; index >= 0; index -= 1) {
      stack.push({
        id: node.childIds[index],
        depth: frame.depth + 1,
      });
    }
  }

  return rows;
}
