import type { Component } from "svelte";
import type { TreeNode } from "$lib/features/tree/types";
import type { TagScalarValue } from "$lib/core/ws/types";

export enum PlantAssetKind {
  TANK = "tank",
  PUMP = "pump",
  VALVE = "valve",
  FAN = "fan",
  SLIDER = "slider",
  TYPED_INPUT = "typed_input",
  ONOFF = "onoff",
  LIGHT = "light",
  LABEL = "label",
}

export type WidgetBindingAccess = "read" | "write" | "readwrite";

export interface WidgetBindingSchema {
  key: string;
  label: string;
  access: WidgetBindingAccess;
  required?: boolean;
  multiple?: boolean;
}

export interface BoundWidgetTag extends Pick<
  TreeNode,
  "id" | "name" | "path" | "kind" | "dataType"
> {}

export interface PlantAssetNodeData extends Record<string, unknown> {
  symbolId?: string;
  assetKind: string;
  title: string;
  primaryBindingKey?: string;
  bindings?: Record<string, BoundWidgetTag[]>;
  liveValues?: Record<string, TagScalarValue | TagScalarValue[] | undefined>;
  sourceNode: Pick<TreeNode, "id" | "name" | "path" | "kind" | "dataType">;
  liveValue?: TagScalarValue;
  onWriteValue?: (value: TagScalarValue) => void;
  onWriteBindingValue?: (
    bindingKey: string,
    value: TagScalarValue,
    tagId?: string,
  ) => void;
  onOpenBindingConfig?: (event: MouseEvent) => void;
}

export interface PlantAssetComponentProps {
  data: PlantAssetNodeData;
  selected?: boolean;
}

export interface GraphAssetDefinition {
  name: string;
  pluginId: string;
  label: string;
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
}
