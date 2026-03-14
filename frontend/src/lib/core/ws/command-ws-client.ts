import type { TagScalarValue } from "./types";
import type { Command, ItemMeta } from "../../proto/namespace/commands";
import type { Value } from "../../proto/namespace/types";
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
): Command {
  return {
    set: {
      cmdId,
      varIdsValues: [{ varId: id, value: toBackendValue(value) }],
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
