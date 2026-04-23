# Frontend Architecture (LLM-Oriented, Current Implementation)

This is the source-of-truth architecture map for `frontend/`.
It is intentionally optimized for LLM contributors who need to make safe, fast changes.

Assume this file is authoritative over older design notes.
When code and docs conflict, code wins and this file should be updated.

## 0) LLM Quickstart

### Fast reading order (minimum context)

1. `src/routes/(app)/+layout.svelte`
2. `src/routes/(app)/+layout.ts`
3. `src/routes/runtime/[id]/+page.svelte`
4. `src/lib/core/ws/tag-stream-client.ts`
5. `src/lib/features/realtime/page-tag-realtime-provider.ts`
6. `src/lib/features/workspace/layout-graph-history.ts`
7. `src/lib/features/workspace/layout-graph-order.ts`
8. `src/lib/features/workspace/layout-graph-metadata.ts`
9. `src/lib/features/graph/live-utils.ts`
10. `src/lib/features/graph/widget-handlers.ts`
11. `src/lib/scada/plugins/internal-widgets/runtime/widget-runtime.ts`
12. `src/lib/features/views/api/views-api.ts`
13. `src/lib/features/tree/components/VariableTree.svelte`
14. `src/lib/features/tree/tree-store.ts`
15. `src/lib/features/tree/tree-remote-reconcile.ts`

### High-signal commands

- Type checks: `npm run check`
- Unit tests: `npm run test`
- Frontend build: `npm run build`

## 1) Runtime and Build Model

- Stack: SvelteKit 2 + Svelte 5 + TypeScript + Tailwind 4.
- Rendering mode: client-only SPA.
  - `src/routes/+layout.ts`: `ssr = false`, `prerender = true`.
- Adapter: `@sveltejs/adapter-static`.
  - `svelte.config.js` outputs to `frontend/build`.
- Deep-link safety:
  - `kit.paths.relative = false` ensures absolute `/_app/...` chunk paths.
- Dev networking (`vite.config.ts`):
  - `/api` -> `http://127.0.0.1:8245`
  - `/ws` -> `ws://127.0.0.1:8245`
- WS endpoint source of truth: `resolveTagStreamWsEndpoint()`.
  - Browser: same-origin + `/ws`
  - Non-browser fallback: `ws://127.0.0.1:8245/ws`

## 2) Route Topology and Auth Rules

### Root shell

- `src/routes/+layout.svelte` handles:
  - global CSS/theme initialization
  - global snackbar
  - guarded route preloading (`preloadCode` + controlled `goto`)
  - navigation progress bar
  - chunk-load failure fallback with hard navigation reload

### Guest/app groups

- `src/routes/(guest)/+layout.ts`
  - if authenticated or auth disabled, redirects to `/`
- `src/routes/(app)/+layout.ts`
  - fetches `/api/auth/status`
  - redirects unauthenticated users to `/auth/login`
  - redirects setup-needed state to `/auth/setup`
  - operator users are redirected to runtime entry:
    - `/runtime/:entryPointId` when configured
    - `/runtime/no-entry` when not configured

### Runtime routes

- `src/routes/runtime/+page.ts`
  - resolves entry point and redirects
  - fallback is `/runtime/no-entry`
- `src/routes/runtime/no-entry/+page.svelte`
  - explicit operator-safe empty-state page
- `src/routes/runtime/+layout.svelte`
  - runtime shell + toolbar
  - workspace mode switch hidden for role `operator`

### Designer routes

- `/views` and `/views/[id]` page files are placeholders.
- Real stateful logic lives in `src/routes/(app)/+layout.svelte`.

## 3) Data Channels

### REST

- `src/lib/features/views/api/views-api.ts` wraps:
  - `GET /api/views`
  - `GET /api/views/:id`
  - `POST /api/views`
  - `PUT /api/views/:id`
  - `DELETE /api/views/:id`
  - `GET /api/views/entry-point`
  - `PUT /api/views/:id/entry-point`
- `src/lib/core/http/api-fetch.ts` redirects browser to `/auth/login` on API `401`.

### WebSocket

- `src/lib/core/ws/tag-stream-client.ts` is the shared command/event client for:
  - tree LIST/ADD/DEL/EDIT_META/ADD_BULK
  - value GET/SET
  - SUB/UNSUB realtime values
  - app-lifetime SUB to `EVENT_TYPE_TREE_CHANGE`
- Command timeout: `COMMAND_TIMEOUT_MS = 60_000`.
- `send(command)` returns `boolean`; failed sends reject pending requests immediately.

### Page-level realtime provider

- `src/lib/features/realtime/page-tag-realtime-provider.ts` wraps `tagStreamClient`.
- Inputs:
  - `setActive(boolean)`
  - `setDesiredIds(string[])`
- Effective tracked IDs are deduplicated and only pushed when set membership changes.

## 4) Designer Workspace (`(app)/+layout.svelte`)

`src/routes/(app)/+layout.svelte` is the main orchestrator.
It owns route-driven mode switches and coordinates all feature modules.

### Primary UI composition

- Toolbar: `PageToolbar`
- Left pane: `VariableTree`
- Right pane:
  - `ViewsListPanel` when route is `/views`
  - View editor (`ViewEditorHeader` + `SvelteFlow` + `GraphInspector`) when `/views/:id`
- Dialogs: `WorkspaceConfirmDialogs`
- Namespace modal: `NamespaceBuilder`

### Mode/state boundaries

- `rightPaneMode`: `views-list | view-editor` (derived from route param)
- `canvasMode`: `edit | play`
- `workspaceMode`: `designer | runtime`
- `canDropToCanvas`: only true in `view-editor + edit`

### Extracted helper modules (critical refactor)

- History and snapshotting:
  - `src/lib/features/workspace/layout-graph-history.ts`
- Z-order / layering:
  - `src/lib/features/workspace/layout-graph-order.ts`
- Metadata propagation:
  - `src/lib/features/workspace/layout-graph-metadata.ts`

These modules are pure logic helpers and should stay framework-agnostic.

### Request race-safety pattern

The layout uses serial counters to ignore stale async results:

- `viewsListRequestSerial` for list requests
- `viewEditorRequestSerial` for loading a specific editor view
- `graphLiveValuesRequestSerial` for backend live-value refreshes

If you add async loaders here, follow the same serial pattern.

### Leave-view guard behavior

- Navigation away from `/views/:id` is intercepted via `beforeNavigate`.
- Confirmation dialog is shown unless bypass flags are set.
- Two bypass flags exist:
  - `allowConfirmedLeaveNavigation`
  - `bypassLeaveViewGuard`
- `bypassLeaveViewGuard` is required for internal flows like deleting current view then `goto("/views")`.

### Widget handler lifecycle

- Handlers are registered by `symbolId`.
- Stale handlers are proactively unregistered via effect comparing current graph symbol IDs.
- Selected-node deletion and view unload also unregister handlers.

## 5) Graph and Widget Model

### Asset registry

- Registry: `src/lib/features/graph/assets/registry.ts`
- Types: `src/lib/features/graph/assets/types.ts`
- Internal widgets plugin registration:
  - `src/lib/scada/plugins/internal-widgets-plugin.ts`
  - widget manifests define bindings, config schema, supported events

### Node payload constraints

- Runtime callbacks are never persisted in serialized canvas JSON.
- Serialization sanitizers live in:
  - `src/lib/features/views/types.ts`
  - `src/lib/features/workspace/layout-graph-history.ts`
- Hydration rebuilds runtime handlers via `registerWidgetHandlers(...)`.

### Handler bridge

- Handler registry: `src/lib/features/graph/widget-handlers.ts`
- Runtime resolution bridge: `resolveWidgetHandlers(...)` in
  `src/lib/scada/plugins/internal-widgets/runtime/widget-runtime.ts`
- Runtime bridge uses registry first and only falls back to legacy in-node callbacks.

### Live values

- Tracking/index logic: `src/lib/features/graph/live-utils.ts`
- Optimized path updates only impacted nodes using dependency index:
  - `buildGraphLiveDependencyIndex`
  - `applyLiveValuesToGraphNodesAtIndexes`

### Connectors

- Connector normalization and config:
  - `src/lib/features/graph/connectors.ts`
- Normalization uses structural comparators (not JSON stringify) to detect no-op changes.

## 6) Runtime View Route (`runtime/[id]`)

- File: `src/routes/runtime/[id]/+page.svelte`
- Responsibilities:
  - fetch/hydrate view graph
  - run realtime subscriptions for current view
  - apply live values
  - apply metadata refreshes
  - handle widget interaction actions in runtime mode

### Runtime request freshness

- Uses `requestCounter` to ignore stale load responses.
- On route change:
  - immediately `setDesiredIds([])` before loading next view
- On leaving `/runtime`:
  - deactivates provider

### Runtime widget actions

- `navigateRuntimeView`: `goto("/runtime/:id")`
- `openContextMenu`: renders runtime context menu with action items

## 7) Tree Domain

### Core pieces

- Store and visible-row derivation: `src/lib/features/tree/tree-store.ts`
- Server adapter: `src/lib/features/tree/server-adapter.ts`
- Push reconcile logic: `src/lib/features/tree/tree-remote-reconcile.ts`
- UI virtualization: `src/lib/features/tree/components/VariableTree.svelte`

### Selection and delete behavior

- Multi-select primitives: `src/lib/features/tree/tree-selection.ts`
- Indeterminate summary optimization:
  - `src/lib/features/tree/tree-selection-summary.ts`
  - avoids recursive partial-check per row render
- Delete payload minimization:
  - `getMinimalAncestorSet(...)` sends only superior selected ancestors

### Cache and remote updates

- Tree snapshot cache: `src/lib/features/tree/tree-cache.ts`
- Global tree-change WS feed reconciles state incrementally when possible.

## 8) Namespace Builder

- Main component:
  - `src/lib/features/namespace-builder/components/NamespaceBuilder.svelte`
- Parser/serializer and AST logic:
  - `src/lib/features/namespace-builder/namespace-yaml.ts`
- Monaco mode/theme/completion:
  - `src/lib/features/namespace-builder/monaco-yaml-config.ts`
- Submit path:
  - `tagStreamClient.addBulkNamespace(...)`

## 9) Cross-Cutting Invariants (Do Not Break)

1. Do not store runtime functions in persisted `canvas_json`.
2. Always clear or unregister widget handlers when unloading/replacing graph state.
3. Realtime subscription transitions must clear old IDs before switching views.
4. In async view/list loaders, stale responses must not overwrite current state.
5. Keep `/runtime/no-entry` as the fallback for missing runtime entry-point.
6. Keep `/views` and `/views/[id]` page files as placeholders unless layout ownership is intentionally redesigned.

## 10) Open-Source Maintainability Notes

### Current intentional tradeoff

- `src/routes/(app)/+layout.svelte` remains a large orchestrator by design.
- Complexity is reduced by extracting pure helper modules and reusable UI components.

### Preferred extension strategy

- Add new pure transformations in `src/lib/features/workspace/*` or `src/lib/features/graph/*`.
- Keep route components focused on orchestration and event wiring.
- Add unit tests beside each new pure module (`*.spec.ts`).
- Reuse request-serial and lifecycle patterns already present.

## 11) LLM Task Routing Map

### If the task is...

- Auth or redirects:
  - `src/routes/(app)/+layout.ts`
  - `src/routes/(guest)/+layout.ts`
  - `src/routes/runtime/+page.ts`
- Designer load/save/race conditions:
  - `src/routes/(app)/+layout.svelte`
  - `src/lib/features/views/api/views-api.ts`
- Graph history or ordering:
  - `src/lib/features/workspace/layout-graph-history.ts`
  - `src/lib/features/workspace/layout-graph-order.ts`
- Metadata propagation:
  - `src/lib/features/workspace/layout-graph-metadata.ts`
- Realtime behavior:
  - `src/lib/features/realtime/page-tag-realtime-provider.ts`
  - `src/lib/core/ws/tag-stream-client.ts`
  - `src/lib/features/graph/live-utils.ts`
- Runtime interaction behavior:
  - `src/routes/runtime/[id]/+page.svelte`
  - `src/lib/scada/plugins/internal-widgets/runtime/widget-runtime.ts`
- Tree performance/selection:
  - `src/lib/features/tree/components/VariableTree.svelte`
  - `src/lib/features/tree/tree-selection-summary.ts`
  - `src/lib/features/tree/tree-selection.ts`

## 12) Test Surface Snapshot

High-value test coverage currently exists for:

- WS command client and transport behavior
- Realtime provider behavior
- Graph connectors/live utils/widget handlers/type compatibility
- Workspace helper modules (`layout-graph-*`)
- Tree store/selection/reconcile/server adapter/flatten
- Namespace builder parser/Monaco helpers

When changing behavior in these modules, update existing specs first, then adjust UI integration.
