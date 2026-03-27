# SCADA Widget Plugin Architecture (Project-Compatible Guide)

This guide adapts `frontend/chatgpt-implementation.md` to the **current codebase** in this repository and gives a migration path from the current coupled graph widgets to an IoC plugin model.

## 1. Current Baseline In This Repo

Today the frontend already has strong foundations we should reuse:

- Canvas/runtime host: `src/routes/+page.svelte` with `@xyflow/svelte`
- Widget rendering entry: `src/lib/features/graph/components/PlantAssetNode.svelte`
- Built-in widget registry: `src/lib/features/graph/assets/registry.ts`
- Widget types/data shape: `src/lib/features/graph/assets/types.ts`
- Live value utilities: `src/lib/features/graph/live-utils.ts`
- Shared WS transport + subscriptions: `src/lib/core/ws/tag-stream-client.ts`
- Page realtime provider: `src/lib/features/realtime/page-tag-realtime-provider.ts`
- Existing context menu pattern: `src/lib/features/tree/context-menu.ts` + `components/ContextMenu.svelte`

Current coupling pain points:

- Node data is asset-specific (`PlantAssetNodeData`) and mostly single-source (`sourceNode`, `liveValue`, `onWriteValue`).
- Built-in assets are Svelte-only and tightly mapped to `PlantAssetKind` enum.
- Event actions (click/double/right/context) are not yet declarative per widget instance.
- No stable plugin boundary/API for 3rd-party widget libraries.

## 2. Target Direction

Build a host-controlled, IoC architecture where:

- The host owns transport, permissions, navigation, action execution, menus, and lifecycle.
- Widgets (internal now, third-party later) are UI modules with no direct backend/store/router access.
- A widget instance can bind **multiple variables** by named binding keys.
- Bindings can be read/write/readwrite and remain reactive.
- Widget events map to declarative actions (navigate, write, popup, custom menu actions).
- A plugin can contribute a **collection of widget nodes** (UI plugin pack).
- Same plugin system can later support non-UI contributions (ingress/egress/3rd party integrations/processing modules/pid controllers/ai/etc).

## 3. Core IoC Contracts (Compatible With Current Runtime)

### 3.1 Plugin Container (future-proof for non-UI plugins)

```ts
export interface ScadaPlugin {
  id: string;
  version: string;
  apiVersion: string;
  displayName: string;

  contributes?: {
    widgets?: WidgetDefinition[];
    ingress?: IngressDefinition[];
    egress?: EgressDefinition[];
    actions?: ActionExecutorDefinition[];
  };
}
```

Use this even if we initially ship only internal widgets. This avoids redesign when ingress/egress plugins arrive.

### 3.2 Widget Definition

```ts
export type BindingAccess = "read" | "write" | "readwrite";

export interface WidgetBindingSchema {
  key: string;                  // level, temperature, pressure, command, etc.
  label: string;
  access: BindingAccess;
  required?: boolean;
  multiple?: boolean;           // allows arrays of tag bindings
  acceptedTypes?: Array<"Text" | "Float" | "Integer" | "Boolean">;
}

export interface WidgetEventSchema {
  name: string;                 // click, doubleClick, rightClick, menu:openTrend
  label: string;
  payload?: Record<string, string>;
}

export interface WidgetPropSchema {
  key: string;
  type: "string" | "number" | "boolean" | "select" | "color" | "json";
  label: string;
  default?: unknown;
  required?: boolean;
  options?: Array<{ label: string; value: string }>;
}

export interface WidgetDefinition {
  type: string;                 // globally unique widget type id
  displayName: string;
  category: string;
  version: string;
  bindings: WidgetBindingSchema[];
  props: WidgetPropSchema[];
  events: WidgetEventSchema[];
  factory: WidgetFactory;
}
```

### 3.3 Host-Injected Runtime API

```ts
export interface WidgetHostApi {
  mode: "edit" | "play";
  nodeId: string;
  pageId: string;

  // binding metadata + reactive reads
  bindings: {
    get(key: string): WidgetBindingRef | WidgetBindingRef[] | undefined;
    read(key: string): TagScalarValue | TagScalarValue[] | undefined;
    subscribe(key: string, cb: (value: TagScalarValue | TagScalarValue[] | undefined) => void): () => void;
    write(key: string, value: TagScalarValue): Promise<void>;
  };

  // host-controlled actions
  events: {
    emit(name: string, payload?: unknown): void;
    openContextMenu(items: WidgetMenuItem[], anchor: { x: number; y: number }): void;
  };

  nav: {
    goto(pageId: string, params?: Record<string, string>): Promise<void>;
    openPopup(pageId: string, params?: Record<string, string>): Promise<void>;
    closePopup(id?: string): void;
  };

  theme: {
    mode: "light" | "dark";
    vars: Record<string, string>;
  };

  permissions: {
    canRead(tagId: string): boolean;
    canWrite(tagId: string): boolean;
    canNavigate(pageId: string): boolean;
  };

  logger: {
    debug(...args: unknown[]): void;
    warn(...args: unknown[]): void;
    error(...args: unknown[]): void;
  };
}
```

Rules:

- Widgets must never touch `tagStreamClient`, router, or global stores directly.
- All writes/navigation/menu actions flow through host API.

## 4. Runtime Model For Svelte + Vanilla Widgets

Support both widget implementations behind one runtime contract:

```ts
export interface WidgetInstance {
  mount(container: HTMLElement): void;
  update(input: WidgetUpdateInput): void;
  dispose(): void;
}

export type WidgetFactory = (api: WidgetHostApi, initial: WidgetUpdateInput) => WidgetInstance;
```

Adapters:

- `SvelteWidgetAdapter`: wraps internal Svelte widget components.
- `CustomElementWidgetAdapter`: wraps vanilla custom elements (3rd party friendly).

This keeps host orchestration framework-agnostic while preserving Svelte performance for built-ins.

## 5. Multi-Binding Node Data Model (Migration Target)

Current `PlantAssetNodeData` should evolve to a generic shape:

```ts
export interface WidgetBindingRef {
  tagId: string;
  access: BindingAccess;
  label?: string;
}

export interface WidgetNodeData {
  widgetType: string;
  title: string;

  // many bindings per widget instance
  bindings: Record<string, WidgetBindingRef | WidgetBindingRef[]>;

  // resolved live values keyed by binding key (host-managed)
  values?: Record<string, TagScalarValue | TagScalarValue[] | undefined>;

  props: Record<string, unknown>;

  // declarative event->action config
  eventBindings?: WidgetEventBinding[];
}
```

Compatibility bridge:

- Keep existing `PlantAssetKind` + asset mapping as internal built-ins.
- Add a mapper that converts legacy `sourceNode/liveValue/onWriteValue` into `bindings/values` during migration.
- Do not break existing saved pages immediately.

## 6. Event -> Action Engine (SCADA-style behavior)

Introduce declarative event bindings per node instance:

```ts
export interface WidgetEventBinding {
  on: string;              // click, doubleClick, rightClick, menu:ack, etc.
  when?: string;           // safe expression
  do: WidgetAction[];
}

export type WidgetAction =
  | { type: "navigate"; pageId: string; params?: Record<string, string> }
  | { type: "openPopup"; pageId: string; params?: Record<string, string> }
  | { type: "closePopup"; id?: string }
  | { type: "writeBinding"; bindingKey: string; value: unknown }
  | { type: "toggleBinding"; bindingKey: string }
  | { type: "notify"; level: "info" | "warning" | "error"; message: string }
  | { type: "openContextMenu"; menuId: string };
```

For your examples:

- Tank widget binds `level`, `temperature`, `pressure`, emits `click` -> action `navigate` to detail page.
- Button widget binds optional `command`, emits `click` -> action `navigate` (or `writeBinding`).

## 7. Context Menu Model (Dynamic + Safe)

Use host-rendered menus only (reuse current ContextMenu pattern):

- Widget may propose menu items based on current values/props.
- Host merges with page-configured menu actions.
- Host filters by permission and executes actions.

Suggested item shape:

```ts
export interface WidgetMenuItem {
  id: string;
  label: string;
  icon?: string;
  enabled?: boolean;
  children?: WidgetMenuItem[];
  do?: WidgetAction[];
}
```

## 8. Reactivity and Performance Strategy (Important)

Given current code (`applyLiveValuesToGraphNodes`) patches nodes in arrays, migrate to indexed updates:

- Build `tagId -> widgetNodeIds[]` index from current page graph.
- On WS value updates, only update affected widget nodes/bindings.
- Coalesce high-frequency updates with `requestAnimationFrame` batching.
- Keep per-widget subscriptions fine-grained via binding keys.
- Avoid full graph-node remap for every tag update.

This preserves UI dynamism without sacrificing throughput.

## 9. Plugin Packaging Strategy (Now + Future)

### Phase A (now)

- Internal plugins only, bundled with app.
- One internal plugin can expose a collection of built-in widgets.

### Phase B (later)

- External plugin packages (ESM) loaded via controlled registry.
- API version check + manifest validation.
- Optional signature/trust policy.

UI plugin as collection of widgets example:

```ts
const coreUiPlugin: ScadaPlugin = {
  id: "lirays.core-ui",
  version: "1.0.0",
  apiVersion: "1",
  displayName: "Core UI Widgets",
  contributes: {
    widgets: [tankWidget, pumpWidget, valveWidget, sliderWidget, typedInputWidget]
  }
};
```

Later non-UI plugin example (same container):

```ts
const mqttIngressPlugin: ScadaPlugin = {
  id: "vendor.mqtt-ingress",
  version: "1.0.0",
  apiVersion: "1",
  displayName: "MQTT Ingress",
  contributes: {
    ingress: [mqttConnector]
  }
};
```

## 10. Concrete Migration Plan For This Repo

### Step 1: Introduce plugin API types (no behavior changes)

Add `src/lib/scada/plugin-api/types.ts` with shared contracts above.

### Step 2: Add plugin registry service

Add `src/lib/scada/registry/plugin-registry.ts`:

- register plugin
- expose `listWidgets()`
- resolve widget by `type`
- validate duplicate ids/types and apiVersion

### Step 3: Wrap current built-ins as internal plugin

Create `src/lib/scada/plugins/core-ui.plugin.ts` that reuses current components from `features/graph/assets/components/*`.

### Step 4: Add generic widget node shell

Create `src/lib/features/graph/components/WidgetNode.svelte`:

- resolve widget definition from registry
- mount/update/dispose instance through adapter
- isolate errors with broken-widget fallback

Keep `PlantAssetNode.svelte` as temporary compatibility wrapper.

### Step 5: Add new node data shape with compatibility mapper

Add mapper utilities to read both legacy and new data model.

### Step 6: Event action engine

Add `src/lib/scada/actions/action-engine.ts` and executors for:

- navigate
- writeBinding
- openPopup/closePopup
- notify
- context menu

### Step 7: Binding resolver on top of existing realtime provider

Reuse current `createPageTagRealtimeProvider` and derive values by binding key per node.

### Step 8: Add custom element adapter

Support vanilla custom element widgets with a host-provided API object.

## 11. Minimal Folder Additions (Non-breaking)

```text
src/lib/scada/
  plugin-api/
    types.ts
    validation.ts
  registry/
    plugin-registry.ts
  runtime/
    widget-runtime.ts
    adapters/
      svelte-widget-adapter.ts
      custom-element-widget-adapter.ts
  actions/
    action-engine.ts
    action-types.ts
    executors/
  bindings/
    binding-resolver.ts
  plugins/
    core-ui.plugin.ts
```

## 12. Guardrails (Bullet-proofing)

- Version every public plugin API (`apiVersion`).
- Reject incompatible manifests at registration time.
- Catch and isolate widget `mount/update/dispose` failures.
- Enforce permissions in action engine, not in widget code.
- Keep all page config serializable JSON.
- Never allow plugin direct socket or router access.
- Add diagnostics with plugin id/widget type/node id context.

## 13. Acceptance Criteria For This Migration

- Existing built-in widgets still render and write/read values.
- At least one widget can bind multiple variables (`level/temp/pressure`) and remain reactive.
- Event bindings can navigate and write via host action engine.
- Context menu actions are host-rendered and permission-checked.
- A UI plugin can register and expose a collection of widgets.
- Architecture supports future non-UI plugins without redesign.

## 14. Practical Recommendation For Next Increment

Implement this in two PR-sized slices:

1. Plugin API + registry + core-ui plugin wrapping current widgets (no UX change).
2. Generic widget node + multi-binding node data + action engine for click/right-click/navigation.

That sequence gives low risk, keeps velocity, and removes coupling progressively instead of a big-bang rewrite.
