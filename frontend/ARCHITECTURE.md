# Frontend Architecture

This document describes the **current implementation** in `frontend/`.

## Overview

- Framework: SvelteKit + Svelte 5 runes + TypeScript
- Adapter/build: `@sveltejs/adapter-static` with SPA fallback (`index.html`)
- Main route: `src/routes/+page.svelte`
- App shell: `src/routes/+layout.svelte`
- Styling: Tailwind CSS v4 + CSS variables in `src/routes/layout.css`
- Graph engine: `@xyflow/svelte`
- Transport: single shared WebSocket client (`TagStreamClient`)

## Runtime Model

The page composes three coordinated subsystems:

1. Tree subsystem
- Browses namespace with `LIST`
- Supports Add / Remove / Namespace Template Builder
- Supports multi-selection with descendant/ancestor propagation and minimal-delete set

2. Graph subsystem
- Accepts dropped tree nodes and creates plant assets
- Displays live values from realtime store
- Sends writes with `SET` from interactive assets

3. Realtime subsystem
- Tracks graph tag IDs (when Play mode is enabled)
- Uses WS subscriptions (`SUB`/`UNSUB`) for value push events
- Publishes normalized values to Svelte stores

## Source Layout

```text
src/
  routes/
    +layout.svelte
    +page.svelte
    layout.css
  lib/
    components/
      Button/
      Snackbar/
    stores/
      theme.ts
      snackbar.ts
    core/
      theme/theme-utils.ts
      ws/
        types.ts
        command-ws-client.ts
        tag-stream-client.ts
    features/
      realtime/page-tag-realtime-provider.ts
      tree/
        types.ts
        flatten.ts
        tree-store.ts
        tree-selection.ts
        tree-remote-reconcile.ts
        server-adapter.ts
        context-menu.ts
        components/
          VariableTree.svelte
          ContextMenu.svelte
          TreeRow.svelte
          TreeChevron.svelte
          TreeIcon.svelte
      graph/
        live-utils.ts
        components/PlantAssetNode.svelte
        assets/
          registry.ts
          types.ts
          controller.ts
          components/*.svelte
      namespace-builder/
        namespace-yaml.ts
        monaco-yaml-config.ts
        types.ts
        components/*.svelte
      workspace/components/PageToolbar.svelte
```

## WebSocket Architecture

### Protocol and Commands

Commands are protobuf envelopes created in `src/lib/core/ws/command-ws-client.ts`.

Currently used by the UI:
- `LIST` for tree child fetch
- `ADD` for node creation
- `ADD_BULK` for namespace template creation
- `DEL` for node deletion
- `SET` for writing values
- `SUB`/`UNSUB` for realtime value streaming and tree-change subscription

Notes:
- The command builder also includes `GET`, but current realtime flow is subscription/event based.
- Command timeout in `TagStreamClient` is `3600_000` ms.

### Shared Client (`TagStreamClient`)

`src/lib/core/ws/tag-stream-client.ts` manages:
- single socket instance
- reconnect/backoff
- pending command correlation by `cmd_id`
- global tree-change subscription after connect
- tracked tag subscriptions/unsubscriptions
- command timeout/error handling via snackbar
- incoming value/tree event decoding and store updates

`src/lib/features/realtime/page-tag-realtime-provider.ts` wraps the client and adds:
- active/inactive gating from page mode
- deduplicated desired ID set
- route-level `start/stop` lifecycle helpers

### Endpoint Resolution

Current code uses two endpoint entry points:
- Page realtime provider: `PUBLIC_DEMO_WS_ENDPOINT` fallback `ws://127.0.0.1:8245`
- Tree server adapter: inferred from page scheme/host (`ws` or `wss` + `location.hostname:8245`)

## Tree Subsystem

### Data and State

- Adapter: `src/lib/features/tree/server-adapter.ts`
  - fetches children with `tagStreamClient.listChildren(parent?.id)`
- Store: `src/lib/features/tree/tree-store.ts`
  - normalized node cache (`nodes`, `rootIds`)
  - expand/collapse and lazy child loading
  - flattened visible rows via `flatten.ts`
  - targeted refresh (`refreshNode`)
  - remote tree reconcile via `tree-remote-reconcile.ts`
- Selection helpers: `tree-selection.ts`
  - descendant propagation
  - partial/indeterminate subtree detection
  - minimal ancestor delete set

### UI Behavior

`VariableTree.svelte` provides:
- virtualized fixed-row rendering window
- keyboard navigation
- context menu + drag support in normal mode
- multi-select mode with checkbox interactions
- add dialog for root/folder creation

## Graph Subsystem

### Node Model and Rendering

- Graph nodes are `plantAsset` nodes rendered by `PlantAssetNode.svelte`
- Asset type resolution comes from `assets/registry.ts`
- Shared asset data contract is in `assets/types.ts`

### Implemented Assets

- Tank
- Pump
- Valve
- Fan
- Slider
- Typed Input
- On/Off Input
- Light Indicator
- Label

### Live and Write Flow

- Tracked tag IDs are derived from graph nodes (`live-utils.ts`)
- Realtime values are applied to node data (`applyLiveValuesToGraphNodes`)
- Interactive assets call page-provided `onWriteValue` -> `SET`

## Namespace Template Builder

`src/lib/features/namespace-builder/` provides:
- Monaco-backed YAML/code mode
- visual tree mode
- YAML parse/serialize and validation
- create flow mapped to WS `ADD_BULK`

Open targets:
- toolbar: root (`""`, title `/`)
- folder context menu: selected folder id/path

## Page Orchestration (`+page.svelte`)

The route coordinates the subsystems:
- starts/stops realtime provider on mount/unmount
- toggles realtime active state by canvas mode (`edit`/`play`)
- maps graph tag IDs -> realtime desired IDs
- applies live values to graph nodes
- wires tree actions (add/remove/bulk create)
- wires drag/drop to graph asset creation
- handles graph selection delete (Delete key in edit mode)
- handles tree multi-selection remove confirmation flow

Canvas interaction rules in current `SvelteFlow` usage:
- edit-only select/connect/drag interactions
- `selectionOnDrag` enabled in edit mode
- middle-mouse pan (`panOnDrag={[1]}`)
- double-click zoom disabled

## Theme and Global UI State

- Theme store: `src/lib/stores/theme.ts`
  - persisted key: `app-theme`
  - initialized in `+layout.svelte` on mount
  - no theme applied until initialized (`themeStore` starts as `null`)
- CSS variable generator: `src/lib/core/theme/theme-utils.ts`
- Global snackbar:
  - store: `src/lib/stores/snackbar.ts`
  - renderer: `src/lib/components/Snackbar/Snackbar.svelte`

## Testing

Vitest unit coverage includes:
- WS client + command builders
- realtime provider
- tree store/flatten/selection/adapter
- graph live utilities
- theme utilities
- namespace builder parse/Monaco helpers/tree helpers

Run:
- `npm run test`
- `npm run test:coverage`

## Current Constraints

- Realtime values are push/subscription based over WS events (not interval polling in page runtime).
- Tree and realtime intentionally share one WS transport.
- Frontend can run standalone (`npm run dev`) or integrated with backend runtime.
