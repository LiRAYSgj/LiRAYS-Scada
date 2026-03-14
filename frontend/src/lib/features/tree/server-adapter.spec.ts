import { describe, expect, it, vi } from "vitest";
import { fetchTreeChildren } from "./server-adapter";
import { tagStreamClient } from "$lib/core/ws/tag-stream-client";

vi.mock("$lib/core/ws/tag-stream-client", () => ({
  tagStreamClient: {
    listChildren: vi.fn(),
  },
}));

describe("fetchTreeChildren", () => {
  it("maps root LIST payload to root tree nodes", async () => {
    vi.mocked(tagStreamClient.listChildren).mockResolvedValue({
      cmdId: "cmd-1",
      childrenFolders: { root: "root-id" },
      childrenVars: {},
    });

    const roots = await fetchTreeChildren(null);
    expect(roots).toEqual([
      expect.objectContaining({
        id: "root-id",
        name: "root",
        parentId: null,
        kind: "folder",
      }),
    ]);
  });

  it("maps folder and variable children for a parent", async () => {
    vi.mocked(tagStreamClient.listChildren).mockResolvedValue({
      cmdId: "cmd-2",
      childrenFolders: { area: "folder-1" },
      childrenVars: { pressure: { varId: "var-1", varDType: "Float" as any } },
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
        path: "root/area",
        kind: "folder",
      }),
      expect.objectContaining({
        id: "var-1",
        name: "pressure",
        path: "root/pressure",
        kind: "tag",
        dataType: "Float",
      }),
    ]);
    expect(tagStreamClient.listChildren).toHaveBeenCalledWith(
      "root-id",
      "ws://127.0.0.1:1236",
    );
  });
});
