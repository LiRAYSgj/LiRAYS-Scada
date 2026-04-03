# LiRAYS SCADA Frontend Architecture

This document reflects the current `frontend/` implementation.

## Runtime Model

- Stack: SvelteKit 2 + Svelte 5 runes + TypeScript + Tailwind CSS 4.
- Rendering mode:
  - `src/routes/+layout.ts` sets `ssr = false` and `prerender = true`.
  - `svelte.config.js` uses `@sveltejs/adapter-static` with output to `build/`.
- Route surface: a single app route (`src/routes/+page.svelte`) inside a shared layout.

## App Shell

- `src/routes/+layout.svelte`:
  - loads global styles (`layout.css`),
  - initializes theme from `localStorage`,
  - mounts a global snackbar overlay.
- Theme state:
  - `themeStore` (`src/lib/stores/theme.ts`) persists `light|dark`,
  - page applies `theme-light` / `theme-dark` on `<html>`,
  - `createThemeVars` injects CSS variables used by workspace components.

## Transport and Backend Contract

All backend interaction is over protobuf WebSocket frames.

- Endpoint resolution: `resolveTagStreamWsEndpoint()`
  - Dev/non-browser: `ws://127.0.0.1:8245/ws`
  - Browser production: same host, `/ws`, protocol-aware (`ws`/`wss`)
- Core client: singleton `TagStreamClient` (`src/lib/core/ws/tag-stream-client.ts`)
  - Exposes stores: connection `status`, live `values`, and pushed `treeChanges`.
  - Handles reconnect with backoff.
  - Correlates command responses by `cmd_id`.
  - Sends commands for:
    - `LIST` (`listChildren`)
    - `ADD` (`addItem`)
    - `DEL` (`removeItems`)
    - `SET` (`sendWriteValue`)
    - `EDIT_META` (`updateMeta`)
    - `ADD_BULK` (`addBulkNamespace`)
  - Subscriptions:
    - Always subscribes to tree-change events after socket open.
    - Subscribes/unsubscribes variable value events based on desired tracked IDs.
  - Decoding:
    - Handles both `Response` envelopes and pushed `Event` frames.
    - Includes legacy fallback for bare `VarIdValue` pushes.

## Feature Composition on Main Page

`src/routes/+page.svelte` composes four major areas:

- toolbar (`PageToolbar`)
- namespace tree (`VariableTree`)
- graph canvas (`@xyflow/svelte`)
- namespace template builder dialog (`NamespaceBuilder`)

It also controls two modes:

- `edit`: graph editing enabled, realtime value tracking disabled.
- `play`: graph editing disabled, realtime tracking enabled.

## Realtime Orchestration

- `createPageTagRealtimeProvider` (`src/lib/features/realtime/page-tag-realtime-provider.ts`)
  wraps `TagStreamClient` for page-level behavior.
- It gates tracked IDs with an `active` flag:
  - inactive (`edit`) => no tracked IDs sent
  - active (`play`) => deduplicated tracked IDs are subscribed
- Tracked IDs are derived from graph node bindings (`getTrackedTagIds`).
- Live values are projected into node `data.liveValue` / `data.liveValues` via `applyLiveValuesToGraphNodes`.

## Namespace Tree Subsystem

Main files: `src/lib/features/tree/*`

- Data adapter:
  - `fetchTreeChildren()` maps backend `ListResponse` to `TreeNode`.
- Store:
  - `createTreeStore()` owns nodes map, root IDs, expansion, loading/error flags.
  - Lazy-loads children on expand.
  - Produces flattened visible rows (`flattenVisibleRows`) for virtualization.
- Remote reconciliation:
  - Applies pushed `TreeChanged` events incrementally when possible.
  - Falls back to targeted reload when backend marks `reload=true`.
- UI (`VariableTree.svelte`):
  - virtualized rendering,
  - keyboard navigation,
  - add/edit dialogs with zod + superforms,
  - tag metadata tooltip,
  - multi-select mode with propagate-up/down behavior.
- Deletion batching:
  - multi-select delete computes minimal ancestor set to avoid redundant deletes.

## Graph Workspace and Asset Binding

Main files: `src/lib/features/graph/*`

- Canvas:
  - `SvelteFlow` hosts nodes/edges in page-local state (`graphNodes`, `graphEdges`).
  - No persistence layer is implemented; graph state is in-memory only.
- Creation flow:
  - Drag tree node to canvas area.
  - Open context menu with registered asset types.
  - Create a `plantAsset` node with initial binding if dropped item is a tag.
- Node inspection:
  - Right-side inspector edits title and manages binding assignments.
  - Supports single and multi-tag binding schemas (depends on widget manifest).
- Writes:
  - Widget write callbacks resolve bound tag ID and call `realtimeProvider.sendWriteValue`.

## Widget Plugin Architecture

Main files:
- `src/lib/scada/plugins/types.ts`
- `src/lib/scada/plugins/internal-widgets-plugin.ts`
- `src/lib/scada/plugins/internal-widgets/runtime/widget-runtime.ts`
- `src/lib/features/graph/assets/registry.ts`

Behavior:

- Assets are registered from an internal plugin at module load.
- Each widget contributes:
  - manifest (`type`, `displayName`, `bindings`, `primaryBindingKey`)
  - runtime (`custom-element` currently used by internal widgets)
- Custom-element runtime:
  - creates shadow DOM shell,
  - renders template + styles,
  - wires event handlers and debounced writes via a bind context.
- Built-in widgets currently registered:
  - `tank`, `pump`, `valve`, `fan`, `slider`, `typed_input`, `onoff`, `light`, `label`.

## Namespace Template Builder (Bulk Namespace)

Main files: `src/lib/features/namespace-builder/*`

- Modal tool with two synchronized modes:
  - visual tree editor (virtualized + drag/drop nesting)
  - YAML editor (Monaco, lazy-loaded)
- YAML pipeline:
  - parse/validate (`parseYamlLike`, `validateNamespaceAst`)
  - serialize (`serializeYamlLike`)
  - convert to backend payload (`astToNamespaceJson`)
- Supports:
  - typed leaves (`Float`, `Integer`, `Text`, `Boolean`)
  - optional metadata (`unit`, `min`, `max`, `maxLength`, `options`)
  - generated naming series (`[start:end:step]` and `[A,B,C]`)
- On create:
  - page calls `tagStreamClient.addBulkNamespace(parentId, json, endpoint)`
  - parent is `"/"` for root bulk create or folder ID for scoped create.

## Validation, Sanitization, and UX Feedback

- Input sanitization helpers in `src/lib/forms/sanitize.ts`.
- Tree add/edit schemas in `src/lib/forms/tree-schemas.ts`.
- Errors and operation feedback are surfaced through `snackbarStore`.

## Tests

Unit tests are concentrated around domain logic and transport:

- WebSocket command/client behavior
- realtime provider gating
- tree flattening/store reconcile logic
- namespace builder parser/helpers
- theme utility behavior

Test runner is Vitest (`npm run test`, `npm run test:coverage`).
