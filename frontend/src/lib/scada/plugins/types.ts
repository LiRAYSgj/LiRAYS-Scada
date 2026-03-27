import type { Component } from "svelte";
import type {
  PlantAssetComponentProps,
  WidgetBindingSchema,
} from "$lib/features/graph/assets/types";

export interface WidgetManifest {
  type: string;
  displayName: string;
  version: string;
  apiVersion: string;
  primaryBindingKey: string;
  bindings: WidgetBindingSchema[];
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
