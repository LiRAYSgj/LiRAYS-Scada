import { get, writable } from "svelte/store";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPageTagRealtimeProvider } from "./page-tag-realtime-provider";
import {
  type TagScalarValue,
  WebSocketConnectionStatus,
} from "$lib/core/ws/types";

const start = vi.fn();
const stop = vi.fn();
const setTrackedIds = vi.fn();
const sendWriteValue = vi.fn();
const addItem = vi.fn().mockResolvedValue([]);
const removeItems = vi.fn().mockResolvedValue(undefined);
const fakeClient = {
  status: writable(WebSocketConnectionStatus.DISCONNECTED),
  values: writable<Record<string, TagScalarValue>>({}),
  start,
  stop,
  setTrackedIds,
  sendWriteValue,
  addItem,
  removeItems,
};

beforeEach(() => {
  start.mockReset();
  stop.mockReset();
  setTrackedIds.mockReset();
  sendWriteValue.mockReset();
  addItem.mockClear();
  removeItems.mockClear();
  fakeClient.status.set(WebSocketConnectionStatus.DISCONNECTED);
  fakeClient.values.set({});
});

describe("createPageTagRealtimeProvider", () => {
  it("delegates start/stop to websocket client", () => {
    const provider = createPageTagRealtimeProvider(
      "ws://localhost:8787",
      fakeClient,
    );
    provider.start();
    provider.stop();

    expect(start).toHaveBeenCalledWith("ws://localhost:8787");
    expect(stop).toHaveBeenCalledTimes(1);
  });

  it("sends tracked ids only when active and deduplicates ids", () => {
    const provider = createPageTagRealtimeProvider(
      "ws://localhost:8787",
      fakeClient,
    );
    provider.setDesiredIds(["a", "a", "b"]);
    provider.setActive(true);

    expect(setTrackedIds).toHaveBeenLastCalledWith(["a", "b"]);

    provider.setActive(false);
    expect(setTrackedIds).toHaveBeenLastCalledWith([]);
  });

  it("does not resend identical effective tracked ids", () => {
    const provider = createPageTagRealtimeProvider(
      "ws://localhost:8787",
      fakeClient,
    );
    setTrackedIds.mockClear();

    provider.setDesiredIds(["a", "b"]);
    provider.setDesiredIds(["b", "a"]);
    provider.setDesiredIds(["a", "a", "b"]);
    expect(setTrackedIds).not.toHaveBeenCalled();

    provider.setActive(true);
    expect(setTrackedIds).toHaveBeenCalledTimes(1);
    expect(setTrackedIds).toHaveBeenLastCalledWith(["a", "b"]);

    provider.setDesiredIds(["b", "a"]);
    provider.setDesiredIds(["a", "b", "a"]);
    expect(setTrackedIds).toHaveBeenCalledTimes(1);
  });

  it("exposes shared status and values stores", () => {
    const provider = createPageTagRealtimeProvider(
      "ws://localhost:8787",
      fakeClient,
    );
    expect(get(provider.status)).toBe(WebSocketConnectionStatus.DISCONNECTED);
    expect(get(provider.values)).toEqual({});
  });

  it("delegates tag writes to websocket client", () => {
    const provider = createPageTagRealtimeProvider(
      "ws://localhost:8787",
      fakeClient,
    );
    provider.sendWriteValue("cmd.tag", 67);
    expect(sendWriteValue).toHaveBeenCalledWith("cmd.tag", 67);
  });

  it("delegates add/remove commands to websocket client", async () => {
    const provider = createPageTagRealtimeProvider(
      "ws://localhost:8787",
      fakeClient,
    );
    await provider.addItem("root", "x", "Folder", null);
    await provider.removeItems(["a"]);
    expect(addItem).toHaveBeenCalledWith(
      "root",
      "x",
      "Folder",
      null,
      "ws://localhost:8787",
    );
    expect(removeItems).toHaveBeenCalledWith(["a"], "ws://localhost:8787");
  });
});
