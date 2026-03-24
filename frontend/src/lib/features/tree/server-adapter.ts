import type { TreeNode } from "./types";
import { tagStreamClient } from "$lib/core/ws/tag-stream-client";
import type { ListResponse } from "$lib/proto/namespace/commands";
import { varDataTypeToJSON } from "$lib/proto/namespace/enums";

const DEFAULT_WS_ENDPOINT = "ws://127.0.0.1:8245";

function resolveWsEndpoint(): string {
  const configured = import.meta.env.PUBLIC_DEMO_WS_ENDPOINT as
    | string
    | undefined;
  return configured || DEFAULT_WS_ENDPOINT;
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
