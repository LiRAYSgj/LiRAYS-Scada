import type { TreeNode } from "./types";
import { tagStreamClient } from "$lib/core/ws/tag-stream-client";
import type { ListResponsePayload } from "$lib/core/ws/types";

const DEFAULT_WS_ENDPOINT = "ws://127.0.0.1:1236";

function resolveWsEndpoint(): string {
  const configured = import.meta.env.PUBLIC_DEMO_WS_ENDPOINT as
    | string
    | undefined;
  return configured || DEFAULT_WS_ENDPOINT;
}

function toTreeNodes(
  parent: TreeNode | null,
  payload: ListResponsePayload,
): TreeNode[] {
  const basePath = parent?.path ?? "";

  const folders = Object.entries(payload.children_folders).map(
    ([name, id]) => ({
      id,
      parentId: parent?.id ?? null,
      name,
      path: basePath ? `${basePath}/${name}` : name,
      kind: "folder" as const,
      hasChildren: true,
      childIds: null,
    }),
  );

  const vars = Object.entries(payload.children_vars).map(
    ([name, [id, dataType]]) => ({
      id,
      parentId: parent?.id ?? null,
      name,
      path: basePath ? `${basePath}/${name}` : name,
      kind: "tag" as const,
      hasChildren: false,
      childIds: null,
      dataType,
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
