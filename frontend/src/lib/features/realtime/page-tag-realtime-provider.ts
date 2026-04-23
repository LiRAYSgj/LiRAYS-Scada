import { derived, writable } from "svelte/store";
import { tagStreamClient } from "$lib/core/ws/tag-stream-client";
import type { Readable } from "svelte/store";
import type {
  TagScalarValue,
  WebSocketConnectionStatus,
} from "$lib/core/ws/types";
import type { ItemType, VarDataType } from "@lirays/scada-proto";

function uniqueIds(ids: string[]): string[] {
  const seen: Record<string, true> = {};
  const result: string[] = [];
  for (const id of ids) {
    if (!id || seen[id]) {
      continue;
    }
    seen[id] = true;
    result.push(id);
  }
  return result;
}

function hasSameIdSet(current: string[], next: string[]): boolean {
  if (current.length !== next.length) {
    return false;
  }
  const nextSet = new Set(next);
  for (const id of current) {
    if (!nextSet.has(id)) {
      return false;
    }
  }
  return true;
}

export interface TagRealtimeClient {
  status: Readable<WebSocketConnectionStatus>;
  values: Readable<Record<string, TagScalarValue>>;
  start: (endpoint: string) => void;
  stop: () => void;
  setTrackedIds: (ids: string[]) => void;
  sendWriteValue: (
    id: string,
    value: TagScalarValue,
    endpoint?: string,
  ) => Promise<void>;
  addItem: (
    parentId: string | null,
    name: string,
    itemType: ItemType,
    varType: VarDataType | undefined,
    meta?: {
      unit?: string;
      min?: number;
      max?: number;
      options?: string[];
      maxLen?: number;
    },
    endpoint?: string,
  ) => Promise<string[]>;
  removeItems: (itemIds: string[], endpoint?: string) => Promise<void>;
  updateMeta: (
    varId: string,
    meta: {
      unit?: string;
      min?: number;
      max?: number;
      options?: string[];
      maxLen?: number;
    },
    endpoint?: string,
  ) => Promise<void>;
}

export function createPageTagRealtimeProvider(
  endpoint: string,
  client: TagRealtimeClient = tagStreamClient,
) {
  const active = writable(false);
  const desiredIds = writable<string[]>([]);
  let lastEffectiveIds: string[] | null = null;

  const effectiveIds = derived(
    [active, desiredIds],
    ([$active, $desiredIds]) => ($active ? uniqueIds($desiredIds) : []),
  );

  const unsubscribeEffectiveIds = effectiveIds.subscribe((ids) => {
    if (lastEffectiveIds && hasSameIdSet(lastEffectiveIds, ids)) {
      return;
    }
    lastEffectiveIds = [...ids];
    client.setTrackedIds(ids);
  });

  return {
    status: client.status,
    values: client.values,
    start: () => {
      client.start(endpoint);
    },
    stop: () => {
      unsubscribeEffectiveIds();
      client.stop();
    },
    setActive: (next: boolean) => {
      active.set(next);
    },
    setDesiredIds: (ids: string[]) => {
      desiredIds.set(ids);
    },
    sendWriteValue: (id: string, value: TagScalarValue) => {
      return client.sendWriteValue(id, value, endpoint);
    },
    addItem: (
      parentId: string | null,
      name: string,
      itemType: ItemType,
      varType: VarDataType | undefined,
      meta?: {
        unit?: string;
        min?: number;
        max?: number;
        options?: string[];
        maxLen?: number;
      },
    ) => {
      return client.addItem(parentId, name, itemType, varType, meta, endpoint);
    },
    removeItems: (itemIds: string[]) => {
      return client.removeItems(itemIds, endpoint);
    },
    updateMeta: (
      varId: string,
      meta: {
        unit?: string;
        min?: number;
        max?: number;
        options?: string[];
        maxLen?: number;
      },
    ) => {
      return client.updateMeta(varId, meta, endpoint);
    },
  };
}
