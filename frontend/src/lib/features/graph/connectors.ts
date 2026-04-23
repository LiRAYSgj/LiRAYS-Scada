import { MarkerType, type Connection, type Edge } from "@xyflow/svelte";

export const CONNECTOR_EDGE_TYPE_ARROW = "connector-arrow";
export const CONNECTOR_EDGE_TYPE_PIPE = "connector-pipe";

export type ConnectorStyle = "arrow" | "pipe";

export interface ArrowConnectorConfig {
  color: string;
  thickness: number;
  arrowSize: number;
}

export interface PipeConnectorConfig {
  thickness: number;
  flangeScale: number;
}

export interface ConnectorData {
  style: ConnectorStyle;
  arrow: ArrowConnectorConfig;
  pipe: PipeConnectorConfig;
}

export const DEFAULT_ARROW_CONNECTOR: ArrowConnectorConfig = {
  color: "#4d5e75",
  thickness: 3,
  arrowSize: 16,
};
export const DEFAULT_PIPE_CONNECTOR: PipeConnectorConfig = {
  thickness: 14,
  flangeScale: 1,
};

export const DEFAULT_PIPE_CONNECTION_LINE_STYLE =
  "stroke:#5b708a;stroke-width:8;";
type ConnectorEdgeLike = Pick<Edge, "id" | "type" | "style" | "data">;

function isMarkerEndEqual(
  left: Edge["markerEnd"],
  right: Edge["markerEnd"],
): boolean {
  if (left === right) {
    return true;
  }
  if (!left || !right) {
    return left === right;
  }
  if (typeof left === "string" || typeof right === "string") {
    return left === right;
  }
  const leftObj = left as Record<string, unknown>;
  const rightObj = right as Record<string, unknown>;
  return (
    leftObj.type === rightObj.type &&
    leftObj.color === rightObj.color &&
    leftObj.width === rightObj.width &&
    leftObj.height === rightObj.height &&
    leftObj.markerUnits === rightObj.markerUnits &&
    leftObj.orient === rightObj.orient
  );
}

function isConnectorDataEqual(
  left: ConnectorData,
  right: ConnectorData,
): boolean {
  return (
    left.style === right.style &&
    left.arrow.color === right.arrow.color &&
    left.arrow.thickness === right.arrow.thickness &&
    left.arrow.arrowSize === right.arrow.arrowSize &&
    left.pipe.thickness === right.pipe.thickness &&
    left.pipe.flangeScale === right.pipe.flangeScale
  );
}

function markerForArrow(arrow: ArrowConnectorConfig): Edge["markerEnd"] {
  return {
    type: MarkerType.ArrowClosed,
    color: arrow.color,
    width: arrow.arrowSize,
    height: arrow.arrowSize,
    markerUnits: "userSpaceOnUse",
    orient: "auto",
  };
}

function styleForArrow(arrow: ArrowConnectorConfig): string {
  return `stroke:${arrow.color};stroke-width:${arrow.thickness};`;
}

function normalizeEdgeDirection<T extends Edge>(edge: T): T {
  return edge;
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  if (!value || typeof value !== "object") {
    return false;
  }
  const prototype = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
}

function toRecord(value: unknown): Record<string, unknown> {
  return isPlainObject(value) ? value : {};
}

function asFiniteNumber(value: unknown): number | null {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return null;
  }
  return value;
}

function clampNumber(
  value: number | null,
  min: number,
  max: number,
  fallback: number,
): number {
  if (value === null) {
    return fallback;
  }
  return Math.max(min, Math.min(max, value));
}

function sanitizeHexColor(value: unknown, fallback: string): string {
  if (typeof value !== "string") {
    return fallback;
  }
  const trimmed = value.trim();
  return /^#[0-9a-fA-F]{6}$/.test(trimmed) || /^#[0-9a-fA-F]{3}$/.test(trimmed)
    ? trimmed
    : fallback;
}

function parseEdgeStyle(style: unknown): {
  stroke?: string;
  strokeWidth?: number;
} {
  if (typeof style !== "string") {
    return {};
  }
  const parsed: { stroke?: string; strokeWidth?: number } = {};
  for (const chunk of style.split(";")) {
    const [rawKey, rawValue] = chunk.split(":");
    if (!rawKey || !rawValue) {
      continue;
    }
    const key = rawKey.trim().toLowerCase();
    const value = rawValue.trim();
    if (key === "stroke") {
      parsed.stroke = value;
      continue;
    }
    if (key === "stroke-width") {
      const width = Number(value);
      if (Number.isFinite(width)) {
        parsed.strokeWidth = width;
      }
    }
  }
  return parsed;
}

export function resolveConnectorStyle(edge: ConnectorEdgeLike): ConnectorStyle {
  const data = toRecord(edge.data);
  const connector = toRecord(data.connector);
  const style = connector.style;
  if (style === "arrow" || style === "pipe") {
    return style;
  }
  if (edge.type === CONNECTOR_EDGE_TYPE_PIPE) {
    return "pipe";
  }
  if (edge.type === CONNECTOR_EDGE_TYPE_ARROW) {
    return "arrow";
  }
  return "arrow";
}

function resolveArrowConnector(edge: ConnectorEdgeLike): ArrowConnectorConfig {
  const parsedStyle = parseEdgeStyle(edge.style);
  const data = toRecord(edge.data);
  const connector = toRecord(data.connector);
  const arrow = toRecord(connector.arrow);
  return {
    color: sanitizeHexColor(
      arrow.color ?? connector.color ?? parsedStyle.stroke,
      DEFAULT_ARROW_CONNECTOR.color,
    ),
    thickness: clampNumber(
      asFiniteNumber(
        arrow.thickness ?? connector.thickness ?? parsedStyle.strokeWidth,
      ),
      1,
      20,
      DEFAULT_ARROW_CONNECTOR.thickness,
    ),
    arrowSize: clampNumber(
      asFiniteNumber(arrow.arrowSize ?? connector.arrowSize),
      6,
      64,
      DEFAULT_ARROW_CONNECTOR.arrowSize,
    ),
  };
}

function resolvePipeConnector(edge: ConnectorEdgeLike): PipeConnectorConfig {
  const parsedStyle = parseEdgeStyle(edge.style);
  const data = toRecord(edge.data);
  const connector = toRecord(data.connector);
  const pipe = toRecord(connector.pipe);
  const styleThickness =
    edge.type === CONNECTOR_EDGE_TYPE_PIPE
      ? parsedStyle.strokeWidth
      : undefined;
  return {
    thickness: clampNumber(
      asFiniteNumber(pipe.thickness ?? connector.thickness ?? styleThickness),
      2,
      64,
      DEFAULT_PIPE_CONNECTOR.thickness,
    ),
    flangeScale: Math.round(
      clampNumber(
        asFiniteNumber(pipe.flangeScale ?? connector.flangeScale),
        1,
        8,
        DEFAULT_PIPE_CONNECTOR.flangeScale,
      ),
    ),
  };
}

export function resolveConnectorData(edge: ConnectorEdgeLike): ConnectorData {
  return {
    style: resolveConnectorStyle(edge),
    arrow: resolveArrowConnector(edge),
    pipe: resolvePipeConnector(edge),
  };
}

export function resolveArrowConnectorFromEdge(
  edge: ConnectorEdgeLike,
): ArrowConnectorConfig {
  return resolveConnectorData(edge).arrow;
}

export function resolvePipeConnectorFromEdge(
  edge: ConnectorEdgeLike,
): PipeConnectorConfig {
  return resolveConnectorData(edge).pipe;
}

export function edgeTypeForConnectorStyle(style: ConnectorStyle): string {
  return style === "arrow"
    ? CONNECTOR_EDGE_TYPE_ARROW
    : CONNECTOR_EDGE_TYPE_PIPE;
}

export function normalizeConnectorEdge(edge: Edge): Edge {
  const directionNormalized = normalizeEdgeDirection(edge);
  const connector = resolveConnectorData(edge);
  const data = toRecord(edge.data);
  const nextData = {
    ...data,
    connector,
  };
  const markerEnd =
    connector.style === "arrow" ? markerForArrow(connector.arrow) : undefined;
  const style =
    connector.style === "arrow" ? styleForArrow(connector.arrow) : undefined;
  return {
    ...directionNormalized,
    type: edgeTypeForConnectorStyle(connector.style),
    animated: false,
    style,
    markerEnd,
    data: nextData,
  };
}

export function normalizeConnectorEdges(edges: Edge[]): {
  edges: Edge[];
  changed: boolean;
} {
  let changed = false;
  const normalized = edges.map((edge) => {
    const next = normalizeConnectorEdge(edge);
    const currentConnector = resolveConnectorData(edge);
    const nextConnector = resolveConnectorData(next);
    if (
      next.type === edge.type &&
      edge.animated === false &&
      next.style === edge.style &&
      isMarkerEndEqual(next.markerEnd, edge.markerEnd) &&
      isConnectorDataEqual(currentConnector, nextConnector)
    ) {
      return edge;
    }
    changed = true;
    return next;
  });
  return { edges: normalized, changed };
}

export function createDefaultConnectorEdge(
  connection: Connection,
  id: string,
): Edge {
  return normalizeConnectorEdge({
    ...connection,
    id,
    type: CONNECTOR_EDGE_TYPE_ARROW,
    animated: false,
    data: {
      connector: {
        style: "arrow",
        arrow: { ...DEFAULT_ARROW_CONNECTOR },
        pipe: { ...DEFAULT_PIPE_CONNECTOR },
      },
    },
  } as Edge);
}

export function applyConnectorStyleToEdge(
  edge: Edge,
  style: ConnectorStyle,
): Edge {
  const connector = resolveConnectorData(edge);
  const nextConnector: ConnectorData = {
    ...connector,
    style,
  };
  const data = toRecord(edge.data);
  const markerEnd =
    style === "arrow" ? markerForArrow(nextConnector.arrow) : undefined;
  const nextStyle =
    style === "arrow" ? styleForArrow(nextConnector.arrow) : undefined;
  return {
    ...edge,
    type: edgeTypeForConnectorStyle(style),
    animated: false,
    style: nextStyle,
    markerEnd,
    data: {
      ...data,
      connector: nextConnector,
    },
  };
}

export function applyArrowConnectorPatchToEdge(
  edge: Edge,
  patch: Partial<ArrowConnectorConfig>,
): Edge {
  const connector = resolveConnectorData(edge);
  const nextArrow: ArrowConnectorConfig = {
    color: sanitizeHexColor(patch.color, connector.arrow.color),
    thickness: clampNumber(
      asFiniteNumber(patch.thickness),
      1,
      20,
      connector.arrow.thickness,
    ),
    arrowSize: clampNumber(
      asFiniteNumber(patch.arrowSize),
      6,
      64,
      connector.arrow.arrowSize,
    ),
  };
  const data = toRecord(edge.data);
  const markerEnd =
    connector.style === "arrow" ? markerForArrow(nextArrow) : edge.markerEnd;
  const nextStyle =
    connector.style === "arrow" ? styleForArrow(nextArrow) : edge.style;
  return {
    ...edge,
    style: nextStyle,
    markerEnd,
    data: {
      ...data,
      connector: {
        ...connector,
        arrow: nextArrow,
      },
    },
  };
}

export function applyPipeConnectorPatchToEdge(
  edge: Edge,
  patch: Partial<PipeConnectorConfig>,
): Edge {
  const connector = resolveConnectorData(edge);
  const nextPipe: PipeConnectorConfig = {
    thickness: clampNumber(
      asFiniteNumber(patch.thickness),
      2,
      64,
      connector.pipe.thickness,
    ),
    flangeScale: Math.round(
      clampNumber(
        asFiniteNumber(patch.flangeScale),
        1,
        8,
        connector.pipe.flangeScale,
      ),
    ),
  };
  const data = toRecord(edge.data);
  return {
    ...edge,
    data: {
      ...data,
      connector: {
        ...connector,
        pipe: nextPipe,
      },
    },
  };
}
