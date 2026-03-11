export type TagScalarValue = number | string | boolean;

export enum WebSocketConnectionStatus {
  DISCONNECTED = "disconnected",
  CONNECTING = "connecting",
  RECONNECTING = "reconnecting",
  CONNECTED = "connected",
}

export type BackendValueEnvelope =
  | { Integer: number }
  | { Float: number }
  | { Text: string }
  | { Boolean: boolean };

export interface GetCommandPayload {
  cmd_id: string;
  var_ids: string[];
}

export interface SetCommandPayload {
  cmd_id: string;
  var_ids_values: [string, BackendValueEnvelope][];
}

export type BackendItemType = "Folder" | "Variable";
export type BackendVarDataType = "Integer" | "Float" | "Text" | "Boolean";

export interface AddCommandPayload {
  cmd_id: string;
  parent_id: string;
  items_meta: [string, BackendItemType, BackendVarDataType | null][];
}

export interface ListCommandPayload {
  cmd_id: string;
  item_id?: string;
}

export interface DelCommandPayload {
  cmd_id: string;
  item_ids: string[];
}

export interface GetResponsePayload {
  cmd_id: string;
  var_values: Array<BackendValueEnvelope | null>;
}

export interface ListResponsePayload {
  cmd_id: string;
  children_folders: Record<string, string>;
  children_vars: Record<string, [string, string]>;
}

export interface AddResponsePayload {
  cmd_id: string;
  item_ids: string[];
}

export interface DelResponsePayload {
  cmd_id: string;
}

export type BackendCommandEnvelope =
  | { GET: GetCommandPayload }
  | { SET: SetCommandPayload }
  | { LIST: ListCommandPayload }
  | { ADD: AddCommandPayload }
  | { DEL: DelCommandPayload };

export type BackendResponseEnvelope =
  | { GET: GetResponsePayload }
  | { LIST: ListResponsePayload }
  | { ADD: AddResponsePayload }
  | { DEL: DelResponsePayload };
