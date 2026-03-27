IMPLEMENTATION GUIDE FOR CODEX
Project: SCADA HMI Runtime + Plugin Architecture on top of Svelte + Svelte Flow
Goal: Build a production-grade SCADA editor/runtime where third-party developers can create widget libraries mostly with vanilla JavaScript, HTML, and CSS, while the host application remains in control of lifecycle, subscriptions, permissions, navigation, context menus, and event/action execution.

================================================================================
1. PRODUCT INTENT
================================================================================

Build a SCADA web application with two major modes:

1) EDITOR MODE
- Users compose HMI pages visually using Svelte Flow.
- Each widget is represented as a node on the canvas.
- Users can move, resize, configure, bind variables, and define event actions.
- Users can assign several variables to one widget instance using named bindings.
- Users can define click, double-click, right-click, and context-menu behaviors.

2) RUNTIME MODE
- The same page is rendered as a live HMI.
- Widgets subscribe to process variables through a central broker.
- Widgets can read values, write values if allowed, navigate to pages, open popups, and show context menus.
- Runtime must stay performant under high-frequency variable updates.
- One broken widget must not break the entire page.

Core product principle:
A widget never owns transport, routing, or privilege. It only renders UI and asks the host to do things.

================================================================================
2. TECHNICAL FOUNDATIONS
================================================================================

Use:
- Svelte 5
- SvelteKit
- TypeScript
- @xyflow/svelte for page layout and node hosting
- WebSocket for real-time variable traffic
- Vanilla JS / HTML / CSS as the preferred plugin authoring format
- Custom Elements as the preferred plugin boundary for third-party widgets

Rationale:
- Svelte Flow supports custom nodes by mapping node types to Svelte components.
- Svelte components can be compiled to custom elements.
- Custom elements provide a framework-neutral boundary for plugins.
- The host app can remain Svelte-first while third-party plugins remain framework-agnostic.

Important implementation constraints:
- Do not let third-party widgets access the global WebSocket directly.
- Do not let third-party widgets import application stores directly.
- Do not push all process values into one giant reactive object that causes broad rerenders.
- Do not couple plugin APIs to Svelte internals.
- Keep the plugin API stable and versioned.

================================================================================
3. ARCHITECTURAL OVERVIEW
================================================================================

Implement the application in 5 layers:

A. TRANSPORT LAYER
- Manages the live WebSocket connection.
- Receives process variable updates from the backend.
- Sends write commands to the backend.
- Handles reconnect, heartbeat, stale detection, and connection state.

B. TAG BROKER LAYER
- Central in-memory broker for variable/tag subscriptions.
- Widgets subscribe to specific tag IDs through the broker.
- Broker deduplicates subscriptions across widgets.
- Broker stores snapshots with value, quality, timestamp, and sequence metadata.
- Broker supports batch/coalesced delivery for performance.

C. ACTION + NAVIGATION LAYER
- Executes declarative actions triggered by widget events.
- Supports actions like navigate, open popup, close popup, write tag, toggle boolean, show confirmation, show notification, emit custom event, show context menu.
- All actions pass through permission checks and condition evaluation.

D. WIDGET RUNTIME LAYER
- Resolves plugin definitions from a widget registry.
- Creates widget instances.
- Injects the controlled host API into each widget.
- Mounts the widget into a DOM container.
- Owns disposal, updates, and error fallback.

E. EDITOR + PAGE COMPOSITION LAYER
- Uses Svelte Flow for page composition and node layout.
- Provides custom node shells for widget instances.
- Exposes a properties inspector, binding editor, event editor, and context menu editor.
- Stores pages as JSON documents.

================================================================================
4. INVERSION OF CONTROL STRATEGY
================================================================================

This project must use strong inversion of control.

The host controls:
- widget lifecycle
- variable subscriptions
- variable writes
- permissions
- navigation
- context menu rendering
- popup management
- action execution
- edit/runtime mode switching
- theme and layout context
- plugin validation and compatibility checks

The widget controls:
- its internal DOM
- how it visualizes values
- which semantic events it emits
- optional menu contributions
- optional helper UI logic

Never allow widgets to:
- open their own process socket
- directly navigate through the router
- directly mutate global stores
- directly execute privileged commands
- assume edit/runtime mode on their own
- directly fetch protected backend endpoints without host approval

================================================================================
5. CORE DATA MODELS
================================================================================

Define the following core models.

5.1 TAG SAMPLE

type Quality = 'good' | 'bad' | 'uncertain' | 'stale' | 'disconnected';

interface TagSample<T = unknown> {
  value: T;
  quality: Quality;
  ts: number;
  seq?: number;
  source?: string;
}

5.2 TAG BINDING

type BindingDirection = 'read' | 'write' | 'readwrite';

interface TagBindingRef {
  tagId: string;
  access: BindingDirection;
  label?: string;
  transformIn?: string;
  transformOut?: string;
}

5.3 WIDGET MANIFEST

interface BindingSchema {
  name: string;
  label: string;
  direction: BindingDirection;
  types: string[];
  required?: boolean;
  multiple?: boolean;
  description?: string;
}

interface PropSchema {
  name: string;
  type: 'string' | 'number' | 'boolean' | 'color' | 'select' | 'json';
  label: string;
  default?: unknown;
  options?: { label: string; value: string }[];
  required?: boolean;
  description?: string;
}

interface EventSchema {
  name: string;
  label: string;
  payloadShape?: Record<string, string>;
}

interface MenuContributionSchema {
  id: string;
  label: string;
  description?: string;
}

interface WidgetManifest {
  type: string;
  version: string;
  apiVersion: string;
  displayName: string;
  category: string;
  icon?: string;
  bindings: BindingSchema[];
  props: PropSchema[];
  events: EventSchema[];
  menuContributions?: MenuContributionSchema[];
  capabilities: {
    readsTags: boolean;
    writesTags: boolean;
    contributesContextMenu: boolean;
    resizable: boolean;
  };
}

5.4 PAGE NODE MODEL

interface HmiNode {
  id: string;
  type: string; // Svelte Flow node type, likely "widget-node"
  position: { x: number; y: number };
  width: number;
  height: number;
  selected?: boolean;
  data: {
    widgetType: string;
    widgetVersion?: string;
    props: Record<string, unknown>;
    bindings: Record<string, TagBindingRef | TagBindingRef[]>;
    eventBindings: EventBinding[];
    style?: Record<string, unknown>;
    locked?: boolean;
    hidden?: boolean;
  };
}

5.5 EVENT BINDING MODEL

type ActionType =
  | 'navigate'
  | 'openPopup'
  | 'closePopup'
  | 'writeTag'
  | 'toggleTag'
  | 'showContextMenu'
  | 'notify'
  | 'confirm'
  | 'emitAppEvent';

interface EventBinding {
  on: string;
  when?: string;
  do: ActionConfig[];
}

interface ActionConfig {
  type: ActionType;
  params: Record<string, unknown>;
}

================================================================================
6. HOST API INJECTED INTO WIDGETS
================================================================================

Define a stable widget host API.

interface WidgetContext {
  instanceId: string;
  widgetId: string;
  pageId: string;
  mode: 'editor' | 'runtime';

  props: Readonly<Record<string, unknown>>;
  layout: Readonly<{
    width: number;
    height: number;
    zoom: number;
    selected: boolean;
    disabled: boolean;
  }>;

  tags: {
    getBinding(name: string): TagBindingRef | TagBindingRef[] | undefined;
    read(name: string): TagSample | TagSample[] | undefined;
    subscribe(name: string, cb: (sample: TagSample | TagSample[]) => void): () => void;
    write(name: string, value: unknown, options?: Record<string, unknown>): Promise<unknown>;
    list(): { name: string; binding: TagBindingRef | TagBindingRef[] }[];
  };

  events: {
    emit(name: string, payload?: unknown): void;
    trigger(actionId: string, payload?: unknown): Promise<void>;
    openContextMenu(items: ContextMenuItem[], event?: MouseEvent | PointerEvent): void;
  };

  nav: {
    goto(pageId: string, params?: Record<string, string>): Promise<void>;
    openPopup(pageId: string, params?: Record<string, string>): Promise<void>;
    closePopup(popupId?: string): void;
  };

  ui: {
    theme(): Record<string, unknown>;
    setStatus(status: 'ok' | 'warning' | 'alarm' | 'stale' | 'error'): void;
    requestRender(): void;
  };

  lifecycle: {
    onDispose(cb: () => void): void;
  };

  logger: {
    debug(...args: unknown[]): void;
    warn(...args: unknown[]): void;
    error(...args: unknown[]): void;
  };
}

Important rules:
- Widgets receive this object from the host.
- Widgets must treat this object as their only bridge to application behavior.
- The host may freeze parts of this object to avoid mutation.
- Future host API additions must be backward compatible.

================================================================================
7. PLUGIN FORMAT
================================================================================

Support plugins as registered widget providers.

interface ScadaWidgetPlugin {
  manifest: WidgetManifest;
  create(ctx: WidgetContext): WidgetInstance;
}

interface WidgetInstance {
  mount(el: HTMLElement): void;
  update(input: {
    props: Record<string, unknown>;
    layout: {
      width: number;
      height: number;
      zoom: number;
      selected: boolean;
      disabled: boolean;
    };
    bindings: Record<string, TagBindingRef | TagBindingRef[]>;
    mode: 'editor' | 'runtime';
  }): void;
  dispose(): void;
}

Rules:
- create() returns a stateful instance.
- mount() attaches DOM to the given element.
- update() refreshes props/layout/bindings/mode.
- dispose() must clean up subscriptions, timers, observers, and listeners.

================================================================================
8. PREFERRED THIRD-PARTY AUTHORING MODEL
================================================================================

Preferred third-party format: vanilla custom element.

Example concept:
- Plugin package exports a registration function or default plugin object.
- The widget instance internally creates a custom element and passes context/props/bindings into it.
- The custom element manages its own DOM and CSS.

Alternative supported format:
- Svelte-authored widget compiled as a custom element.

Do not require third-party authors to write Svelte unless they want to.

Recommended packaging target:
- ESM
- No framework runtime dependencies required for vanilla widgets
- Optional CSS bundled with the plugin
- Optional shadow DOM isolation

================================================================================
9. CUSTOM ELEMENT BRIDGE DESIGN
================================================================================

Implement a bridge so the host can mount either:
- a vanilla widget object
- or a custom element-based widget

Recommended host-side adapter pattern:

class CustomElementWidgetAdapter implements WidgetInstance {
  private el?: HTMLElement;

  constructor(
    private readonly ctx: WidgetContext,
    private readonly tagName: string,
    private input: {
      props: Record<string, unknown>;
      layout: {
        width: number;
        height: number;
        zoom: number;
        selected: boolean;
        disabled: boolean;
      };
      bindings: Record<string, TagBindingRef | TagBindingRef[]>;
      mode: 'editor' | 'runtime';
    }
  ) {}

  mount(container: HTMLElement) {
    this.el = document.createElement(this.tagName);
    (this.el as any).widgetContext = this.ctx;
    (this.el as any).widgetProps = this.input.props;
    (this.el as any).widgetLayout = this.input.layout;
    (this.el as any).widgetBindings = this.input.bindings;
    (this.el as any).widgetMode = this.input.mode;
    container.appendChild(this.el);
  }

  update(next: typeof this.input) {
    this.input = next;
    if (!this.el) return;
    (this.el as any).widgetProps = next.props;
    (this.el as any).widgetLayout = next.layout;
    (this.el as any).widgetBindings = next.bindings;
    (this.el as any).widgetMode = next.mode;
  }

  dispose() {
    if (this.el?.parentNode) {
      this.el.parentNode.removeChild(this.el);
    }
    this.el = undefined;
  }
}

================================================================================
10. WIDGET REGISTRY
================================================================================

Implement a widget registry service.

Responsibilities:
- register plugins
- reject invalid or duplicate manifests
- validate apiVersion compatibility
- resolve plugin by widget type
- expose manifests to the editor palette
- optionally lazy-load plugins

Suggested interface:

class WidgetRegistry {
  register(plugin: ScadaWidgetPlugin): void;
  resolve(type: string): ScadaWidgetPlugin | undefined;
  list(): WidgetManifest[];
  validateManifest(manifest: WidgetManifest): void;
}

Validation requirements:
- type must be unique
- apiVersion must match host-supported range
- bindings names must be unique
- props names must be unique
- event names must be unique
- no invalid capability combinations

================================================================================
11. TAG BROKER DESIGN
================================================================================

Implement a subscription broker with fine-grained subscriptions.

Requirements:
- widgets subscribe by tag ID
- tag broker deduplicates subscriptions
- broker maintains latest snapshot
- broker supports synchronous getSnapshot()
- broker dispatches updates efficiently
- broker can batch UI notifications at animation frame boundaries if needed
- broker tracks connection status and stale quality transitions

Suggested interface:

class TagBroker {
  private snapshots = new Map<string, TagSample>();
  private listeners = new Map<string, Set<(sample: TagSample) => void>>();

  getSnapshot(tagId: string): TagSample | undefined;
  publish(tagId: string, sample: TagSample): void;
  subscribe(tagId: string, cb: (sample: TagSample) => void): () => void;
  batchPublish(entries: Array<{ tagId: string; sample: TagSample }>): void;
}

Performance requirements:
- no page-wide rerender for single-tag changes
- avoid deep-cloning large broker state
- deliver only to widgets that subscribed
- support throttling/coalescing for high-frequency tags like trends or rapidly changing analog values

================================================================================
12. WEBSOCKET CLIENT DESIGN
================================================================================

Implement a transport service separate from widgets.

Responsibilities:
- open one shared process data socket per session or project context
- authenticate if necessary
- subscribe/unsubscribe tag IDs with backend when reference counts change
- receive variable updates and publish into TagBroker
- send write requests
- handle reconnect/backoff
- mark data stale after disconnect
- expose online/offline connection state

Suggested interface:

class ProcessTransport {
  connect(): Promise<void>;
  disconnect(): Promise<void>;
  requestSubscriptions(tagIds: string[]): void;
  releaseSubscriptions(tagIds: string[]): void;
  writeTag(tagId: string, value: unknown, options?: Record<string, unknown>): Promise<unknown>;
}

Important:
- TagBroker and ProcessTransport collaborate.
- Widgets never touch ProcessTransport directly.

================================================================================
13. ACTION ENGINE
================================================================================

Implement a declarative action engine.

Goal:
- widget emits semantic events
- page event binding chooses which actions run
- action engine performs permission checks and execution

Supported initial actions:
- navigate
- openPopup
- closePopup
- writeTag
- toggleTag
- showContextMenu
- notify
- confirm
- emitAppEvent

Suggested execution flow:
1. widget emits semantic event
2. host captures event
3. event bindings for this widget instance are filtered by event name
4. optional "when" condition is evaluated
5. actions are executed in order
6. failures are logged and optionally surfaced in runtime diagnostics

Example event binding:
{
  "on": "click",
  "do": [
    {
      "type": "navigate",
      "params": {
        "pageId": "tank-detail",
        "params": {
          "assetId": "$props.assetId"
        }
      }
    }
  ]
}

================================================================================
14. CONDITION / EXPRESSION SYSTEM
================================================================================

Implement a safe, constrained expression evaluator.

Use cases:
- visibility conditions
- enable/disable rules
- dynamic text formatting
- event conditions
- tag write transforms
- navigation params from widget props or bindings

Inputs allowed to expressions:
- props
- tags
- layout
- event payload
- user/session metadata if needed

Do not allow arbitrary JS eval().
Use either:
- a custom small expression parser
- or a very constrained safe expression library

Expression examples:
- tags.level.value > 80
- props.enabled === true
- event.itemId === "open-trend"
- tags.mode.value === "manual" && tags.interlock.value === false

Rules:
- expressions must be deterministic
- expressions must not mutate state
- expressions must not access global objects
- failures return false or undefined safely and log diagnostics

================================================================================
15. CONTEXT MENU SYSTEM
================================================================================

Implement dynamic context menus as a host-owned feature.

Pattern:
- Widget may request a context menu.
- Widget may contribute menu items.
- Host merges widget-contributed items with page-configured items.
- Host renders the menu.
- Host executes the selected action.

Menu item model:
interface ContextMenuItem {
  id: string;
  label: string;
  icon?: string;
  enabled?: boolean;
  visible?: boolean;
  children?: ContextMenuItem[];
  action?: ActionConfig[];
}

Typical SCADA examples:
- Open faceplate
- Open trend
- Go to detail page
- Write setpoint
- Ack alarm
- Force value
- Disable/Enable
- Show diagnostics

Security rule:
Widgets may suggest menu items, but the host decides which ones actually appear and what they do.

================================================================================
16. EDITOR/RUNTIME MODE SPLIT
================================================================================

Every widget must run correctly in two modes:

EDITOR MODE
- Clicking selects the node, not the widget action
- Resize handles are visible if allowed
- Dragging moves the node
- Variable values may be mocked or live-previewed depending on configuration
- Context menu may show editor actions
- Runtime write actions should be disabled or simulated

RUNTIME MODE
- Widget interaction triggers configured event bindings
- Writes are allowed only if permissions allow
- Context menu shows runtime actions
- Widget shows live process values

Implement a mode flag in WidgetContext and pass it consistently.

================================================================================
17. SVELTE FLOW INTEGRATION
================================================================================

Use Svelte Flow only as the layout/editor host, not as the widget logic owner.

Implement a custom node shell component in Svelte.

Responsibilities of the Svelte node shell:
- receives the Svelte Flow node props
- renders selection/resize/editor chrome
- creates a DOM mount point for the widget runtime
- resolves the widget plugin from the registry
- creates/mounts the widget instance
- updates the widget instance when props/layout/bindings change
- cleans up on destroy
- intercepts editor interactions vs runtime interactions
- shows fallback UI if widget creation fails

Important implementation note:
Treat widget body rendering as imperative mounting into a container.
Do not force all third-party widget internals into Svelte reactive templating.

================================================================================
18. ERROR ISOLATION AND DIAGNOSTICS
================================================================================

This system must be resilient.

Required protections:
- if widget plugin resolution fails, show a broken-widget placeholder
- if widget mount() throws, show fallback and keep page alive
- if widget update() throws, isolate failure to that widget
- if widget dispose() throws, log and continue
- runtime should expose a diagnostics panel in dev mode

Broken widget placeholder should show:
- widget type
- plugin version if known
- error message in dev mode
- actionable warning like "Plugin API mismatch" or "Missing widget library"

================================================================================
19. PERMISSIONS MODEL
================================================================================

Implement host-level permission checks.

Permissions may depend on:
- user role
- session claims
- project/site scope
- page mode
- widget instance config
- backend authorization policy

Permission examples:
- read tag X
- write tag X
- navigate to page Y
- open popup Z
- acknowledge alarm
- show certain context menu items
- run command action

Permission engine interface:
interface PermissionService {
  canReadTag(tagId: string): boolean;
  canWriteTag(tagId: string): boolean;
  canNavigate(pageId: string): boolean;
  canExecuteAction(action: ActionConfig): boolean;
}

Never trust widget intent alone.

================================================================================
20. SERIALIZATION FORMAT FOR PAGES
================================================================================

Persist HMI pages as JSON.

A page document should contain:
- page metadata
- background/grid settings
- nodes
- edges if needed for logical relationships
- page-level actions or variables
- optional popup definitions

Example shape:
interface HmiPageDocument {
  id: string;
  name: string;
  version: string;
  viewport?: { x: number; y: number; zoom: number };
  nodes: HmiNode[];
  edges?: unknown[];
  metadata?: Record<string, unknown>;
}

Rules:
- keep widget props serializable
- keep event bindings serializable
- keep bindings serializable
- avoid storing live runtime state in persisted documents

================================================================================
21. INITIAL FOLDER STRUCTURE
================================================================================

Generate the following initial folder structure:

src/
  lib/
    scada/
      actions/
        ActionEngine.ts
        action-types.ts
        executors/
          navigate.ts
          openPopup.ts
          closePopup.ts
          writeTag.ts
          toggleTag.ts
          notify.ts
          showContextMenu.ts
      expressions/
        ExpressionEngine.ts
      permissions/
        PermissionService.ts
      registry/
        WidgetRegistry.ts
      runtime/
        WidgetRuntime.ts
        WidgetContextFactory.ts
        adapters/
          CustomElementWidgetAdapter.ts
      tags/
        TagBroker.ts
        ProcessTransport.ts
        TagSubscriptionManager.ts
      pages/
        models.ts
        page-store.ts
      editor/
        inspectors/
          widget-props-inspector.ts
          widget-bindings-inspector.ts
          widget-events-inspector.ts
      widgets/
        host/
          WidgetNodeShell.svelte
          BrokenWidget.svelte
        builtins/
          button/
          tank/
          text-indicator/
      plugin-api/
        types.ts
        constants.ts
        validation.ts
  routes/
    editor/
    runtime/
  plugins/
    builtin/
      button.plugin.ts
      tank.plugin.ts
    examples/
      vanilla-button/
      vanilla-tank/

================================================================================
22. BUILT-IN WIDGETS TO IMPLEMENT FIRST
================================================================================

Implement these built-in widgets first to prove the architecture:

1. BUTTON WIDGET
- props: label, variant
- bindings: optional command/readwrite binding
- events: click, doubleClick, rightClick
- common action: navigate or write a tag

2. TEXT INDICATOR
- props: label, format
- bindings: value
- events: click, rightClick
- shows value, quality, timestamp if configured

3. TANK WIDGET
- props: title, min, max, showTemperature, showPressure
- bindings: level, temperature, pressure, setpoint(optional)
- events: click, doubleClick, rightClick
- may contribute context menu items like open-faceplate or open-trend

4. TOGGLE / SWITCH
- bindings: state (readwrite)
- events: click, rightClick
- writes boolean or enum values

================================================================================
23. VANILLA WIDGET AUTHOR EXPERIENCE
================================================================================

Third-party developers should be able to write a widget with:
- one JS file
- optional CSS
- a manifest
- a plugin registration entry

Example mental model:
- widget receives widgetContext, widgetProps, widgetBindings, widgetLayout
- widget subscribes to named bindings
- widget updates DOM when values change
- widget emits semantic events via widgetContext.events.emit()

Recommended conventions for custom elements:
- host API provided as element.widgetContext
- host updates properties rather than string attributes
- widget cleans up its own subscriptions on disconnectedCallback

================================================================================
24. EXAMPLE MINIMAL VANILLA WIDGET CONTRACT
================================================================================

Codex should implement an example vanilla widget library.

Example file: plugins/examples/vanilla-button/vanilla-button.js

Concept:
- define custom element "scada-vanilla-button"
- element renders a button
- if "command" binding exists, clicking writes a value or emits click
- right-click opens a context menu through host
- element subscribes to a bound tag if present
- element updates CSS class based on quality/state

Important:
Keep this example very small and heavily commented so it becomes the canonical template for plugin authors.

================================================================================
25. REACTIVITY STRATEGY
================================================================================

This is critical.

Use fine-grained subscription-based reactivity for process variables.

Rules:
- widget instances subscribe only to the specific bindings they need
- tag updates must not cause full page rerenders
- widget host shell rerenders only when node/layout/config changes
- widget internal DOM updates can be imperative for speed
- use requestAnimationFrame batching where useful
- for rapidly changing values, consider sampling/throttling by widget type

Examples:
- a text indicator can update every change
- a heavy chart or trend may require throttling
- hidden widgets can suspend non-critical subscriptions if desired
- editor mode can use lower-frequency refresh than runtime mode if needed

================================================================================
26. PAGE NAVIGATION + POPUP MODEL
================================================================================

Implement a runtime page manager.

Features:
- navigate to another page
- pass route/page params
- open popup pages as overlays
- close popup pages
- support widget-driven navigation through action engine only

Page manager interface:
interface PageRuntimeService {
  goto(pageId: string, params?: Record<string, string>): Promise<void>;
  openPopup(pageId: string, params?: Record<string, string>): Promise<string>;
  closePopup(id?: string): void;
}

Example SCADA use cases:
- click a tank to open tank detail page
- click a motor to open a faceplate popup
- right-click a pump to open trend popup

================================================================================
27. THEMING MODEL
================================================================================

Implement theme propagation through the host.

Requirements:
- widgets must be able to adapt to light/dark themes
- theme should be accessible through CSS variables and WidgetContext.ui.theme()
- custom elements may use shadow DOM, but should still inherit design tokens via CSS variables

Suggested token groups:
- background
- surface
- border
- text
- accent
- success
- warning
- danger
- quality-good
- quality-bad
- quality-uncertain
- quality-stale

================================================================================
28. VALIDATION IN THE EDITOR
================================================================================

The editor must validate widget instances before runtime.

Validate:
- widget type exists
- plugin version compatible
- required props present
- required bindings present
- binding direction compatible with the widget schema
- tag access allowed for current user/session if applicable
- expressions syntactically valid
- event actions valid
- page references valid

Show warnings/errors in the inspector and optionally as badges on the node shell.

================================================================================
29. DEV EXPERIENCE AND TOOLING
================================================================================

Add the following:
- TypeScript types for plugin authors
- sample plugin package
- dev-only diagnostics overlay
- runtime logger with widget instance context
- unit tests for TagBroker
- unit tests for ActionEngine
- unit tests for manifest validation
- integration test for mounting a custom element widget in WidgetNodeShell
- integration test for navigating on click
- integration test for writing a tag through action engine
- integration test for context menu contribution merging

================================================================================
30. SECURITY AND STABILITY RULES
================================================================================

Mandatory rules:
- no eval()
- no direct plugin access to transport
- no implicit trust of widget actions
- all writes go through permission checks
- all navigation goes through action engine or PageRuntimeService
- sanitize any HTML from plugin props if ever allowed
- treat third-party widget CSS carefully to avoid bleeding into host if shadow DOM is not used
- catch and isolate plugin exceptions
- support plugin API version negotiation

Optional future hardening:
- plugin signatures
- lazy-loading plugins from approved registries
- stronger sandbox mode for untrusted vendors
- iframe mode only for truly untrusted plugins, not as the default

================================================================================
31. PHASED IMPLEMENTATION PLAN
================================================================================

Phase 1: Foundations
- scaffold SvelteKit app
- install @xyflow/svelte
- implement basic editor and runtime routes
- create WidgetManifest, WidgetRegistry, TagBroker, ProcessTransport, ActionEngine, PermissionService interfaces
- create WidgetNodeShell.svelte
- create BrokenWidget.svelte
- create simple page JSON model

Phase 2: Built-in Widgets
- implement built-in Button, TextIndicator, Tank widgets
- implement property inspector and binding editor
- implement runtime mounting path
- implement minimal action engine with navigate, writeTag, showContextMenu

Phase 3: Third-Party Plugin Model
- implement CustomElementWidgetAdapter
- implement example vanilla-button custom element
- register example plugin through WidgetRegistry
- document plugin author contract

Phase 4: Advanced Runtime
- add popup manager
- add safe expression engine
- add diagnostics overlay
- add better stale quality handling
- add batching/throttling for high-frequency updates

Phase 5: Hardening
- add manifest validation UI
- add plugin compatibility checks
- add test coverage
- add permission enforcement throughout writes/navigation/actions
- add plugin loading strategy and version pinning

================================================================================
32. CONCRETE CODING PRIORITIES
================================================================================

Codex should implement in this order:

1. plugin API types
2. widget registry
3. tag broker
4. process transport stub/mock
5. action engine
6. page models
7. widget runtime host
8. Svelte Flow widget node shell
9. built-in Button widget
10. built-in TextIndicator widget
11. built-in Tank widget
12. custom element adapter
13. example vanilla widget plugin
14. editor inspectors
15. runtime route and page manager
16. diagnostics + tests

================================================================================
33. EXPECTED CODE STYLE
================================================================================

Code style requirements:
- strongly typed TypeScript
- small focused modules
- no giant god classes unless clearly justified
- clear interfaces before implementations
- comments only where they add architectural clarity
- prefer explicit naming over clever abstractions
- no hidden global mutable state except intentional singleton runtime services
- keep all plugin API types in one stable public package/folder
- make extension points obvious and well documented

================================================================================
34. ACCEPTANCE CRITERIA
================================================================================

The implementation is considered successful when all of the following are true:

- A page can be built in Svelte Flow with multiple widget nodes.
- Each widget node can bind one or more named variables.
- A central broker receives variable updates from a shared transport and only updates interested widgets.
- A widget can emit click, doubleClick, and rightClick semantic events.
- The action engine can navigate to another page from a widget click.
- The action engine can write to a variable when a widget action is triggered.
- A tank widget can display multiple bound values such as level, temperature, and pressure.
- A widget can contribute dynamic context menu items through the host.
- A third-party vanilla custom element widget can be registered and mounted without direct coupling to Svelte internals.
- Plugin errors are isolated to the widget and do not crash the whole page.
- Edit mode and runtime mode behavior are clearly separated.
- Permissions are enforced for writes and navigation.

================================================================================
35. FINAL IMPLEMENTATION DIRECTIVE FOR CODEX
================================================================================

Build the system as a reusable platform, not as a one-off page renderer.

Prioritize:
- inversion of control
- stable plugin API
- performance under frequent tag updates
- clear editor/runtime separation
- declarative event-to-action architecture
- framework-neutral third-party widget development
- resilient failure isolation

Do not optimize prematurely for untrusted sandboxed plugins.
First build the highest-quality same-window plugin architecture with host-controlled capabilities.
Keep the API clean enough so a stricter sandbox mode can be added later without rewriting the widget model.

Deliverables:
- working SvelteKit project
- Svelte Flow-based editor shell
- runtime page renderer
- widget registry
- tag broker + transport stub
- action engine
- three built-in widgets
- one example third-party vanilla widget plugin
- documentation comments and README for plugin authors

// EXAMPLE: how a third-party user would declare a custom SCADA node/widget
// using the proposed IoC approach.
//
// This example shows:
//
// 1) A custom element implemented with vanilla JS/TS style
// 2) A plugin manifest describing props, bindings, and events
// 3) A plugin object exported to the host registry
// 4) A sample page-node instance showing how the widget is configured
//
// The key idea:
// - The widget does NOT access WebSocket directly
// - The widget does NOT navigate directly
// - The widget does NOT mutate host state directly
// - The widget only uses the injected host API (WidgetContext)

////////////////////////////////////////////////////////////////////////////////
// 1) TYPES THE HOST EXPOSES TO PLUGIN AUTHORS
////////////////////////////////////////////////////////////////////////////////

type BindingDirection = 'read' | 'write' | 'readwrite';
type Quality = 'good' | 'bad' | 'uncertain' | 'stale' | 'disconnected';

interface TagSample<T = unknown> {
  value: T;
  quality: Quality;
  ts: number;
  seq?: number;
}

interface TagBindingRef {
  tagId: string;
  access: BindingDirection;
  label?: string;
}

interface ContextMenuItem {
  id: string;
  label: string;
  icon?: string;
  enabled?: boolean;
}

interface WidgetContext {
  instanceId: string;
  widgetId: string;
  pageId: string;
  mode: 'editor' | 'runtime';

  props: Readonly<Record<string, unknown>>;
  layout: Readonly<{
    width: number;
    height: number;
    zoom: number;
    selected: boolean;
    disabled: boolean;
  }>;

  tags: {
    getBinding(name: string): TagBindingRef | TagBindingRef[] | undefined;
    read(name: string): TagSample | TagSample[] | undefined;
    subscribe(name: string, cb: (sample: TagSample | TagSample[]) => void): () => void;
    write(name: string, value: unknown, options?: Record<string, unknown>): Promise<unknown>;
    list(): { name: string; binding: TagBindingRef | TagBindingRef[] }[];
  };

  events: {
    emit(name: string, payload?: unknown): void;
    openContextMenu(items: ContextMenuItem[], event?: MouseEvent | PointerEvent): void;
  };

  nav: {
    goto(pageId: string, params?: Record<string, string>): Promise<void>;
    openPopup(pageId: string, params?: Record<string, string>): Promise<void>;
    closePopup(popupId?: string): void;
  };

  ui: {
    theme(): Record<string, unknown>;
    setStatus(status: 'ok' | 'warning' | 'alarm' | 'stale' | 'error'): void;
    requestRender(): void;
  };

  lifecycle: {
    onDispose(cb: () => void): void;
  };

  logger: {
    debug(...args: unknown[]): void;
    warn(...args: unknown[]): void;
    error(...args: unknown[]): void;
  };
}

interface BindingSchema {
  name: string;
  label: string;
  direction: BindingDirection;
  types: string[];
  required?: boolean;
}

interface PropSchema {
  name: string;
  type: 'string' | 'number' | 'boolean' | 'color' | 'select';
  label: string;
  default?: unknown;
  options?: { label: string; value: string }[];
}

interface EventSchema {
  name: string;
  label: string;
}

interface WidgetManifest {
  type: string;
  version: string;
  apiVersion: string;
  displayName: string;
  category: string;
  bindings: BindingSchema[];
  props: PropSchema[];
  events: EventSchema[];
  capabilities: {
    readsTags: boolean;
    writesTags: boolean;
    contributesContextMenu: boolean;
    resizable: boolean;
  };
}

interface WidgetInstance {
  mount(el: HTMLElement): void;
  update(input: {
    props: Record<string, unknown>;
    layout: {
      width: number;
      height: number;
      zoom: number;
      selected: boolean;
      disabled: boolean;
    };
    bindings: Record<string, TagBindingRef | TagBindingRef[]>;
    mode: 'editor' | 'runtime';
  }): void;
  dispose(): void;
}

interface ScadaWidgetPlugin {
  manifest: WidgetManifest;
  create(ctx: WidgetContext): WidgetInstance;
}

////////////////////////////////////////////////////////////////////////////////
// 2) CUSTOM ELEMENT IMPLEMENTED BY THE THIRD-PARTY USER
////////////////////////////////////////////////////////////////////////////////

class TankSummaryElement extends HTMLElement {
  private root!: ShadowRoot;
  private unsubscribers: Array<() => void> = [];

  // Injected by the host adapter:
  public widgetContext!: WidgetContext;
  public widgetProps: Record<string, unknown> = {};
  public widgetBindings: Record<string, TagBindingRef | TagBindingRef[]> = {};
  public widgetLayout!: {
    width: number;
    height: number;
    zoom: number;
    selected: boolean;
    disabled: boolean;
  };
  public widgetMode: 'editor' | 'runtime' = 'runtime';

  private titleEl!: HTMLDivElement;
  private levelEl!: HTMLDivElement;
  private tempEl!: HTMLDivElement;
  private pressureEl!: HTMLDivElement;

  constructor() {
    super();
    this.root = this.attachShadow({ mode: 'open' });
    this.renderSkeleton();
  }

  connectedCallback() {
    this.bindDomEvents();
    this.syncFromInputs();
    this.startSubscriptions();
  }

  disconnectedCallback() {
    this.cleanup();
  }

  // The host updates properties directly; this lets the widget react.
  set widgetPropsValue(value: Record<string, unknown>) {
    this.widgetProps = value;
    this.syncFromInputs();
  }

  set widgetBindingsValue(value: Record<string, TagBindingRef | TagBindingRef[]>) {
    this.widgetBindings = value;
    this.restartSubscriptions();
  }

  set widgetLayoutValue(value: {
    width: number;
    height: number;
    zoom: number;
    selected: boolean;
    disabled: boolean;
  }) {
    this.widgetLayout = value;
    this.syncFromInputs();
  }

  set widgetModeValue(value: 'editor' | 'runtime') {
    this.widgetMode = value;
    this.syncFromInputs();
  }

  private renderSkeleton() {
    const style = document.createElement('style');
    style.textContent = `
      :host {
        display: block;
        width: 100%;
        height: 100%;
        box-sizing: border-box;
        font-family: sans-serif;
      }

      .tank {
        width: 100%;
        height: 100%;
        border: 1px solid #888;
        border-radius: 8px;
        padding: 10px;
        box-sizing: border-box;
        background: #111;
        color: #eee;
        display: grid;
        grid-template-rows: auto 1fr auto auto;
        gap: 8px;
        user-select: none;
      }

      .title {
        font-weight: 700;
        font-size: 14px;
      }

      .level {
        display: grid;
        align-items: end;
        min-height: 80px;
        border: 1px solid #555;
        border-radius: 6px;
        overflow: hidden;
        background: #1a1a1a;
        position: relative;
      }

      .fill {
        width: 100%;
        background: #2f9bff;
        transition: height 120ms linear;
        align-self: end;
      }

      .overlay {
        position: absolute;
        inset: 0;
        display: grid;
        place-items: center;
        font-size: 12px;
        font-weight: 600;
      }

      .metric {
        font-size: 12px;
      }

      .stale {
        opacity: 0.65;
      }

      .selected {
        outline: 2px solid #ffd54a;
      }
    `;

    const wrapper = document.createElement('div');
    wrapper.className = 'tank';
    wrapper.innerHTML = `
      <div class="title"></div>
      <div class="level">
        <div class="fill" style="height: 0%;"></div>
        <div class="overlay">--%</div>
      </div>
      <div class="metric temp">Temp: --</div>
      <div class="metric pressure">Pressure: --</div>
    `;

    this.root.append(style, wrapper);

    this.titleEl = wrapper.querySelector('.title') as HTMLDivElement;
    this.levelEl = wrapper.querySelector('.level') as HTMLDivElement;
    this.tempEl = wrapper.querySelector('.temp') as HTMLDivElement;
    this.pressureEl = wrapper.querySelector('.pressure') as HTMLDivElement;
  }

  private bindDomEvents() {
    this.addEventListener('click', this.onClick);
    this.addEventListener('dblclick', this.onDoubleClick);
    this.addEventListener('contextmenu', this.onContextMenu);
  }

  private onClick = () => {
    if (this.widgetMode !== 'runtime') return;

    // The widget emits a semantic event.
    // The host action engine decides what to do with it.
    this.widgetContext.events.emit('click', {
      source: 'tank-summary',
      instanceId: this.widgetContext.instanceId,
    });
  };

  private onDoubleClick = () => {
    if (this.widgetMode !== 'runtime') return;

    this.widgetContext.events.emit('doubleClick', {
      source: 'tank-summary',
      instanceId: this.widgetContext.instanceId,
    });
  };

  private onContextMenu = (event: MouseEvent) => {
    event.preventDefault();

    const items: ContextMenuItem[] = [
      { id: 'open-faceplate', label: 'Open faceplate' },
      { id: 'open-trend', label: 'Open trend' },
    ];

    // The widget contributes menu items.
    // The host renders the menu and executes allowed actions.
    this.widgetContext.events.openContextMenu(items, event);
    this.widgetContext.events.emit('rightClick', { x: event.clientX, y: event.clientY });
  };

  private syncFromInputs() {
    const title = String(this.widgetProps.title ?? 'Tank');
    this.titleEl.textContent = title;

    const tank = this.root.querySelector('.tank') as HTMLDivElement;
    tank.classList.toggle('selected', !!this.widgetLayout?.selected);
    tank.classList.toggle('stale', false);
  }

  private restartSubscriptions() {
    this.cleanupSubscriptions();
    this.startSubscriptions();
  }

  private startSubscriptions() {
    if (!this.widgetContext) return;

    const levelNow = this.widgetContext.tags.read('level') as TagSample<number> | undefined;
    if (levelNow) this.renderLevel(levelNow);

    const tempNow = this.widgetContext.tags.read('temperature') as TagSample<number> | undefined;
    if (tempNow) this.renderTemp(tempNow);

    const pressureNow = this.widgetContext.tags.read('pressure') as TagSample<number> | undefined;
    if (pressureNow) this.renderPressure(pressureNow);

    this.unsubscribers.push(
      this.widgetContext.tags.subscribe('level', (sample) => {
        this.renderLevel(sample as TagSample<number>);
      })
    );

    this.unsubscribers.push(
      this.widgetContext.tags.subscribe('temperature', (sample) => {
        this.renderTemp(sample as TagSample<number>);
      })
    );

    this.unsubscribers.push(
      this.widgetContext.tags.subscribe('pressure', (sample) => {
        this.renderPressure(sample as TagSample<number>);
      })
    );
  }

  private renderLevel(sample: TagSample<number>) {
    const min = Number(this.widgetProps.min ?? 0);
    const max = Number(this.widgetProps.max ?? 100);
    const raw = Number(sample.value ?? 0);
    const pct = Math.max(0, Math.min(100, ((raw - min) / (max - min)) * 100));

    const fill = this.root.querySelector('.fill') as HTMLDivElement;
    const overlay = this.root.querySelector('.overlay') as HTMLDivElement;

    fill.style.height = `${pct}%`;
    overlay.textContent = `${raw.toFixed(1)}%`;

    this.widgetContext.ui.setStatus(sample.quality === 'good' ? 'ok' : 'stale');
  }

  private renderTemp(sample: TagSample<number>) {
    this.tempEl.textContent = `Temp: ${Number(sample.value).toFixed(1)} °C`;
  }

  private renderPressure(sample: TagSample<number>) {
    this.pressureEl.textContent = `Pressure: ${Number(sample.value).toFixed(1)} bar`;
  }

  private cleanupSubscriptions() {
    for (const unsub of this.unsubscribers) unsub();
    this.unsubscribers = [];
  }

  private cleanup() {
    this.cleanupSubscriptions();
    this.removeEventListener('click', this.onClick);
    this.removeEventListener('dblclick', this.onDoubleClick);
    this.removeEventListener('contextmenu', this.onContextMenu);
  }
}

customElements.define('scada-tank-summary', TankSummaryElement);

////////////////////////////////////////////////////////////////////////////////
// 3) THIRD-PARTY PLUGIN DECLARATION
////////////////////////////////////////////////////////////////////////////////

export const tankSummaryPlugin: ScadaWidgetPlugin = {
  manifest: {
    type: 'vendor.tank-summary',
    version: '1.0.0',
    apiVersion: '1.0.0',
    displayName: 'Tank Summary',
    category: 'Process',
    bindings: [
      {
        name: 'level',
        label: 'Level',
        direction: 'read',
        types: ['number'],
        required: true,
      },
      {
        name: 'temperature',
        label: 'Temperature',
        direction: 'read',
        types: ['number'],
        required: false,
      },
      {
        name: 'pressure',
        label: 'Pressure',
        direction: 'read',
        types: ['number'],
        required: false,
      },
      {
        name: 'setpoint',
        label: 'Setpoint',
        direction: 'write',
        types: ['number'],
        required: false,
      },
    ],
    props: [
      {
        name: 'title',
        type: 'string',
        label: 'Title',
        default: 'Tank',
      },
      {
        name: 'min',
        type: 'number',
        label: 'Minimum',
        default: 0,
      },
      {
        name: 'max',
        type: 'number',
        label: 'Maximum',
        default: 100,
      },
    ],
    events: [
      { name: 'click', label: 'Click' },
      { name: 'doubleClick', label: 'Double Click' },
      { name: 'rightClick', label: 'Right Click' },
    ],
    capabilities: {
      readsTags: true,
      writesTags: true,
      contributesContextMenu: true,
      resizable: true,
    },
  },

  create(ctx: WidgetContext): WidgetInstance {
    let el: TankSummaryElement | null = null;

    return {
      mount(container: HTMLElement) {
        el = document.createElement('scada-tank-summary') as TankSummaryElement;

        // Inject host-controlled capabilities
        el.widgetContext = ctx;
        el.widgetProps = { ...ctx.props };
        el.widgetBindings = Object.fromEntries(
          ctx.tags.list().map(({ name, binding }) => [name, binding])
        );
        el.widgetLayout = { ...ctx.layout };
        el.widgetMode = ctx.mode;

        container.appendChild(el);
      },

      update(input) {
        if (!el) return;
        el.widgetPropsValue = input.props;
        el.widgetBindingsValue = input.bindings;
        el.widgetLayoutValue = input.layout;
        el.widgetModeValue = input.mode;
      },

      dispose() {
        if (el?.parentNode) {
          el.parentNode.removeChild(el);
        }
        el = null;
      },
    };
  },
};

////////////////////////////////////////////////////////////////////////////////
// 4) HOW THE HOST APPLICATION REGISTERS THE PLUGIN
////////////////////////////////////////////////////////////////////////////////

// Example:
class WidgetRegistry {
  private map = new Map<string, ScadaWidgetPlugin>();

  register(plugin: ScadaWidgetPlugin) {
    if (this.map.has(plugin.manifest.type)) {
      throw new Error(`Widget type already registered: ${plugin.manifest.type}`);
    }
    this.map.set(plugin.manifest.type, plugin);
  }

  resolve(type: string) {
    return this.map.get(type);
  }
}

const widgetRegistry = new WidgetRegistry();
widgetRegistry.register(tankSummaryPlugin);

////////////////////////////////////////////////////////////////////////////////
// 5) HOW A USER OF THE SCADA DECLARES A NODE INSTANCE IN A PAGE
////////////////////////////////////////////////////////////////////////////////

// This is not plugin code.
// This is the page JSON an HMI author would create in the SCADA editor/runtime.

const tankNodeInstance = {
  id: 'node-tank-01',
  type: 'widget-node', // Svelte Flow node shell type
  position: { x: 220, y: 140 },
  width: 220,
  height: 260,
  data: {
    widgetType: 'vendor.tank-summary',
    widgetVersion: '1.0.0',
    props: {
      title: 'Tank 7',
      min: 0,
      max: 100,
    },
    bindings: {
      level: {
        tagId: 'Plant1.AreaA.Tank7.Level',
        access: 'read',
      },
      temperature: {
        tagId: 'Plant1.AreaA.Tank7.Temperature',
        access: 'read',
      },
      pressure: {
        tagId: 'Plant1.AreaA.Tank7.Pressure',
        access: 'read',
      },
      setpoint: {
        tagId: 'Plant1.AreaA.Tank7.LevelSP',
        access: 'write',
      },
    },
    eventBindings: [
      {
        on: 'click',
        do: [
          {
            type: 'navigate',
            params: {
              pageId: 'tank-detail-page',
              params: {
                assetId: 'Tank7',
              },
            },
          },
        ],
      },
      {
        on: 'rightClick',
        do: [
          {
            type: 'showContextMenu',
            params: {
              menuId: 'tank-default-menu',
            },
          },
        ],
      },
    ],
  },
};

////////////////////////////////////////////////////////////////////////////////
// 6) WHAT THIS GIVES YOU
////////////////////////////////////////////////////////////////////////////////

// - Third-party author declares widget type through a manifest
// - Widget is framework-neutral and can be vanilla JS
// - Widget receives controlled capabilities from the host
// - Widget can subscribe reactively to named bindings
// - Widget emits semantic events instead of performing privileged actions
// - Page author decides what widget events trigger through declarative actions
// - The host remains in control of navigation, writes, permissions, and menus

// A shorter example for plugin authors could look like this:

export const simpleButtonPlugin: ScadaWidgetPlugin = {
  manifest: {
    type: 'vendor.simple-button',
    version: '1.0.0',
    apiVersion: '1.0.0',
    displayName: 'Simple Button',
    category: 'Controls',
    bindings: [
      { name: 'command', label: 'Command', direction: 'write', types: ['boolean'], required: false }
    ],
    props: [
      { name: 'label', type: 'string', label: 'Label', default: 'Button' }
    ],
    events: [
      { name: 'click', label: 'Click' }
    ],
    capabilities: {
      readsTags: false,
      writesTags: true,
      contributesContextMenu: false,
      resizable: true,
    },
  },

  create(ctx) {
    let button: HTMLButtonElement | null = null;

    return {
      mount(container) {
        button = document.createElement('button');
        button.textContent = String(ctx.props.label ?? 'Button');

        button.onclick = async () => {
          ctx.events.emit('click');

          // Optional direct host-mediated write by named binding
          const hasCommand = !!ctx.tags.getBinding('command');
          if (hasCommand && ctx.mode === 'runtime') {
            await ctx.tags.write('command', true);
          }
        };

        container.appendChild(button);
      },

      update(input) {
        if (button) {
          button.textContent = String(input.props.label ?? 'Button');
        }
      },

      dispose() {
        button?.remove();
        button = null;
      },
    };
  },
};

Add plugin trust levels:

type PluginTrustLevel = 
  | 'internal'
  | 'partner'
  | 'untrusted';