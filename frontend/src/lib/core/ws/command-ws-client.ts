import type {
  AddCommandPayload,
  BackendItemType,
  BackendVarDataType,
  BackendCommandEnvelope,
  BackendValueEnvelope,
  DelCommandPayload,
  ListCommandPayload,
  TagScalarValue,
} from "./types";

export function createCommandId(prefix: string): string {
  return `${prefix}-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

export function toBackendValue(value: TagScalarValue): BackendValueEnvelope {
  if (typeof value === "string") {
    return { Text: value };
  }
  if (typeof value === "boolean") {
    return { Boolean: value };
  }
  if (Number.isInteger(value)) {
    return { Integer: value };
  }
  return { Float: value };
}

export function fromBackendValue(
  value: BackendValueEnvelope | null,
): TagScalarValue | undefined {
  if (!value) {
    return undefined;
  }
  if ("Integer" in value) {
    return value.Integer;
  }
  if ("Float" in value) {
    return value.Float;
  }
  if ("Text" in value) {
    return value.Text;
  }
  if ("Boolean" in value) {
    return value.Boolean;
  }
  return undefined;
}

export function createGetCommand(
  varIds: string[],
  cmdId = createCommandId("get"),
): {
  cmdId: string;
  command: BackendCommandEnvelope;
} {
  return {
    cmdId,
    command: {
      GET: {
        cmd_id: cmdId,
        var_ids: varIds,
      },
    },
  };
}

export function createSetCommand(
  id: string,
  value: TagScalarValue,
  cmdId = createCommandId("set"),
): BackendCommandEnvelope {
  return {
    SET: {
      cmd_id: cmdId,
      var_ids_values: [[id, toBackendValue(value)]],
    },
  };
}

export function createListCommand(
  itemId?: string,
  cmdId = createCommandId("list"),
): { cmdId: string; command: { LIST: ListCommandPayload } } {
  return {
    cmdId,
    command: {
      LIST: {
        cmd_id: cmdId,
        ...(itemId ? { item_id: itemId } : {}),
      },
    },
  };
}

export function createAddCommand(
  parentId: string,
  itemsMeta: AddCommandPayload["items_meta"],
  cmdId = createCommandId("add"),
): { cmdId: string; command: { ADD: AddCommandPayload } } {
  return {
    cmdId,
    command: {
      ADD: {
        cmd_id: cmdId,
        parent_id: parentId,
        items_meta: itemsMeta,
      },
    },
  };
}

export function createDelCommand(
  itemIds: string[],
  cmdId = createCommandId("del"),
): { cmdId: string; command: { DEL: DelCommandPayload } } {
  return {
    cmdId,
    command: {
      DEL: {
        cmd_id: cmdId,
        item_ids: itemIds,
      },
    },
  };
}

export function createSingleItemMeta(
  name: string,
  itemType: BackendItemType,
  varType: BackendVarDataType | null,
): [string, BackendItemType, BackendVarDataType | null] {
  return [name, itemType, varType];
}
