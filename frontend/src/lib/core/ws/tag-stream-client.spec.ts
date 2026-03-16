import { get } from "svelte/store";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TagStreamClient } from "./tag-stream-client";
import { WebSocketConnectionStatus } from "./types";
import { Command, Response } from "../../proto/namespace/commands";
import { ItemType, OperationStatus } from "../../proto/namespace/enums";

vi.mock("$app/environment", () => ({
  browser: true,
}));

type Handler = (event?: unknown) => void;

class FakeWebSocket {
  static OPEN = 1;
  static CONNECTING = 0;
  static instances: FakeWebSocket[] = [];

  public readyState = FakeWebSocket.CONNECTING;
  public sent: any[] = [];
  private handlers: Record<string, Handler[]> = {};

  constructor(public readonly url: string) {
    FakeWebSocket.instances.push(this);
  }

  addEventListener(type: string, handler: Handler): void {
    this.handlers[type] = this.handlers[type] ?? [];
    this.handlers[type].push(handler);
  }

  send(payload: any): void {
    this.sent.push(payload);
  }

  close(): void {
    this.readyState = 3;
    this.emit("close");
  }

  emit(type: string, event?: unknown): void {
    for (const handler of this.handlers[type] ?? []) {
      handler(event);
    }
  }
}

describe("TagStreamClient", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    FakeWebSocket.instances = [];
    vi.stubGlobal("WebSocket", FakeWebSocket as unknown as typeof WebSocket);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("transitions status on connect/open", () => {
    const client = new TagStreamClient();
    client.start("ws://localhost:8787");

    expect(get(client.status)).toBe(WebSocketConnectionStatus.CONNECTING);

    const ws = FakeWebSocket.instances[0];
    ws.readyState = FakeWebSocket.OPEN;
    ws.emit("open");

    expect(get(client.status)).toBe(WebSocketConnectionStatus.CONNECTED);
  });

  it("sends GET request when tracked ids are set", () => {
    const client = new TagStreamClient();
    client.start("ws://localhost:8787");
    const ws = FakeWebSocket.instances[0];
    ws.readyState = FakeWebSocket.OPEN;
    ws.emit("open");

    client.setTrackedIds(["tag-a", "tag-b"]);

    const decodedRequests = ws.sent.map((msg) => Command.decode(msg));
    expect(decodedRequests.some((req) => req.get !== undefined)).toBe(true);
    expect(
      decodedRequests.some(
        (req) =>
          req.get?.varIds.includes("tag-a") &&
          req.get?.varIds.includes("tag-b"),
      ),
    ).toBe(true);
  });

  it("updates value store on GET response messages", () => {
    const client = new TagStreamClient();
    client.start("ws://localhost:8787");
    const ws = FakeWebSocket.instances[0];
    ws.readyState = FakeWebSocket.OPEN;
    ws.emit("open");
    client.setTrackedIds(["tag-a"]);

    const decodedRequests = ws.sent.map((msg) => Command.decode(msg));
    const request = decodedRequests.find((req) => req.get !== undefined);
    expect(request).toBeDefined();

    const responseBytes = Response.encode({
      status: OperationStatus.OPERATION_STATUS_OK,
      get: {
        cmdId: request!.get!.cmdId,
        varValues: [{ value: { integerValue: 57 } }],
      },
    }).finish();

    ws.emit("message", {
      data: responseBytes.slice().buffer,
    });

    expect(get(client.values)).toEqual({ "tag-a": 57 });
  });

  it("sends write payload and resolves when Response.status is OK", async () => {
    const client = new TagStreamClient();
    client.start("ws://localhost:8787");
    const ws = FakeWebSocket.instances[0];
    ws.readyState = FakeWebSocket.OPEN;
    ws.emit("open");

    const writePromise = client.sendWriteValue("tag-cmd", 42);
    await Promise.resolve();
    const setMsg = ws.sent.find((msg) => Command.decode(msg).set !== undefined);
    expect(setMsg).toBeDefined();
    const cmdId = Command.decode(setMsg!).set!.cmdId;
    expect(
      Command.decode(setMsg!).set?.varIdsValues.some(
        (v) => v.varId === "tag-cmd" && v.value?.integerValue === 42,
      ),
    ).toBe(true);

    const responseBytes = Response.encode({
      status: OperationStatus.OPERATION_STATUS_OK,
      set: { cmdId },
    }).finish();
    ws.emit("message", { data: responseBytes.slice().buffer });
    await expect(writePromise).resolves.toBeUndefined();
  });

  it("reuses existing socket when list request happens before start", async () => {
    const client = new TagStreamClient();
    const listPromise = client.listChildren(undefined, "ws://localhost:8787");
    const ws = FakeWebSocket.instances[0];
    ws.readyState = FakeWebSocket.OPEN;
    ws.emit("open");

    let listRequestMsg = ws.sent.find(
      (msg) => Command.decode(msg).list !== undefined,
    );
    for (let attempt = 0; !listRequestMsg && attempt < 5; attempt += 1) {
      await Promise.resolve();
      listRequestMsg = ws.sent.find(
        (msg) => Command.decode(msg).list !== undefined,
      );
    }
    expect(listRequestMsg).toBeDefined();
    const parsed = Command.decode(listRequestMsg!);

    const responseBytes = Response.encode({
      status: OperationStatus.OPERATION_STATUS_OK,
      list: {
        cmdId: parsed.list!.cmdId,
        childrenFolders: { root: "root-id" },
        childrenVars: {},
      },
    }).finish();

    ws.emit("message", {
      data: responseBytes.slice().buffer,
    });

    await listPromise;

    client.start("ws://localhost:8787");
    expect(FakeWebSocket.instances.length).toBe(1);
  });

  it("does not open a second socket while first is connecting", () => {
    const client = new TagStreamClient();
    void client.listChildren(undefined, "ws://localhost:8787");
    expect(FakeWebSocket.instances.length).toBe(1);
    client.start("ws://localhost:8787");
    expect(FakeWebSocket.instances.length).toBe(1);
  });

  it("resolves addBulkNamespace when Response.add_bulk matches cmd_id", async () => {
    const client = new TagStreamClient();
    client.start("ws://localhost:8787");
    const ws = FakeWebSocket.instances[0];
    ws.readyState = FakeWebSocket.OPEN;
    ws.emit("open");

    const bulkPromise = client.addBulkNamespace("", {
      Area_: { Power: "Float" },
    });
    await Promise.resolve();
    const bulkMsg = ws.sent.find(
      (msg) => Command.decode(msg).addBulk !== undefined,
    );
    expect(bulkMsg).toBeDefined();
    const cmdId = Command.decode(bulkMsg!).addBulk!.cmdId;

    const responseBytes = Response.encode({
      status: OperationStatus.OPERATION_STATUS_OK,
      addBulk: { cmdId },
    }).finish();
    ws.emit("message", { data: responseBytes.slice().buffer });

    await expect(bulkPromise).resolves.toBeUndefined();
  });

  it("sends add command with empty parentId when parentId is null (root creation)", async () => {
    const client = new TagStreamClient();
    client.start("ws://localhost:8787");
    const ws = FakeWebSocket.instances[0];
    ws.readyState = FakeWebSocket.OPEN;
    ws.emit("open");

    const addPromise = client.addItem(
      null,
      "newRoot",
      ItemType.ITEM_TYPE_FOLDER,
      undefined,
      "ws://localhost:8787",
    );
    await Promise.resolve();
    const addMsg = ws.sent.find((msg) => Command.decode(msg).add !== undefined);
    expect(addMsg).toBeDefined();
    const decoded = Command.decode(addMsg!);
    expect(decoded.add?.parentId).toBe("");
    const cmdId = decoded.add!.cmdId;
    const responseBytes = Response.encode({
      status: OperationStatus.OPERATION_STATUS_OK,
      add: { cmdId },
    }).finish();
    ws.emit("message", { data: responseBytes.slice().buffer });
    await expect(addPromise).resolves.toEqual([]);
  });

  it("enters reconnecting state after unexpected close", () => {
    const client = new TagStreamClient();
    client.start("ws://localhost:8787");
    const first = FakeWebSocket.instances[0];
    first.readyState = FakeWebSocket.OPEN;
    first.emit("open");
    first.emit("close");

    expect(get(client.status)).toBe(WebSocketConnectionStatus.CONNECTED);

    vi.advanceTimersByTime(2100);
    expect(get(client.status)).toBe(WebSocketConnectionStatus.RECONNECTING);
    expect(FakeWebSocket.instances.length).toBe(2);
  });
});
