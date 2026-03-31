import { describe, expect, it, vi } from "vitest";
import { fetchTreeChildren } from "./server-adapter";
import { tagStreamClient } from "$lib/core/ws/tag-stream-client";
import { VarDataType } from "$lib/proto/namespace/enums";

vi.mock("$lib/core/ws/resolve-ws-endpoint", () => ({
  resolveTagStreamWsEndpoint: () => "ws://test-backend/ws",
}));

vi.mock("$lib/core/ws/tag-stream-client", () => ({
  tagStreamClient: {
    listChildren: vi.fn(),
  },
}));

describe("fetchTreeChildren", () => {
  it("maps root LIST payload to root tree nodes", async () => {
    vi.mocked(tagStreamClient.listChildren).mockResolvedValue({
      cmdId: "cmd-1",
      folders: [{ id: "root-id", name: "root" }],
      variables: [],
    });

    const roots = await fetchTreeChildren(null);
    expect(roots).toEqual([
      expect.objectContaining({
        id: "root-id",
        name: "root",
        parentId: null,
        kind: "folder",
        hasChildren: true,
        childIds: null,
      }),
    ]);
  });

  it("maps folder and variable children for a parent", async () => {
    // LIST payloads use VarDataType (int32); varDataTypeToJSON drives TreeNode.dataType
    vi.mocked(tagStreamClient.listChildren).mockResolvedValue({
      cmdId: "cmd-2",
      folders: [{ id: "folder-1", name: "area" }],
      variables: [
        {
          id: "var-1",
          name: "pressure",
          varDType: VarDataType.VAR_DATA_TYPE_FLOAT,
          options: [],
        },
      ],
    });

    const parent = {
      id: "root-id",
      parentId: null,
      name: "root",
      path: "root",
      kind: "folder" as const,
      hasChildren: true,
      childIds: null,
    };
    const children = await fetchTreeChildren(parent);

    expect(children).toEqual([
      expect.objectContaining({
        id: "folder-1",
        name: "area",
        parentId: "root-id",
        path: "folder-1",
        kind: "folder",
        hasChildren: true,
        childIds: null,
      }),
      expect.objectContaining({
        id: "var-1",
        name: "pressure",
        parentId: "root-id",
        path: "var-1",
        kind: "tag",
        hasChildren: false,
        childIds: null,
        dataType: "VAR_DATA_TYPE_FLOAT",
      }),
    ]);
    expect(tagStreamClient.listChildren).toHaveBeenCalledWith(
      "root-id",
      "ws://test-backend/ws",
    );
  });
});
