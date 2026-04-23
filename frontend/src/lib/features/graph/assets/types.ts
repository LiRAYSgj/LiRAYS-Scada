import type { Component } from "svelte";
import type { TreeNode } from "$lib/features/tree/types";
import type { TagScalarValue } from "$lib/core/ws/types";
import type { WidgetConfigFieldSchema } from "$lib/scada/plugins/types";

export enum PlantAssetKind {
  PUSH_BUTTON = "push_button",
  TANK = "tank",
  THERMOMETER = "thermometer",
  GAUGE = "gauge",
  SEVEN_SEGMENT = "seven_segment",
  ALARM = "alarm",
  PUMP = "pump",
  VALVE = "valve",
  FAN = "fan",
  SLIDER = "slider",
  TYPED_INPUT = "typed_input",
  ONOFF = "onoff",
  LIGHT = "light",
  LABEL = "label",
  TEXT = "text",
  RECTANGLE = "rectangle",
  MARKDOWN = "markdown",
  LEVEL_INDICATOR = "level_indicator",
}

export type WidgetBindingAccess = "read" | "write" | "readwrite";
export type WidgetInteractionEventName = "click" | "doubleClick" | "rightClick";
export type WidgetVarDataType =
  | "VAR_DATA_TYPE_INTEGER"
  | "VAR_DATA_TYPE_FLOAT"
  | "VAR_DATA_TYPE_TEXT"
  | "VAR_DATA_TYPE_BOOLEAN";

export type NodePortSide = "top" | "right" | "bottom" | "left";

export interface NodePortOffsets {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

export const DEFAULT_NODE_PORT_OFFSETS: NodePortOffsets = {
  top: 50,
  right: 50,
  bottom: 50,
  left: 50,
};

export function normalizeNodePortOffsets(value: unknown): NodePortOffsets {
  const source =
    value && typeof value === "object"
      ? (value as Partial<NodePortOffsets>)
      : {};
  const normalize = (candidate: unknown, fallback: number): number => {
    if (typeof candidate !== "number" || !Number.isFinite(candidate)) {
      return fallback;
    }
    return Math.max(0, Math.min(100, Math.round(candidate)));
  };

  return {
    top: normalize(source.top, DEFAULT_NODE_PORT_OFFSETS.top),
    right: normalize(source.right, DEFAULT_NODE_PORT_OFFSETS.right),
    bottom: normalize(source.bottom, DEFAULT_NODE_PORT_OFFSETS.bottom),
    left: normalize(source.left, DEFAULT_NODE_PORT_OFFSETS.left),
  };
}

export interface NavigateRuntimeViewAction {
  type: "navigateRuntimeView";
  params: {
    viewId: string;
    viewName?: string;
  };
}

export interface ContextMenuActionItem {
  id: string;
  label: string;
  viewId?: string;
  viewName?: string;
  enabled?: boolean;
}

export interface OpenContextMenuAction {
  type: "openContextMenu";
  params: {
    items: ContextMenuActionItem[];
  };
}

export type WidgetEventAction =
  | NavigateRuntimeViewAction
  | OpenContextMenuAction;

export interface WidgetEventBinding {
  on: WidgetInteractionEventName;
  do: WidgetEventAction[];
}

export interface WidgetBindingSchema {
  key: string;
  label: string;
  access: WidgetBindingAccess;
  dataTypes?: WidgetVarDataType[];
  required?: boolean;
  multiple?: boolean;
}

export interface BoundWidgetTag extends Pick<
  TreeNode,
  | "id"
  | "name"
  | "path"
  | "kind"
  | "dataType"
  | "unit"
  | "min"
  | "max"
  | "options"
> {}

export interface PlantAssetNodeData extends Record<string, unknown> {
  symbolId?: string;
  assetKind: string;
  title: string;
  graphReadOnly?: boolean;
  primaryBindingKey?: string;
  bindings?: Record<string, BoundWidgetTag[]>;
  liveValues?: Record<string, TagScalarValue | TagScalarValue[] | undefined>;
  sourceNode: Pick<
    TreeNode,
    | "id"
    | "name"
    | "path"
    | "kind"
    | "dataType"
    | "unit"
    | "min"
    | "max"
    | "options"
  >;
  liveValue?: TagScalarValue;
  onWriteValue?: (value: TagScalarValue) => void;
  onWriteBindingValue?: (
    bindingKey: string,
    value: TagScalarValue,
    tagId?: string,
  ) => void;
  interactionMode?: "editor" | "runtime";
  eventBindings?: WidgetEventBinding[];
  widgetConfig?: Record<string, unknown>;
  portOffsets?: NodePortOffsets;
  connectDraftActive?: boolean;
  onWidgetEvent?: (
    eventName: WidgetInteractionEventName,
    payload?: unknown,
    event?: MouseEvent,
  ) => void;
  onOpenBindingConfig?: (event: MouseEvent) => void;
}

export interface PlantAssetComponentProps {
  id: string;
  data: PlantAssetNodeData;
  selected?: boolean;
}

export interface GraphAssetDefinition {
  name: string;
  pluginId: string;
  label: string;
  resizable?: boolean;
  keepAspectRatio?: boolean;
  minWidth?: number;
  minHeight?: number;
  supportedEvents?: WidgetInteractionEventName[];
  runtime:
    | {
        kind: "svelte";
        component: Component<PlantAssetComponentProps>;
      }
    | {
        kind: "custom-element";
        tagName: string;
        register?: () => void;
      };
  bindings: WidgetBindingSchema[];
  primaryBindingKey: string;
  configSchema?: WidgetConfigFieldSchema[];
  defaultConfig?: Record<string, unknown>;
}
