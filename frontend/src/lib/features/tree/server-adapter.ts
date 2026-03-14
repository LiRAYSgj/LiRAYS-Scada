import type { TreeNode } from "./types";
import { tagStreamClient } from "$lib/core/ws/tag-stream-client";
import type { ListResponse } from "$lib/proto/namespace/commands";
import { varDataTypeToJSON } from "$lib/proto/namespace/enums";

const DEFAULT_WS_ENDPOINT = "ws://127.0.0.1:1236";

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
  const basePath = parent?.path ?? "";

  const folders = Object.entries(payload.childrenFolders).map(([name, id]) => ({
    id,
    parentId: parent?.id ?? null,
    name,
    path: basePath ? `${basePath}/${name}` : name,
    kind: "folder" as const,
    hasChildren: true,
    childIds: null,
  }));

  const vars = Object.entries(payload.childrenVars).map(
    ([name, { varId: id, varDType: dataType }]) => ({
      id,
      parentId: parent?.id ?? null,
      name,
      path: basePath ? `${basePath}/${name}` : name,
      kind: "tag" as const,
      hasChildren: false,
      childIds: null,
      dataType:
        dataType !== undefined ? varDataTypeToJSON(dataType) : undefined,
    }),
  );

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
