import type { Edge, Node, Viewport } from "@xyflow/svelte";

export interface ScadaView {
  id: string;
  name: string;
  description: string;
  is_entry_point: boolean;
  canvas_json: string;
  created_at: number;
  updated_at: number;
}

export interface ViewInputPayload {
  name: string;
  description: string;
  is_entry_point?: boolean;
  canvas_json: string;
}

export interface ViewsPage {
  items: ScadaView[];
  total: number;
  page: number;
  page_size: number;
  sort_by: "name" | "updated_at" | "is_entry_point";
  sort_direction: "asc" | "desc";
}

export type RightPaneMode = "views-list" | "view-editor";
export type CanvasMode = "edit" | "play";

interface SerializedCanvasState {
  version: number;
  nodes: Node[];
  edges: Edge[];
  viewport: Viewport;
}

function stripFunctions(_key: string, value: unknown): unknown {
  if (typeof value === "function") {
    return undefined;
  }
  return value;
}

type NodeRecord = Record<string, unknown>;

function stripBoundTagRuntimeMeta(tag: unknown): unknown {
  if (!tag || typeof tag !== "object") {
    return tag;
  }
  const record = tag as NodeRecord;
  const { min: _min, max: _max, ...rest } = record;
  return rest;
}

function sanitizePlantAssetNode(node: Node): Node {
  if (node.type !== "plantAsset") {
    return node;
  }

  const data = (node.data ?? {}) as NodeRecord;
  const rawBindings = data.bindings;
  const rawSourceNode = data.sourceNode;

  const sanitizedBindings =
    rawBindings && typeof rawBindings === "object"
      ? Object.fromEntries(
          Object.entries(rawBindings as Record<string, unknown>).map(
            ([bindingKey, tags]) => [
              bindingKey,
              Array.isArray(tags)
                ? tags.map((tag) => stripBoundTagRuntimeMeta(tag))
                : tags,
            ],
          ),
        )
      : rawBindings;

  const nextData: NodeRecord = {
    ...data,
    bindings: sanitizedBindings,
    sourceNode: stripBoundTagRuntimeMeta(rawSourceNode),
  };
  delete nextData.liveValue;
  delete nextData.liveValues;
  delete nextData.onWriteValue;
  delete nextData.onWriteBindingValue;
  delete nextData.onWidgetEvent;
  delete nextData.onOpenBindingConfig;

  return {
    ...node,
    data: nextData,
  };
}

export function serializeCanvasState(
  nodes: Node[],
  edges: Edge[],
  viewport: Viewport,
): string {
  const payload: SerializedCanvasState = {
    version: 1,
    nodes: nodes.map((node) => sanitizePlantAssetNode(node)),
    edges,
    viewport,
  };
  return JSON.stringify(payload, stripFunctions);
}

export function deserializeCanvasState(canvasJson: string): {
  nodes: Node[];
  edges: Edge[];
  viewport: Viewport;
} {
  if (!canvasJson || !canvasJson.trim()) {
    return { nodes: [], edges: [], viewport: { x: 0, y: 0, zoom: 1 } };
  }

  try {
    const raw = JSON.parse(canvasJson) as Partial<SerializedCanvasState>;
    const nodes = Array.isArray(raw.nodes)
      ? (raw.nodes as Node[]).map((node) => sanitizePlantAssetNode(node))
      : [];
    const edges = Array.isArray(raw.edges) ? (raw.edges as Edge[]) : [];
    const viewport =
      raw.viewport &&
      typeof raw.viewport === "object" &&
      typeof raw.viewport.x === "number" &&
      typeof raw.viewport.y === "number" &&
      typeof raw.viewport.zoom === "number"
        ? raw.viewport
        : { x: 0, y: 0, zoom: 1 };

    return { nodes, edges, viewport };
  } catch {
    return { nodes: [], edges: [], viewport: { x: 0, y: 0, zoom: 1 } };
  }
}
