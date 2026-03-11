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

export interface PlantAssetNodeData extends Record<string, unknown> {
  symbolId?: string;
  assetKind: PlantAssetKind;
  title: string;
  sourceNode: Pick<TreeNode, "id" | "name" | "path" | "kind" | "dataType">;
  liveValue?: TagScalarValue;
  onWriteValue?: (value: TagScalarValue) => void;
}

export interface PlantAssetComponentProps {
  data: PlantAssetNodeData;
  selected?: boolean;
}

export interface GraphAssetDefinition {
  name: PlantAssetKind;
  label: string;
  component: Component<PlantAssetComponentProps>;
}
