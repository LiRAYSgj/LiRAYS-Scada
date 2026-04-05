import type { TagScalarValue } from "./types";
import type { Command } from "../../proto/namespace/commands";
import type {
  NamespaceFolder,
  NamespaceNode,
  NamespaceSchema,
  Value,
  ItemMeta,
} from "../../proto/namespace/types";
import { EventType, ItemType, VarDataType } from "../../proto/namespace/enums";

export function createCommandId(prefix: string): string {
  return `${prefix}-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function stringToVarType(t: string): VarDataType {
  const v = t.toLowerCase();
  if (v === "float") return VarDataType.VAR_DATA_TYPE_FLOAT;
  if (v === "integer" || v === "int") return VarDataType.VAR_DATA_TYPE_INTEGER;
  if (v === "text" || v === "string") return VarDataType.VAR_DATA_TYPE_TEXT;
  if (v === "boolean" || v === "bool") return VarDataType.VAR_DATA_TYPE_BOOLEAN;
  return VarDataType.VAR_DATA_TYPE_INVALID;
}

function buildNamespaceVariable(t: string) {
  return {
    varDType: stringToVarType(t),
    unit: undefined,
    min: undefined,
    max: undefined,
    options: [],
    maxLen: undefined,
  };
}

type NamespaceVariableSpec = {
  type: string;
  unit?: unknown;
  min?: unknown;
  max?: unknown;
  maxLength?: unknown;
  options?: unknown;
};

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function isNamespaceVariableSpec(
  value: unknown,
): value is NamespaceVariableSpec {
  if (!isPlainObject(value)) return false;
  return typeof value.type === "string";
}

function toOptionalNumber(value: unknown, field: string): number | undefined {
  if (value === undefined) return undefined;
  if (typeof value === "number" && Number.isFinite(value)) return value;
  throw new Error(`Invalid namespace variable "${field}" (expected number).`);
}

function toOptionalString(value: unknown, field: string): string | undefined {
  if (value === undefined) return undefined;
  if (typeof value === "string") return value;
  throw new Error(`Invalid namespace variable "${field}" (expected string).`);
}

function toStringArray(value: unknown, field: string): string[] {
  if (value === undefined) return [];
  if (
    !Array.isArray(value) ||
    !value.every((item) => typeof item === "string")
  ) {
    throw new Error(
      `Invalid namespace variable "${field}" (expected string[]).`,
    );
  }
  return value;
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

export function createSubscribeCommand(
  varIds: string[],
  events: EventType[],
  cmdId = createCommandId("sub"),
): {
  cmdId: string;
  command: Command;
} {
  return {
    cmdId,
    command: {
      sub: {
        cmdId,
        varIds,
        events,
      },
    },
  };
}

export function createUnsubscribeCommand(
  varIds: string[],
  events: EventType[],
  cmdId = createCommandId("unsub"),
): {
  cmdId: string;
  command: Command;
} {
  return {
    cmdId,
    command: {
      unsub: {
        cmdId,
        varIds,
        events,
      },
    },
  };
}

/** App-lifetime subscription to tree change pushes (never unsubscribed). */
export function createTreeChangeSubscribeCommand(
  cmdId = createCommandId("sub-tree"),
): { cmdId: string; command: Command } {
  return createSubscribeCommand([], [EventType.EVENT_TYPE_TREE_CHANGE], cmdId);
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
  const roots: { [key: string]: NamespaceNode } = {};
  for (const [key, val] of Object.entries(json)) {
    roots[key] = nodeFromJson(val);
  }
  return { roots };
}

function nodeFromJson(val: unknown): NamespaceNode {
  if (typeof val === "string") {
    return { variable: buildNamespaceVariable(val) };
  }
  if (isNamespaceVariableSpec(val)) {
    return {
      variable: {
        varDType: stringToVarType(val.type),
        unit: toOptionalString(val.unit, "unit"),
        min: toOptionalNumber(val.min, "min"),
        max: toOptionalNumber(val.max, "max"),
        options: toStringArray(val.options, "options"),
        maxLen: toOptionalNumber(val.maxLength, "maxLength"),
      },
    };
  }
  if (isPlainObject(val)) {
    const children: { [key: string]: NamespaceNode } = {};
    for (const [key, child] of Object.entries(val)) {
      children[key] = nodeFromJson(child);
    }
    return { folder: { children } as NamespaceFolder };
  }
  throw new Error(
    `Invalid namespace node (expected string or object): ${typeof val}`,
  );
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
  meta?: {
    unit?: string;
    min?: number;
    max?: number;
    options?: string[];
    maxLen?: number;
  },
): ItemMeta {
  return {
    name,
    iType: itemType,
    varDType: varType,
    unit: meta?.unit,
    min: meta?.min,
    max: meta?.max,
    options: meta?.options ?? [],
    maxLen: meta?.maxLen,
  };
}

export function createEditMetaCommand(
  varId: string,
  meta: {
    unit?: string;
    min?: number;
    max?: number;
    options?: string[];
    maxLen?: number;
  },
  cmdId = createCommandId("edit-meta"),
): { cmdId: string; command: Command } {
  return {
    cmdId,
    command: {
      editMeta: {
        cmdId,
        varId,
        unit: meta.unit,
        min: meta.min,
        max: meta.max,
        options: meta.options ?? [],
        maxLen: meta.maxLen,
      },
    },
  };
}
