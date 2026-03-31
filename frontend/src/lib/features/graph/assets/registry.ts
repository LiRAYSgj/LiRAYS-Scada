import type { GraphAssetDefinition } from "./types";
import type {
  InternalWidgetDeclaration,
  ScadaInternalPlugin,
} from "$lib/scada/plugins/types";
import { internalWidgetsPlugin } from "$lib/scada/plugins/internal-widgets-plugin";

const definitionByKind = new Map<string, GraphAssetDefinition>();
const orderedDefinitions: GraphAssetDefinition[] = [];
const registeredPluginIds = new Set<string>();

function upsertAssetDefinition(definition: GraphAssetDefinition): void {
  const existingIndex = orderedDefinitions.findIndex(
    (item) => item.name === definition.name,
  );
  if (existingIndex >= 0) {
    orderedDefinitions[existingIndex] = definition;
  } else {
    orderedDefinitions.push(definition);
  }
  definitionByKind.set(definition.name, definition);
}

function toGraphAssetDefinition(
  plugin: ScadaInternalPlugin,
  declaration: InternalWidgetDeclaration,
): GraphAssetDefinition {
  return {
    name: declaration.manifest.type,
    pluginId: plugin.id,
    label: declaration.manifest.displayName,
    runtime: declaration.runtime,
    bindings: declaration.manifest.bindings,
    primaryBindingKey: declaration.manifest.primaryBindingKey,
  };
}

export function registerInternalPlugin(plugin: ScadaInternalPlugin): void {
  if (registeredPluginIds.has(plugin.id)) return;
  for (const widget of plugin.contributes.ui?.widgets ?? []) {
    if (widget.runtime.kind === "custom-element") {
      widget.runtime.register?.();
    }
    upsertAssetDefinition(toGraphAssetDefinition(plugin, widget));
  }
  registeredPluginIds.add(plugin.id);
}

export function registerAsset(definition: GraphAssetDefinition): void {
  upsertAssetDefinition(definition);
}

export function getRegisteredAssetDefinitions(): GraphAssetDefinition[] {
  return [...orderedDefinitions];
}

export const PLANT_ASSET_DEFINITIONS: GraphAssetDefinition[] =
  orderedDefinitions;

export function resolveAssetDefinition(kind: string): GraphAssetDefinition {
  return definitionByKind.get(kind) ?? orderedDefinitions[0];
}

registerInternalPlugin(internalWidgetsPlugin);
