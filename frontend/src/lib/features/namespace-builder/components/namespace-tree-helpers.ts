/**
 * Pure helpers for the namespace visual tree (flatten, locate, DnD rules).
 * Keeps NamespaceBuilder script smaller; no DOM here except getGhostParent.
 */
import type { FlatRow, NamespaceNode } from "../types.js";

export const ROW_HEIGHT = 36;
export const OVERSCAN = 8;

export function flatten(
  nodes: NamespaceNode[],
  depth = 0,
  parentId: string | null = null,
): FlatRow[] {
  const rows: FlatRow[] = [];
  for (let i = 0; i < nodes.length; i += 1) {
    const node = nodes[i];
    rows.push({
      id: node.id,
      depth,
      node,
      parentId,
      parentChildren: nodes,
      index: i,
    });
    rows.push(...flatten(node.children, depth + 1, node.id));
  }
  return rows;
}

export function findRowById(
  ast: NamespaceNode[],
  nodeId: string,
): FlatRow | null {
  return flatten(ast).find((row) => row.id === nodeId) ?? null;
}

export function findParentContainer(
  ast: NamespaceNode[],
  parentId: string | null,
): NamespaceNode[] {
  if (!parentId) return ast;
  const row = findRowById(ast, parentId);
  return row?.node.children ?? ast;
}

export function findNodeLocation(
  nodes: NamespaceNode[],
  nodeId: string,
  parent: NamespaceNode | null = null,
  parentChildren: NamespaceNode[] = nodes,
): {
  node: NamespaceNode;
  parent: NamespaceNode | null;
  parentChildren: NamespaceNode[];
  index: number;
} | null {
  for (let index = 0; index < nodes.length; index += 1) {
    const node = nodes[index];
    if (node.id === nodeId) {
      return { node, parent, parentChildren, index };
    }
    const nested = findNodeLocation(node.children, nodeId, node, node.children);
    if (nested) return nested;
  }
  return null;
}

export function isDescendant(
  root: NamespaceNode,
  candidateId: string,
): boolean {
  for (const child of root.children) {
    if (child.id === candidateId) return true;
    if (isDescendant(child, candidateId)) return true;
  }
  return false;
}

export function isDropAllowed(
  ast: NamespaceNode[],
  draggedId: string,
  targetRowId: string,
): boolean {
  if (draggedId === targetRowId) return false;
  const draggedRow = findRowById(ast, draggedId);
  if (!draggedRow) return false;
  return !isDescendant(draggedRow.node, targetRowId);
}

export function resolveDropPositionForPointer(
  target: HTMLElement,
  clientY: number,
): "before" | "child" {
  const rect = target.getBoundingClientRect();
  const y = clientY - rect.top;
  const h = rect.height;
  if (h <= 0) return "child";
  return y < h / 2 ? "before" : "child";
}

export function getGhostParent(sourceEl: HTMLElement | null): HTMLElement {
  if (sourceEl) {
    const dialog = sourceEl.closest("dialog");
    if (dialog && (dialog as HTMLDialogElement).open) return dialog;
  }
  const openDialog = document.querySelector("dialog[open]");
  if (openDialog) return openDialog as HTMLElement;
  return document.body;
}
