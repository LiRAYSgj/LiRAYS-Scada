import type { TreeNode } from "./types";
import { tagStreamClient } from "$lib/core/ws/tag-stream-client";
import type { ListResponse } from "$lib/proto/namespace/commands";
import { varDataTypeToJSON } from "$lib/proto/namespace/enums";

function resolveWsEndpoint(): string {
  // Browser has no access to server env; infer TLS need from page scheme.
  // If the UI is served over https, use wss to avoid mixed-content errors.
  try {
    const isHttps = location.protocol === "https:";
    const scheme = isHttps ? "wss" : "ws";
    const host = location.host || location.hostname;
    return `${scheme}://${host}/ws`;
  } catch {
    return "ws://localhost/ws";
  }
}

function toTreeNodes(
  parent: TreeNode | null,
  payload: ListResponse,
): TreeNode[] {
  const folders = payload.folders.map((folder) => ({
    id: folder.id,
    parentId: parent?.id ?? null,
    name: folder.name,
    path: folder.id,
    kind: "folder" as const,
    hasChildren: true,
    childIds: null,
  }));

  const vars = payload.variables.map((variable) => ({
    id: variable.id,
    parentId: parent?.id ?? null,
    name: variable.name,
    path: variable.id,
    kind: "tag" as const,
    hasChildren: false,
    childIds: null,
    dataType:
      variable.varDType !== undefined
        ? varDataTypeToJSON(variable.varDType)
        : undefined,
    unit: variable.unit ?? undefined,
    min: variable.min ?? undefined,
    max: variable.max ?? undefined,
    options: variable.options ?? [],
    maxLen: variable.maxLen ?? [],
  }));

  return [...folders, ...vars];
}

export async function fetchTreeChildren(
  parent: TreeNode | null,
): Promise<TreeNode[]> {
  const list = await tagStreamClient.listChildren(
    parent?.id,
    resolveWsEndpoint(),
  );
  return toTreeNodes(parent, list);
}
