import type { Component } from "svelte";
import type {
  PlantAssetComponentProps,
  WidgetBindingSchema,
  WidgetInteractionEventName,
} from "$lib/features/graph/assets/types";

export interface WidgetEventSchema {
  name: WidgetInteractionEventName;
  label: string;
}

export interface WidgetCapabilitySchema {
  readsTags?: boolean;
  writesTags?: boolean;
  contributesContextMenu?: boolean;
  showTitle?: boolean;
  titleOverlaysBody?: boolean;
  keepAspectRatio?: boolean;
  resizable?: boolean;
  inputWidget?: boolean;
  minWidth?: number;
  minHeight?: number;
}

export type WidgetConfigFieldType =
  | "text"
  | "textarea"
  | "number"
  | "boolean"
  | "color"
  | "select"
  | "thresholds";

export interface WidgetConfigOption {
  value: string;
  label: string;
}

export interface WidgetConfigThresholdRange {
  min?: number;
  max?: number;
  color: string;
  label?: string;
}

export interface WidgetConfigFieldSchema {
  key: string;
  label: string;
  type: WidgetConfigFieldType;
  options?: WidgetConfigOption[];
  description?: string;
  min?: number;
  max?: number;
  step?: number;
  placeholder?: string;
}

export interface WidgetManifest {
  type: string;
  displayName: string;
  version: string;
  apiVersion: string;
  primaryBindingKey: string;
  bindings: WidgetBindingSchema[];
  events?: WidgetEventSchema[];
  capabilities?: WidgetCapabilitySchema;
  configSchema?: WidgetConfigFieldSchema[];
  defaultConfig?: Record<string, unknown>;
}

export type InternalWidgetRuntime =
  | {
      kind: "svelte";
      component: Component<PlantAssetComponentProps>;
    }
  | {
      kind: "custom-element";
      tagName: string;
      register?: () => void;
    };

export interface InternalWidgetDeclaration {
  manifest: WidgetManifest;
  runtime: InternalWidgetRuntime;
}

export interface UiPluginContribution {
  widgets: InternalWidgetDeclaration[];
}

export interface ScadaInternalPlugin {
  id: string;
  version: string;
  displayName: string;
  contributes: {
    ui?: UiPluginContribution;
  };
}
