import { browser } from "$app/environment";
import { writable } from "svelte/store";
import {
  createAddCommand,
  createGetCommand,
  createSingleItemMeta,
  createDelCommand,
  createListCommand,
  createSetCommand,
  fromBackendValue,
} from "./command-ws-client";
import { WebSocketConnectionStatus, type TagScalarValue } from "./types";
import {
  Command,
  Response,
  type ListResponse,
} from "../../proto/namespace/commands";
import { ItemType, type VarDataType } from "../../proto/namespace/enums";

const RETRY_BASE_MS = 2000;
const RETRY_MAX_MS = 10000;
const POLL_MS = 2000;
const REQUEST_TIMEOUT_MS = 5000;

interface PendingListRequest {
  resolve: (value: ListResponse) => void;
  reject: (reason?: unknown) => void;
  timeoutId: ReturnType<typeof setTimeout>;
}

interface PendingAddRequest {
  resolve: (itemIds: string[]) => void;
  reject: (reason?: unknown) => void;
  timeoutId: ReturnType<typeof setTimeout>;
}

interface PendingDelRequest {
  resolve: () => void;
  reject: (reason?: unknown) => void;
  timeoutId: ReturnType<typeof setTimeout>;
}

export class TagStreamClient {
  private socket: WebSocket | null = null;
  private pollTimer: ReturnType<typeof setInterval> | null = null;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private retries = 0;
  private started = false;
  private connecting = false;
  private desiredIds = new Set<string>();
  private intentionallyClosed = false;
  private endpoint = "";
  private inflightByCmdId = new Map<string, string[]>();
  private inflightListByCmdId = new Map<string, PendingListRequest>();
  private inflightAddByCmdId = new Map<string, PendingAddRequest>();
  private inflightDelByCmdId = new Map<string, PendingDelRequest>();
  private connectionWaiters: Array<{
    resolve: () => void;
    reject: (reason?: unknown) => void;
  }> = [];

  public readonly status = writable<WebSocketConnectionStatus>(
    WebSocketConnectionStatus.DISCONNECTED,
  );
  public readonly values = writable<Record<string, TagScalarValue>>({});

  start(endpoint: string): void {
    if (!browser || this.started) {
      return;
    }

    this.endpoint = endpoint;
    this.started = true;
    this.intentionallyClosed = false;
    if (this.socket && this.socket.readyState === WebSocket.CONNECTING) {
      this.status.set(WebSocketConnectionStatus.CONNECTING);
      return;
    }
    if (this.socket && this.socket.readyState === WebSocket.OPEN) {
      this.status.set(WebSocketConnectionStatus.CONNECTED);
      this.startPolling();
      this.pollOnce();
      return;
    }
    this.connect(false);
  }

  stop(): void {
    this.intentionallyClosed = true;
    this.started = false;
    this.desiredIds.clear();
    this.inflightByCmdId.clear();
    this.rejectAllListRequests(new Error("Stopped"));
    this.rejectAllAddRequests(new Error("Stopped"));
    this.rejectAllDelRequests(new Error("Stopped"));
    this.clearReconnect();
    this.stopPolling();
    this.status.set(WebSocketConnectionStatus.DISCONNECTED);
    if (this.socket) {
      this.socket.close();
      this.socket = null;
    }
  }

  setTrackedIds(ids: string[]): void {
    this.desiredIds = new Set(ids);

    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) {
      return;
    }

    this.pollOnce();
  }

  sendWriteValue(id: string, value: TagScalarValue): void {
    if (!id) {
      return;
    }
    this.send(createSetCommand(id, value));
  }

  async listChildren(
    parentId: string | undefined,
    endpoint?: string,
  ): Promise<ListResponse> {
    await this.ensureConnected(endpoint);
    const { cmdId, command } = createListCommand(parentId);
    return new Promise<ListResponse>((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        this.inflightListByCmdId.delete(cmdId);
        reject(new Error("LIST request timed out"));
      }, REQUEST_TIMEOUT_MS);

      this.inflightListByCmdId.set(cmdId, { resolve, reject, timeoutId });
      this.send(command);
    });
  }

  async addItem(
    parentId: string,
    name: string,
    itemType: ItemType,
    varType: VarDataType | undefined,
    endpoint?: string,
  ): Promise<string[]> {
    await this.ensureConnected(endpoint);
    const { cmdId, command } = createAddCommand(parentId, [
      createSingleItemMeta(
        name,
        itemType,
        itemType === ItemType.ITEM_TYPE_FOLDER ? undefined : varType,
      ),
    ]);
    return new Promise<string[]>((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        this.inflightAddByCmdId.delete(cmdId);
        reject(new Error("ADD request timed out"));
      }, REQUEST_TIMEOUT_MS);

      this.inflightAddByCmdId.set(cmdId, { resolve, reject, timeoutId });
      this.send(command);
    });
  }

  async removeItems(itemIds: string[], endpoint?: string): Promise<void> {
    await this.ensureConnected(endpoint);
    const { cmdId, command } = createDelCommand(itemIds);
    return new Promise<void>((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        this.inflightDelByCmdId.delete(cmdId);
        reject(new Error("DEL request timed out"));
      }, REQUEST_TIMEOUT_MS);
      this.inflightDelByCmdId.set(cmdId, { resolve, reject, timeoutId });
      this.send(command);
    });
  }

  private async ensureConnected(endpoint?: string): Promise<void> {
    if (endpoint) {
      this.endpoint = endpoint;
    }
    if (!this.endpoint) {
      throw new Error("WebSocket endpoint is not configured");
    }
    if (this.socket && this.socket.readyState === WebSocket.OPEN) {
      return;
    }
    return new Promise<void>((resolve, reject) => {
      this.connectionWaiters.push({ resolve, reject });
      if (!this.socket && !this.connecting) {
        this.connect(false);
      }
    });
  }

  private connect(isRetry: boolean): void {
    if (!browser || !this.endpoint) {
      return;
    }
    this.connecting = true;

    if (this.started) {
      this.status.set(
        isRetry
          ? WebSocketConnectionStatus.RECONNECTING
          : WebSocketConnectionStatus.CONNECTING,
      );
    }

    this.socket = new WebSocket(this.endpoint);
    this.socket.binaryType = "arraybuffer";

    this.socket.addEventListener("open", () => {
      this.connecting = false;
      this.retries = 0;
      if (this.started) {
        this.status.set(WebSocketConnectionStatus.CONNECTED);
      }
      this.resolveConnectionWaiters();
      if (this.started) {
        this.startPolling();
        this.pollOnce();
      }
    });

    this.socket.addEventListener("message", (event) => {
      this.handleMessage(event.data);
    });

    this.socket.addEventListener("close", () => {
      this.connecting = false;
      this.stopPolling();
      this.socket = null;
      this.rejectConnectionWaiters(new Error("WebSocket connection closed"));
      this.rejectAllListRequests(new Error("WebSocket connection closed"));
      this.rejectAllAddRequests(new Error("WebSocket connection closed"));
      this.rejectAllDelRequests(new Error("WebSocket connection closed"));
      if (this.intentionallyClosed || !this.started) {
        if (this.started || this.intentionallyClosed) {
          this.status.set(WebSocketConnectionStatus.DISCONNECTED);
        }
        return;
      }
      this.scheduleReconnect();
    });

    this.socket.addEventListener("error", () => {
      if (this.socket && this.socket.readyState !== WebSocket.OPEN) {
        this.socket.close();
      }
      if (!this.socket || this.socket.readyState !== WebSocket.OPEN) {
        this.rejectConnectionWaiters(new Error("WebSocket connection failed"));
      }
    });
  }

  private scheduleReconnect(): void {
    this.clearReconnect();
    const delay = Math.min(RETRY_BASE_MS * 2 ** this.retries, RETRY_MAX_MS);
    this.retries += 1;
    this.reconnectTimer = setTimeout(() => {
      this.connect(true);
    }, delay);
  }

  private clearReconnect(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }

  private startPolling(): void {
    this.stopPolling();
    this.pollTimer = setInterval(() => {
      this.pollOnce();
    }, POLL_MS);
  }

  private stopPolling(): void {
    if (this.pollTimer) {
      clearInterval(this.pollTimer);
      this.pollTimer = null;
    }
  }

  private send(message: Command): void {
    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) {
      return;
    }
    const bytes = Command.encode(message).finish();
    this.socket.send(bytes);
  }

  private pollOnce(): void {
    if (this.desiredIds.size === 0) {
      return;
    }
    const ids = Array.from(this.desiredIds);
    const { cmdId, command } = createGetCommand(ids);
    this.inflightByCmdId.set(cmdId, ids);
    this.send(command);
  }

  private handleMessage(rawData: unknown): void {
    if (!(rawData instanceof ArrayBuffer)) {
      return;
    }

    try {
      const payload = Response.decode(new Uint8Array(rawData));
      const listPayload = payload.list;
      if (listPayload) {
        const request = this.inflightListByCmdId.get(listPayload.cmdId);
        if (!request) {
          return;
        }
        this.inflightListByCmdId.delete(listPayload.cmdId);
        clearTimeout(request.timeoutId);
        request.resolve(listPayload);
        return;
      }
      const getPayload = payload.get;
      if (!getPayload) {
        const addPayload = payload.add;
        if (addPayload) {
          const request = this.inflightAddByCmdId.get(addPayload.cmdId);
          if (!request) {
            return;
          }
          this.inflightAddByCmdId.delete(addPayload.cmdId);
          clearTimeout(request.timeoutId);
          request.resolve([]);
          return;
        }

        const delPayload = payload.del;
        if (delPayload) {
          const request = this.inflightDelByCmdId.get(delPayload.cmdId);
          if (!request) {
            return;
          }
          this.inflightDelByCmdId.delete(delPayload.cmdId);
          clearTimeout(request.timeoutId);
          request.resolve();
          return;
        }
        return;
      }
      const ids = this.inflightByCmdId.get(getPayload.cmdId);
      if (!ids) {
        return;
      }
      this.inflightByCmdId.delete(getPayload.cmdId);
      this.values.update((current) => {
        const next = { ...current };
        for (let index = 0; index < ids.length; index += 1) {
          const id = ids[index];
          const raw = getPayload.varValues[index];
          const parsed = fromBackendValue(raw?.value);
          if (parsed !== undefined) {
            next[id] = parsed;
          }
        }
        return next;
      });
    } catch {
      // Ignore malformed payloads or non-command text.
    }
  }

  private resolveConnectionWaiters(): void {
    for (const waiter of this.connectionWaiters) {
      waiter.resolve();
    }
    this.connectionWaiters = [];
  }

  private rejectConnectionWaiters(error: Error): void {
    for (const waiter of this.connectionWaiters) {
      waiter.reject(error);
    }
    this.connectionWaiters = [];
  }

  private rejectAllListRequests(error: Error): void {
    for (const [, request] of this.inflightListByCmdId) {
      clearTimeout(request.timeoutId);
      request.reject(error);
    }
    this.inflightListByCmdId.clear();
  }

  private rejectAllAddRequests(error: Error): void {
    for (const [, request] of this.inflightAddByCmdId) {
      clearTimeout(request.timeoutId);
      request.reject(error);
    }
    this.inflightAddByCmdId.clear();
  }

  private rejectAllDelRequests(error: Error): void {
    for (const [, request] of this.inflightDelByCmdId) {
      clearTimeout(request.timeoutId);
      request.reject(error);
    }
    this.inflightDelByCmdId.clear();
  }
}

export const tagStreamClient = new TagStreamClient();
