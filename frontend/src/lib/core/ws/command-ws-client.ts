import type { TagScalarValue } from "./types";
import type { Command, ItemMeta } from "../../proto/namespace/commands";
import type {
  NamespaceFolder,
  NamespaceNode,
  NamespaceSchema,
  Value,
} from "../../proto/namespace/types";
import type { ItemType, VarDataType } from "../../proto/namespace/enums";

export function createCommandId(prefix: string): string {
  return `${prefix}-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

export function toBackendValue(value: TagScalarValue): Value {
  if (typeof value === "string") {
    return { textValue: value };
  }
  if (typeof value === "boolean") {
    return { booleanValue: value };
  }
  if (Number.isInteger(value)) {
    return { integerValue: value as number };
  }
  return { floatValue: value as number };
}

export function fromBackendValue(
  value: Value | null | undefined,
): TagScalarValue | undefined {
  if (!value) {
    return undefined;
  }
  if (value.integerValue !== undefined) {
    return value.integerValue;
  }
  if (value.floatValue !== undefined) {
    return value.floatValue;
  }
  if (value.textValue !== undefined) {
    return value.textValue;
  }
  if (value.booleanValue !== undefined) {
    return value.booleanValue;
  }
  return undefined;
}

export function createGetCommand(
  varIds: string[],
  cmdId = createCommandId("get"),
): {
  cmdId: string;
  command: Command;
} {
  return {
    cmdId,
    command: {
      get: {
        cmdId,
        varIds,
      },
    },
  };
}

export function createSetCommand(
  id: string,
  value: TagScalarValue,
  cmdId = createCommandId("set"),
): { cmdId: string; command: Command } {
  return {
    cmdId,
    command: {
      set: {
        cmdId,
        varIdsValues: [{ varId: id, value: toBackendValue(value) }],
      },
    },
  };
}

export function createListCommand(
  itemId?: string,
  cmdId = createCommandId("list"),
): { cmdId: string; command: Command } {
  return {
    cmdId,
    command: {
      list: {
        cmdId,
        folderId: itemId,
      },
    },
  };
}

export function createAddCommand(
  parentId: string,
  itemsMeta: ItemMeta[],
  cmdId = createCommandId("add"),
): { cmdId: string; command: Command } {
  return {
    cmdId,
    command: {
      add: {
        cmdId,
        parentId,
        itemsMeta,
      },
    },
  };
}

export function createDelCommand(
  itemIds: string[],
  cmdId = createCommandId("del"),
): { cmdId: string; command: Command } {
  return {
    cmdId,
    command: {
      del: {
        cmdId,
        itemIds,
      },
    },
  };
}

/** Builder export JSON: nested objects; leaves are type strings (e.g. "Float"). */
export function namespaceJsonToSchema(
  json: Record<string, unknown>,
): NamespaceSchema {
  function nodeFromJson(val: unknown): NamespaceNode {
    if (typeof val === "string") {
      return { variableType: val };
    }
    if (val !== null && typeof val === "object" && !Array.isArray(val)) {
      const children: { [key: string]: NamespaceNode } = {};
      for (const [key, child] of Object.entries(
        val as Record<string, unknown>,
      )) {
        children[key] = nodeFromJson(child);
      }
      return { folder: { children } as NamespaceFolder };
    }
    throw new Error(
      `Invalid namespace node (expected string or object): ${typeof val}`,
    );
  }
  const roots: { [key: string]: NamespaceNode } = {};
  for (const [key, val] of Object.entries(json)) {
    roots[key] = nodeFromJson(val);
  }
  return { roots };
}

export function createAddBulkCommand(
  parentId: string,
  schema: NamespaceSchema,
  cmdId = createCommandId("add-bulk"),
): { cmdId: string; command: Command } {
  return {
    cmdId,
    command: {
      addBulk: {
        cmdId,
        parentId,
        schema,
      },
    },
  };
}

export function createSingleItemMeta(
  name: string,
  itemType: ItemType,
  varType: VarDataType | undefined,
): ItemMeta {
  return {
    name,
    iType: itemType,
    varDType: varType,
  };
}
