import type { TreeNode } from "$lib/features/tree/types";
import type {
  GraphAssetDefinition,
  WidgetBindingSchema,
  WidgetVarDataType,
} from "./assets/types";

export function normalizeVarDataType(
  value: string | undefined,
): WidgetVarDataType | undefined {
  if (!value) return undefined;
  const normalized = value.trim().toUpperCase();
  if (!normalized) return undefined;

  const prefixed = normalized.startsWith("VAR_DATA_TYPE_")
    ? normalized
    : `VAR_DATA_TYPE_${normalized}`;

  if (
    prefixed === "VAR_DATA_TYPE_INTEGER" ||
    prefixed === "VAR_DATA_TYPE_FLOAT" ||
    prefixed === "VAR_DATA_TYPE_TEXT" ||
    prefixed === "VAR_DATA_TYPE_BOOLEAN"
  ) {
    return prefixed;
  }
  return undefined;
}

export function isNodeCompatibleWithBinding(
  node: TreeNode,
  binding: WidgetBindingSchema | undefined,
): boolean {
  if (node.kind !== "tag") {
    return true;
  }
  if (!binding?.dataTypes || binding.dataTypes.length === 0) {
    return true;
  }
  const nodeDataType = normalizeVarDataType(node.dataType);
  if (!nodeDataType) {
    return false;
  }
  return binding.dataTypes.includes(nodeDataType);
}

export function acceptsPrimaryBindingType(
  definition: GraphAssetDefinition,
  node: TreeNode,
): boolean {
  if (node.kind !== "tag") {
    return true;
  }
  const primaryBinding = definition.bindings.find(
    (binding) => binding.key === definition.primaryBindingKey,
  );
  if (!primaryBinding) {
    return false;
  }
  return isNodeCompatibleWithBinding(node, primaryBinding);
}

export function filterAssetDefinitionsByPrimaryType(
  definitions: GraphAssetDefinition[],
  node: TreeNode,
): GraphAssetDefinition[] {
  return definitions.filter((definition) =>
    acceptsPrimaryBindingType(definition, node),
  );
}
