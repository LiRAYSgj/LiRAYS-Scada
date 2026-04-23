import type { TreeNode } from "./types";
import { tagStreamClient } from "$lib/core/ws/tag-stream-client";
import { resolveTagStreamWsEndpoint } from "$lib/core/ws/resolve-ws-endpoint";
import type { ListResponse } from "@lirays/scada-proto";
import { varDataTypeToJSON } from "@lirays/scada-proto";

function toNumberArray(value: unknown): number[] | undefined {
  if (Array.isArray(value)) {
    return value.filter((item): item is number => typeof item === "number");
  }
  if (typeof value === "number") {
    return [value];
  }
  return undefined;
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
    maxLen: toNumberArray(variable.maxLen),
  }));

  return [...folders, ...vars];
}

export async function fetchTreeChildren(
  parent: TreeNode | null,
): Promise<TreeNode[]> {
  const list = await tagStreamClient.listChildren(
    parent?.id,
    resolveTagStreamWsEndpoint(),
  );
  return toTreeNodes(parent, list);
}
