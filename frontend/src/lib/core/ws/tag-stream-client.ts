import { browser } from "$app/environment";
import { writable } from "svelte/store";
import {
  createAddCommand,
  createAddBulkCommand,
  createSingleItemMeta,
  createDelCommand,
  createSubscribeCommand,
  createTreeChangeSubscribeCommand,
  createUnsubscribeCommand,
  createListCommand,
  createSetCommand,
  fromBackendValue,
  namespaceJsonToSchema,
} from "./command-ws-client";
import { WebSocketConnectionStatus, type TagScalarValue } from "./types";
import {
  Command,
  Response,
  type ListResponse,
} from "../../proto/namespace/commands";
import {
  VarIdValue as VarIdValueMessage,
  type VarIdValue,
} from "../../proto/namespace/types";
import { Event, type TreeChanged } from "../../proto/namespace/events";
import {
  EventType,
  ItemType,
  OperationStatus,
  type VarDataType,
} from "../../proto/namespace/enums";
import { snackbarStore } from "../../stores/snackbar";

const RETRY_BASE_MS = 2000;
const RETRY_MAX_MS = 10000;
/** All commands use the same timeout; success/failure is determined by Response.status and optional error_msg. */
const COMMAND_TIMEOUT_MS = 3600_000;

const TIMEOUT_USER_MESSAGE = "Requested operation timed out. Try again.";

type PendingType =
  | "list"
  | "add"
  | "set"
  | "del"
  | "add_bulk"
  | "sub"
  | "unsub";

interface PendingCommand {
  type: PendingType;
  resolve: (value?: unknown) => void;
  reject: (reason?: unknown) => void;
  timeoutId: ReturnType<typeof setTimeout>;
}

export class TagStreamClient {
  private socket: WebSocket | null = null;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private retries = 0;
  private started = false;
  private connecting = false;
  private desiredIds = new Set<string>();
  private intentionallyClosed = false;
  private endpoint = "";
  private pendingByCmdId = new Map<string, PendingCommand>();
  private connectionWaiters: Array<{
    resolve: () => void;
    reject: (reason?: unknown) => void;
  }> = [];

  public readonly status = writable<WebSocketConnectionStatus>(
    WebSocketConnectionStatus.DISCONNECTED,
  );
  public readonly values = writable<Record<string, TagScalarValue>>({});
  /** Latest tree sync payload from other UI instances (global subscription). */
  public readonly treeChanges = writable<TreeChanged | undefined>(undefined);

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
      this.wireSubscriptionsAfterOpen();
      return;
    }
    this.connect(false);
  }

  stop(): void {
    this.intentionallyClosed = true;
    this.started = false;
    this.desiredIds.clear();
    this.rejectAllPending(new Error("Stopped"));
    this.clearReconnect();
    this.status.set(WebSocketConnectionStatus.DISCONNECTED);
    if (this.socket) {
      this.socket.close();
      this.socket = null;
    }
  }

  setTrackedIds(ids: string[]): void {
    const prevIds = this.desiredIds;
    this.desiredIds = new Set(ids);

    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) {
      // Not connected yet; will subscribe on connect open.
      return;
    }

    const nextSet = this.desiredIds;
    const toSubscribe: string[] = [];
    const toUnsubscribe: string[] = [];

    for (const id of nextSet) {
      if (!prevIds.has(id)) toSubscribe.push(id);
    }
    for (const id of prevIds) {
      if (!nextSet.has(id)) toUnsubscribe.push(id);
    }

    void this.subscribeIds(toSubscribe);
    void this.unsubscribeIds(toUnsubscribe);
  }

  async sendWriteValue(
    id: string,
    value: TagScalarValue,
    endpoint?: string,
  ): Promise<void> {
    if (!id) {
      return;
    }
    await this.ensureConnected(endpoint);
    const { cmdId, command } = createSetCommand(id, value);
    return new Promise<void>((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        this.pendingByCmdId.delete(cmdId);
        snackbarStore.error(TIMEOUT_USER_MESSAGE);
        reject(new Error("SET request timed out"));
      }, COMMAND_TIMEOUT_MS);
      this.pendingByCmdId.set(cmdId, {
        type: "set",
        resolve: () => resolve(),
        reject,
        timeoutId,
      });
      this.send(command);
    });
  }

  async listChildren(
    parentId: string | undefined,
    endpoint?: string,
  ): Promise<ListResponse> {
    await this.ensureConnected(endpoint);
    const { cmdId, command } = createListCommand(parentId);
    return new Promise<ListResponse>((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        this.pendingByCmdId.delete(cmdId);
        snackbarStore.error(TIMEOUT_USER_MESSAGE);
        reject(new Error("LIST request timed out"));
      }, COMMAND_TIMEOUT_MS);
      this.pendingByCmdId.set(cmdId, {
        type: "list",
        resolve: (v) => resolve(v as ListResponse),
        reject,
        timeoutId,
      });
      this.send(command);
    });
  }

  async addItem(
    parentId: string | null,
    name: string,
    itemType: ItemType,
    varType: VarDataType | undefined,
    endpoint?: string,
  ): Promise<string[]> {
    await this.ensureConnected(endpoint);
    const { cmdId, command } = createAddCommand(parentId ?? "", [
      createSingleItemMeta(
        name,
        itemType,
        itemType === ItemType.ITEM_TYPE_FOLDER ? undefined : varType,
      ),
    ]);
    return new Promise<string[]>((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        this.pendingByCmdId.delete(cmdId);
        snackbarStore.error(TIMEOUT_USER_MESSAGE);
        reject(new Error("ADD request timed out"));
      }, COMMAND_TIMEOUT_MS);
      this.pendingByCmdId.set(cmdId, {
        type: "add",
        resolve: () => resolve([] as string[]),
        reject,
        timeoutId,
      });
      this.send(command);
    });
  }

  /**
   * Send AddBulkCommand; resolves when Response has status OK for that cmd_id.
   * @param parentId Root parent for bulk tree (often "").
   * @param json Nested object from buildNamespaceJsonFromYaml (leaves = type strings).
   */
  async addBulkNamespace(
    parentId: string,
    json: Record<string, unknown>,
    endpoint?: string,
  ): Promise<void> {
    await this.ensureConnected(endpoint);
    const schema = namespaceJsonToSchema(json);
    const { cmdId, command } = createAddBulkCommand(parentId, schema);
    return new Promise<void>((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        this.pendingByCmdId.delete(cmdId);
        snackbarStore.error(TIMEOUT_USER_MESSAGE);
        reject(new Error("ADD_BULK request timed out"));
      }, COMMAND_TIMEOUT_MS);
      this.pendingByCmdId.set(cmdId, {
        type: "add_bulk",
        resolve: () => resolve(),
        reject,
        timeoutId,
      });
      this.send(command);
    });
  }

  async removeItems(itemIds: string[], endpoint?: string): Promise<void> {
    await this.ensureConnected(endpoint);
    const { cmdId, command } = createDelCommand(itemIds);
    return new Promise<void>((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        this.pendingByCmdId.delete(cmdId);
        snackbarStore.error(TIMEOUT_USER_MESSAGE);
        reject(new Error("DEL request timed out"));
      }, COMMAND_TIMEOUT_MS);
      this.pendingByCmdId.set(cmdId, {
        type: "del",
        resolve: () => resolve(),
        reject,
        timeoutId,
      });
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
        this.wireSubscriptionsAfterOpen();
      }
    });

    this.socket.addEventListener("message", (event) => {
      this.handleMessage(event.data);
    });

    this.socket.addEventListener("close", () => {
      this.connecting = false;
      this.socket = null;
      this.rejectConnectionWaiters(new Error("WebSocket connection closed"));
      this.rejectAllPending(new Error("WebSocket connection closed"));
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

  private send(message: Command): void {
    console.log("Sending message:", message);
    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) {
      return;
    }
    const bytes = Command.encode(message).finish();
    this.socket.send(bytes);
  }

  private wireSubscriptionsAfterOpen(): void {
    this.sendGlobalTreeSubscribe();
    this.subscribeAllTrackedIds();
  }

  /** Global tree-change subscription for the whole session (re-sent on reconnect). */
  private sendGlobalTreeSubscribe(): void {
    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) {
      return;
    }
    const { cmdId, command } = createTreeChangeSubscribeCommand();
    this.pendingByCmdId.set(cmdId, {
      type: "sub",
      resolve: () => {},
      reject: () => {
        snackbarStore.error("Tree subscription request failed");
      },
      timeoutId: setTimeout(() => {
        this.pendingByCmdId.delete(cmdId);
      }, COMMAND_TIMEOUT_MS),
    });
    this.send(command);
  }

  private subscribeAllTrackedIds(): void {
    void this.subscribeIds(Array.from(this.desiredIds));
  }

  private async subscribeIds(ids: string[]): Promise<void> {
    if (ids.length === 0) return;
    await this.ensureConnected();
    const { cmdId, command } = createSubscribeCommand(ids, [
      EventType.EVENT_TYPE_VAR_VALUES,
    ]);
    this.pendingByCmdId.set(cmdId, {
      type: "sub",
      resolve: () => {},
      reject: () => {
        snackbarStore.error("Subscribe request failed");
      },
      timeoutId: setTimeout(() => {
        this.pendingByCmdId.delete(cmdId);
      }, COMMAND_TIMEOUT_MS),
    });
    this.send(command);
  }

  private async unsubscribeIds(ids: string[]): Promise<void> {
    if (ids.length === 0) return;
    await this.ensureConnected();
    const { cmdId, command } = createUnsubscribeCommand(ids, [
      EventType.EVENT_TYPE_VAR_VALUES,
    ]);
    this.pendingByCmdId.set(cmdId, {
      type: "unsub",
      resolve: () => {},
      reject: () => {
        snackbarStore.error("Unsubscribe request failed");
      },
      timeoutId: setTimeout(() => {
        this.pendingByCmdId.delete(cmdId);
      }, COMMAND_TIMEOUT_MS),
    });
    this.send(command);
  }

  /**
   * Called when a response has a non-terminal status (e.g. future Pending/Progress).
   * Resets the timeout for that command so the client keeps waiting.
   * Add new status values here when backend introduces them.
   */
  private resetCommandTimeout(cmdId: string): void {
    const pending = this.pendingByCmdId.get(cmdId);
    if (!pending) return;
    clearTimeout(pending.timeoutId);
    pending.timeoutId = setTimeout(() => {
      this.pendingByCmdId.delete(cmdId);
      pending.reject(new Error("Command timed out"));
    }, COMMAND_TIMEOUT_MS);
  }

  private extractResponseCmdId(payload: Response): string {
    return (
      payload.list?.cmdId ??
      payload.get?.cmdId ??
      payload.set?.cmdId ??
      payload.add?.cmdId ??
      payload.del?.cmdId ??
      payload.sub?.cmdId ??
      payload.unsub?.cmdId ??
      payload.inv?.cmdId ??
      payload.addBulk?.cmdId ??
      ""
    );
  }

  /**
   * `Event` and `Response` share overlapping wire tags (e.g. field 1 / tag 10), so a pushed
   * `Event.var_value_ev` can be mis-parsed as `Response.add`. If the decoded cmd_id is not a
   * pending command, treat the frame as an `Event` push instead.
   */
  private tryHandleEventPush(bytes: Uint8Array): boolean {
    try {
      const push = Event.decode(bytes);
      if (push.varValueEv) {
        const varUpdate = push.varValueEv;
        if (!varUpdate.varId) return false;
        if (!this.desiredIds.has(varUpdate.varId)) return false;
        const parsed = fromBackendValue(varUpdate.value);
        this.values.update((current) => {
          const next = { ...current };
          if (parsed === undefined) {
            delete next[varUpdate.varId];
          } else {
            next[varUpdate.varId] = parsed;
          }
          return next;
        });
        return true;
      }
      if (push.treeChangedEv !== undefined && push.treeChangedEv !== null) {
        this.treeChanges.set(push.treeChangedEv);
        return true;
      }
    } catch {
      // ignore
    }
    return false;
  }

  private handleMessage(rawData: unknown): void {
    if (!(rawData instanceof ArrayBuffer)) {
      return;
    }

    const bytes = new Uint8Array(rawData);

    try {
      const payload = Response.decode(bytes);
      const hasEnvelopeType =
        payload.add != null ||
        payload.list != null ||
        payload.set != null ||
        payload.get != null ||
        payload.del != null ||
        payload.addBulk != null ||
        payload.sub != null ||
        payload.unsub != null ||
        payload.inv != null;

      if (hasEnvelopeType) {
        const cmdId = this.extractResponseCmdId(payload);
        const pendingMatch = cmdId !== "" && this.pendingByCmdId.has(cmdId);
        if (!pendingMatch && this.tryHandleEventPush(bytes)) {
          return;
        }

        console.log("Received message:", payload);
        const errorMsg = payload.errorMsg ?? "";
        const status =
          payload.status ?? OperationStatus.OPERATION_STATUS_INVALID;

        const isTerminalError =
          status === OperationStatus.OPERATION_STATUS_ERR ||
          status === OperationStatus.OPERATION_STATUS_INVALID ||
          payload.inv != null;
        const isTerminalSuccess =
          status === OperationStatus.OPERATION_STATUS_OK;

        if (isTerminalError && cmdId) {
          const pending = this.pendingByCmdId.get(cmdId);
          const message =
            errorMsg ||
            (status === OperationStatus.OPERATION_STATUS_INVALID
              ? "Invalid operation status"
              : "Operation failed");
          if (pending) {
            clearTimeout(pending.timeoutId);
            this.pendingByCmdId.delete(cmdId);
            snackbarStore.error(message);
            pending.reject(new Error(message));
          }
          return;
        }

        if (isTerminalSuccess) {
          const pending = cmdId ? this.pendingByCmdId.get(cmdId) : undefined;
          if (pending) {
            clearTimeout(pending.timeoutId);
            this.pendingByCmdId.delete(cmdId);
            if (pending.type === "list" && payload.list != null) {
              pending.resolve(payload.list);
            } else {
              pending.resolve();
            }
          }
          return;
        }

        // Non-terminal status (e.g. future Pending/Progress): reset timeout and keep waiting.
        if (cmdId) {
          this.resetCommandTimeout(cmdId);
        }
        return;
      }
    } catch {
      // Not a Response envelope. Fall through to Event / legacy VarIdValue decode.
    }

    if (this.tryHandleEventPush(bytes)) {
      return;
    }

    try {
      // Legacy: bare VarIdValue pushes.
      const varUpdate: VarIdValue = VarIdValueMessage.decode(bytes);
      if (!varUpdate.varId) return;
      if (!this.desiredIds.has(varUpdate.varId)) return;
      const parsed = fromBackendValue(varUpdate.value);
      this.values.update((current) => {
        const next = { ...current };
        if (parsed === undefined) {
          delete next[varUpdate.varId];
        } else {
          next[varUpdate.varId] = parsed;
        }
        return next;
      });
    } catch {
      // Ignore malformed updates
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

  private rejectAllPending(error: Error): void {
    for (const [, pending] of this.pendingByCmdId) {
      clearTimeout(pending.timeoutId);
      pending.reject(error);
    }
    this.pendingByCmdId.clear();
  }
}

export const tagStreamClient = new TagStreamClient();
