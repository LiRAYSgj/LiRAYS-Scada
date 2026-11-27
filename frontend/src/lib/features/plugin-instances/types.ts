/** Mirrors `src/plugin_form.rs` and `src/http/resources/plugin_instances.rs` in the backend. */

export type FormFieldType =
  | "text"
  | "textarea"
  | "secret"
  | "number"
  | "boolean"
  | "select"
  | "tag"
  | "tag_list"
  | "json"
  | "group"
  | "list";

export interface FormFieldOption {
  value: string;
  label: string;
}

export interface FormFieldVisibility {
  /** Sibling key inside the same object. */
  field: string;
  one_of: unknown[];
}

export interface FormField {
  key: string;
  label: string;
  type: FormFieldType;
  description?: string;
  required?: boolean;
  default?: unknown;
  placeholder?: string;
  visible_when?: FormFieldVisibility;
  integer?: boolean;
  min?: number;
  max?: number;
  step?: number;
  options?: FormFieldOption[];
  fields?: FormField[];
  min_items?: number;
  /** `tag`/`tag_list`: a dropped tree folder is entered as `folder/**`. */
  folder_as_pattern?: boolean;
}

export interface PluginDescriptor {
  plugin_type: string;
  display_name: string;
  description?: string;
  /** `null` when the plugin declares no form; the configuration is edited as raw JSON. */
  fields: FormField[] | null;
}

export interface PluginCollection {
  id: string;
  label: string;
  base_path: string;
  plugins: PluginDescriptor[];
}

export type PluginInstanceState = "running" | "disabled" | "stopped" | "error";

export interface PluginInstance {
  id: string;
  name: string;
  plugin_type: string;
  enabled: boolean;
  config: Record<string, unknown>;
  status: {
    state: PluginInstanceState;
    last_error: string | null;
  };
}

export interface PluginInstanceInput {
  name: string;
  plugin_type: string;
  enabled: boolean;
  config: Record<string, unknown>;
}

/** Only changed top-level fields; `config` follows the backend merge rules. */
export type PluginInstanceUpdate = Partial<PluginInstanceInput>;

export type FormValues = Record<string, unknown>;
export type FormErrors = Record<string, string>;

/** Editable state of the instance being created or edited. */
export interface PluginInstanceDraft {
  pluginType: string;
  name: string;
  enabled: boolean;
  /** Form values when the plugin declares a form. */
  values: FormValues;
  /** JSON text when the plugin declares no form. */
  rawConfig: string;
}
