import { get } from "svelte/store";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TagStreamClient } from "./tag-stream-client";
import { WebSocketConnectionStatus } from "./types";

vi.mock("$app/environment", () => ({
  browser: true,
}));

type Handler = (event?: unknown) => void;

class FakeWebSocket {
  static OPEN = 1;
  static CONNECTING = 0;
  static instances: FakeWebSocket[] = [];

  public readyState = FakeWebSocket.CONNECTING;
  public sent: string[] = [];
  private handlers: Record<string, Handler[]> = {};

  constructor(public readonly url: string) {
    FakeWebSocket.instances.push(this);
  }

  addEventListener(type: string, handler: Handler): void {
    this.handlers[type] = this.handlers[type] ?? [];
    this.handlers[type].push(handler);
  }

  send(payload: string): void {
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

    expect(ws.sent.some((msg) => msg.includes('"GET"'))).toBe(true);
    expect(
      ws.sent.some((msg) => msg.includes('"var_ids":["tag-a","tag-b"]')),
    ).toBe(true);
  });

  it("updates value store on GET response messages", () => {
    const client = new TagStreamClient();
    client.start("ws://localhost:8787");
    const ws = FakeWebSocket.instances[0];
    ws.readyState = FakeWebSocket.OPEN;
    ws.emit("open");
    client.setTrackedIds(["tag-a"]);
    const request = ws.sent.find((msg) => msg.includes('"GET"'));
    expect(request).toBeDefined();
    const parsed = JSON.parse(request ?? "{}") as { GET: { cmd_id: string } };

    ws.emit("message", {
      data: JSON.stringify({
        GET: {
          cmd_id: parsed.GET.cmd_id,
          var_values: [{ Integer: 57 }],
        },
      }),
    });

    expect(get(client.values)).toEqual({ "tag-a": 57 });
  });

  it("sends write payload when requested", () => {
    const client = new TagStreamClient();
    client.start("ws://localhost:8787");
    const ws = FakeWebSocket.instances[0];
    ws.readyState = FakeWebSocket.OPEN;
    ws.emit("open");

    client.sendWriteValue("tag-cmd", 42);
    expect(ws.sent.some((msg) => msg.includes('"SET"'))).toBe(true);
    expect(ws.sent.some((msg) => msg.includes('"Integer":42'))).toBe(true);
  });

  it("reuses existing socket when list request happens before start", async () => {
    const client = new TagStreamClient();
    const listPromise = client.listChildren(undefined, "ws://localhost:8787");
    const ws = FakeWebSocket.instances[0];
    ws.readyState = FakeWebSocket.OPEN;
    ws.emit("open");
    let listRequest = ws.sent.find((msg) => msg.includes('"LIST"'));
    for (let attempt = 0; !listRequest && attempt < 5; attempt += 1) {
      await Promise.resolve();
      listRequest = ws.sent.find((msg) => msg.includes('"LIST"'));
    }
    expect(listRequest).toBeDefined();
    const parsed = JSON.parse(listRequest ?? "{}") as {
      LIST: { cmd_id: string };
    };
    ws.emit("message", {
      data: JSON.stringify({
        LIST: {
          cmd_id: parsed.LIST.cmd_id,
          children_folders: { root: "root-id" },
          children_vars: {},
        },
      }),
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
