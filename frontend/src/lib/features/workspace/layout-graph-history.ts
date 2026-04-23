import type { Edge, Node, Viewport } from "@xyflow/svelte";
import type { PlantAssetNodeData } from "$lib/features/graph/assets/types";

export interface GraphHistorySnapshot {
  nodes: Node[];
  edges: Edge[];
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  if (!value || typeof value !== "object") {
    return false;
  }
  const prototype = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
}

function cloneSanitized(
  value: unknown,
  seen = new WeakMap<object, unknown>(),
): unknown {
  if (value === null || value === undefined) {
    return value;
  }

  const valueType = typeof value;
  if (
    valueType === "string" ||
    valueType === "number" ||
    valueType === "boolean" ||
    valueType === "bigint"
  ) {
    return value;
  }
  if (valueType === "function" || valueType === "symbol") {
    return undefined;
  }

  if (Array.isArray(value)) {
    return value
      .map((item) => cloneSanitized(item, seen))
      .filter((item) => item !== undefined);
  }

  if (valueType === "object") {
    const objectValue = value as object;
    if (seen.has(objectValue)) {
      return seen.get(objectValue);
    }

    if (
      (typeof Window !== "undefined" && objectValue instanceof Window) ||
      (typeof Document !== "undefined" && objectValue instanceof Document) ||
      (typeof EventTarget !== "undefined" && objectValue instanceof EventTarget)
    ) {
      return undefined;
    }

    if (!isPlainObject(value)) {
      return undefined;
    }

    const result: Record<string, unknown> = {};
    seen.set(objectValue, result);
    for (const [key, entryValue] of Object.entries(value)) {
      const cloned = cloneSanitized(entryValue, seen);
      if (cloned !== undefined) {
        result[key] = cloned;
      }
    }
    return result;
  }

  return undefined;
}

export function cloneForHistory<T>(value: T): T {
  const cloned = cloneSanitized(value);
  if (cloned === undefined) {
    throw new Error("Unable to clone graph state for history.");
  }
  return cloned as T;
}

function sanitizeNodeForHistory(node: Node): Node {
  const next = cloneForHistory(node) as Node & {
    selected?: boolean;
    dragging?: boolean;
    resizing?: boolean;
  };
  delete next.selected;
  delete next.dragging;
  delete next.resizing;

  if (next.type !== "plantAsset") {
    return next;
  }

  const data = { ...(next.data as PlantAssetNodeData) };
  delete data.liveValue;
  delete data.liveValues;
  delete data.onWriteValue;
  delete data.onWriteBindingValue;
  delete data.onWidgetEvent;
  delete data.onOpenBindingConfig;
  delete data.graphReadOnly;
  delete data.interactionMode;
  next.data = data;
  return next;
}

function sanitizeEdgeForHistory(edge: Edge): Edge {
  const next = cloneForHistory(edge) as Edge & { selected?: boolean };
  delete next.selected;
  return next;
}

export function createGraphHistorySnapshot(
  nodes: Node[],
  edges: Edge[],
): GraphHistorySnapshot {
  return {
    nodes: nodes.map((node) => sanitizeNodeForHistory(node)),
    edges: edges.map((edge) => sanitizeEdgeForHistory(edge)),
  };
}

export function snapshotSignature(snapshot: GraphHistorySnapshot): string {
  let hash = 2166136261;
  const pushString = (value: string): void => {
    for (let index = 0; index < value.length; index += 1) {
      hash ^= value.charCodeAt(index);
      hash = Math.imul(hash, 16777619);
    }
  };
  const pushUnknown = (value: unknown): void => {
    if (value === null) {
      pushString("null");
      return;
    }
    if (value === undefined) {
      pushString("undefined");
      return;
    }
    if (Array.isArray(value)) {
      pushString("[");
      for (const item of value) {
        pushUnknown(item);
        pushString(",");
      }
      pushString("]");
      return;
    }
    const valueType = typeof value;
    if (valueType === "string") {
      pushString(`s:${value}`);
      return;
    }
    if (valueType === "number") {
      pushString(`n:${value}`);
      return;
    }
    if (valueType === "boolean") {
      pushString(value ? "b:1" : "b:0");
      return;
    }
    if (valueType === "bigint") {
      pushString(`bi:${value.toString()}`);
      return;
    }
    if (valueType === "object") {
      pushString("{");
      for (const key of Object.keys(value as Record<string, unknown>).sort()) {
        pushString(key);
        pushString(":");
        pushUnknown((value as Record<string, unknown>)[key]);
        pushString(";");
      }
      pushString("}");
    }
  };
  pushUnknown(snapshot);
  return `h${hash >>> 0}`;
}

export function buildNextEntityId(
  previousId: string,
  nextCounter: number,
): string {
  const base = previousId.replace(/-\d+$/, "");
  if (base.length === 0 || base === previousId) {
    return `${previousId}-${nextCounter}`;
  }
  return `${base}-${nextCounter}`;
}

function computeNodeBounds(nodes: Node[]): {
  x: number;
  y: number;
  width: number;
  height: number;
} | null {
  const assets = nodes.filter((node) => node.type === "plantAsset");
  if (assets.length === 0) {
    return null;
  }

  let minX = Number.POSITIVE_INFINITY;
  let minY = Number.POSITIVE_INFINITY;
  let maxX = Number.NEGATIVE_INFINITY;
  let maxY = Number.NEGATIVE_INFINITY;

  for (const node of assets) {
    const width = typeof node.width === "number" ? node.width : 240;
    const height = typeof node.height === "number" ? node.height : 160;
    minX = Math.min(minX, node.position.x);
    minY = Math.min(minY, node.position.y);
    maxX = Math.max(maxX, node.position.x + width);
    maxY = Math.max(maxY, node.position.y + height);
  }

  if (!Number.isFinite(minX) || !Number.isFinite(minY)) {
    return null;
  }

  return {
    x: minX,
    y: minY,
    width: Math.max(1, maxX - minX),
    height: Math.max(1, maxY - minY),
  };
}

export function resolveInitialEditorViewport(
  savedViewport: Viewport,
  nodes: Node[],
  canvasWidth = 1280,
  canvasHeight = 720,
): Viewport {
  const bounds = computeNodeBounds(nodes);
  if (!bounds) {
    return savedViewport;
  }

  const zoom = Number.isFinite(savedViewport.zoom)
    ? Math.min(1.6, Math.max(0.4, savedViewport.zoom))
    : 1;
  const contentCenterX = bounds.x + bounds.width / 2;
  const contentCenterY = bounds.y + bounds.height / 2;
  const x = canvasWidth / 2 - contentCenterX * zoom;
  const y = canvasHeight / 2 - contentCenterY * zoom;

  return { x, y, zoom };
}
