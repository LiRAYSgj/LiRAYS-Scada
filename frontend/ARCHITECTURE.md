# LiRAYS SCADA Architecture (Frontend + Rust Backend)

This document reflects the current implementation in `frontend/` and `src/`.

## 1) Runtime Topology

```text
Browser (SvelteKit static SPA)
  ├─ HTTP(S): static assets + auth + views REST
  └─ WebSocket: protobuf command/response + push events

Rust binary (`lirays-scada`)
  ├─ Axum unified server (HTTP + WS on same port)
  ├─ RT namespace/value engine (sled)
  └─ Views/Auth persistence (SQLite via SeaORM)
```

- Single listener (`BIND_HOST`/`BIND_PORT`, default `0.0.0.0:8245`) serves both HTTP and WS.
- Optional TLS (`WS_TLS_ENABLE`) applies to HTTP and WebSocket upgrades together.

## 2) Frontend Architecture (SvelteKit + Svelte 5)

### 2.1 Build and rendering model

- `frontend/src/routes/+layout.ts`:
  - `ssr = false`
  - `prerender = true`
- `frontend/svelte.config.js` uses `@sveltejs/adapter-static` outputting to `frontend/build`.
- Rust serves that built output from the embedded `include_dir` bundle.

### 2.2 Route structure

- Root shell:
  - `src/routes/+layout.svelte` handles:
    - global CSS (`layout.css`)
    - theme init (`initThemeFromStorage`)
    - snackbar host
    - guarded navigation preload/progress UI
- App-protected group:
  - `src/routes/(app)/+layout.ts` checks `/auth/status` and redirects to `/auth/login` or `/auth/setup` when auth is enabled.
  - `src/routes/(app)/+layout.svelte` contains the full workspace UI.
  - `src/routes/(app)/+page.ts` redirects to `/views`.
  - `src/routes/(app)/views/[id]/+page.ts` uses `prerender = "auto"`.
- Guest group:
  - `src/routes/(guest)/+layout.ts` redirects authenticated users away from guest pages.
  - `/auth/login` and `/auth/setup` render form components.
- Dev bridge routes:
  - `src/routes/auth/status/+server.ts`
  - `src/routes/auth/logout/+server.ts`
  - These forward to backend via `src/lib/server/auth-proxy.ts`.

### 2.3 Main workspace composition

`src/routes/(app)/+layout.svelte` hosts three major areas:

- Left: namespace browser (`VariableTree`)
- Right (mode-dependent):
  - views table (`ViewsListPanel`), or
  - view editor (`ViewEditorHeader` + `SvelteFlow` canvas + inspector)
- Global dialogs:
  - remove single/multiple tree nodes
  - remove view
  - namespace template builder (`NamespaceBuilder`)

Notable state boundaries:

- `rightPaneMode`: derived from route (`/views` list vs `/views/:id` editor)
- `canvasMode`: `edit | play` (controls realtime activation)
- `workspaceMode`: toolbar toggle only (currently cosmetic)

### 2.4 View persistence model

- REST API client: `src/lib/features/views/api/views-api.ts`
- Backend stores per-view `canvas_json` in SQLite.
- Canvas serialization/deserialization:
  - `serializeCanvasState()` strips function properties from nodes before save.
  - `deserializeCanvasState()` provides safe fallbacks.

### 2.5 Namespace tree subsystem

Core files:

- `src/lib/features/tree/tree-store.ts`
- `src/lib/features/tree/server-adapter.ts`
- `src/lib/features/tree/tree-remote-reconcile.ts`
- `src/lib/features/tree/components/VariableTree.svelte`

Behavior:

- Lazy loading of folder children via WS `LIST`.
- Local store tracks nodes, expansion, loading/error, selected node.
- Remote incremental reconciliation from pushed `TreeChanged` events.
- Falls back to targeted reload when backend marks `reload=true`.
- Supports single-select and multi-select delete optimization (`getMinimalAncestorSet`).

### 2.6 Realtime and WS client model

Core files:

- `src/lib/core/ws/tag-stream-client.ts`
- `src/lib/features/realtime/page-tag-realtime-provider.ts`
- `src/lib/core/ws/command-ws-client.ts`

Behavior:

- One singleton `TagStreamClient` owns socket lifecycle, reconnect backoff, pending command correlation by `cmd_id`.
- Binary protobuf is default; client can decode both `Response` envelopes and `Event` pushes.
- Global subscription on connect:
  - `SUB` to `EVENT_TYPE_TREE_CHANGE`
- Per-tag subscriptions:
  - `SUB/UNSUB` to `EVENT_TYPE_VAR_VALUES` based on tracked graph bindings
- Page-level gating:
  - `createPageTagRealtimeProvider` sends tracked IDs only when active
  - active only in view editor + play mode

### 2.7 Graph/widget plugin architecture

Core files:

- `src/lib/features/graph/assets/registry.ts`
- `src/lib/features/graph/components/PlantAssetNode.svelte`
- `src/lib/features/graph/components/CustomElementAssetHost.svelte`
- `src/lib/scada/plugins/internal-widgets/*`

Behavior:

- Asset definitions are registered from internal plugin `lirays.internal.widgets` at module load.
- Runtime currently uses custom elements for internal widgets.
- Widget shell/runtime provides:
  - shadow DOM rendering
  - per-binding value reads
  - debounced write callbacks to backend `SET`
- Built-ins: `tank`, `pump`, `valve`, `fan`, `slider`, `typed_input`, `onoff`, `light`, `label`.

### 2.8 Namespace template builder

Core files:

- `src/lib/features/namespace-builder/components/NamespaceBuilder.svelte`
- `src/lib/features/namespace-builder/namespace-yaml.ts`

Behavior:

- Dual-mode editor:
  - visual tree editor
  - YAML-like code editor (Monaco)
- Converts YAML AST to backend `NamespaceSchema` payload.
- Supports repeated-name expansion (`[start:end:step]`, `[A,B,C]`) and typed metadata.
- Submit path calls WS `ADD_BULK`.

### 2.9 Theme and global UX

- Theme store: `src/lib/stores/theme.ts` (`light|dark`, persisted in `localStorage`).
- Global tokens/styles in `src/routes/layout.css` (Tailwind v4 + custom variables).
- Snackbar store centralizes operation feedback and transport errors.

## 3) Rust Backend Architecture

### 3.1 Entry point and server bootstrap

- `src/main.rs`
  - resolves env config (host/port/data dir/TLS/auth)
  - initializes optional self-signed cert when TLS enabled and paths absent
  - starts unified HTTP server via `run_http_server(...)`

### 3.2 HTTP layer (Axum)

Core file: `src/http/mod.rs`

`run_http_server(...)` responsibilities:

- Open SQLite DB (`static.db`) and run migrations (`src/migration.rs`, users table).
- Initialize `VariableManager` (sled-backed RT store).
- Start periodic flush loop for dirty variable values.
- Initialize `ViewManager` and ensure default entry-point view exists.
- Build `AppState` with managers + auth config.
- Register routes:
  - Views API: `/api/views*`
  - WS endpoint: `/ws`
  - Auth: `/auth/*`
  - OpenAPI JSON + Swagger
  - fallback static serving
- Apply auth middleware globally.

### 3.3 Auth model

Core files:

- `src/http/mod.rs` (middleware + token helpers)
- `src/http/resources/auth/service.rs`
- `src/http/resources/user/service.rs`

Behavior:

- Auth is optional (`AUTH_ENABLED`).
- Session format: HMAC-signed claims (`access`/`refresh`), not JWT.
- Cookie names:
  - `lirays_session` (access)
  - `lirays_refresh` (refresh)
- Access checks allow token from:
  - `Authorization: Bearer ...`
  - `?token=...`
  - session cookie
- Middleware rules:
  - when unauthenticated, `/api` and `/ws` return `401`
  - regular app paths redirect to `/auth/login`
  - if no admin user, force `/auth/setup`

### 3.4 Static frontend serving

Core file: `src/http/resources/static_files/service.rs`

- Serves files from embedded `frontend/build` (`include_dir!`).
- Path resolution strategy:
  - exact file
  - `path/index.html`
  - `path.html`
  - fallback `index.html` for SPA routing

### 3.5 Views domain (REST + SQLite)

Core file: `src/http/resources/views/service.rs`

- CRUD + pagination/sorting/search for views.
- Exactly one `is_entry_point = true` is maintained transactionally.
- Prevents deleting the last remaining view.
- Stores `canvas_json` as JSON string.

### 3.6 Realtime namespace/value engine (WS + sled)

Core files:

- `src/http/resources/ws/service.rs`
- `src/rtdata/variable.rs`
- `src/rtdata/events.rs`
- `src/rtdata/mod.rs`

Behavior:

- Each WS session keeps:
  - subscribed tag-id set
  - tree-change subscription flag
- Command execution delegated to `VariableManager::exec_cmd(...)`.
- Supports protobuf command set:
  - `ADD`, `LIST`, `SET`, `GET`, `DEL`, `ADD_BULK`, `SUB`, `UNSUB`, `EDIT_META`
- `should_send(...)` filters push events per session subscriptions.
- Values are written to in-memory cache first, then coalesced flush to sled.
- Namespace metadata/items and values are stored in separate sled trees.

### 3.7 Storage boundaries

- `DATA_DIR/rt_data` (sled): namespace structure + live values.
- `DATA_DIR/static.db` (SQLite):
  - users
  - views

## 4) Frontend ↔ Backend Contracts

### 4.1 WebSocket protobuf

- Proto sources under `proto/namespace/*.proto`.
- Rust types generated by `build.rs` (prost).
- Frontend TS generated into `frontend/src/lib/proto` via `npm run generate:proto`.

Key envelopes:

- `Command` / `Response` for request-response operations
- `Event` for push updates:
  - `var_value_ev`
  - `tree_changed_ev`

### 4.2 REST endpoints used by frontend

- `GET /auth/status`
- `POST /auth/login`
- `POST /auth/setup`
- `GET /auth/logout`
- `GET /api/views`
- `POST /api/views`
- `GET /api/views/{id}`
- `PUT /api/views/{id}`
- `DELETE /api/views/{id}`
- `GET /api/views/entry-point`
- `PUT /api/views/{id}/entry-point`

### 4.3 Dev-time networking

In `frontend/vite.config.ts`:

- Proxy `/api` -> `http://127.0.0.1:8245`
- Proxy `/ws` -> `ws://127.0.0.1:8245`
- Middleware forwards POST `/auth/login` and `/auth/setup` to backend.

## 5) End-to-End Flow Summary

1. Browser loads static SPA from Rust fallback/static route.
2. App route guards call `/auth/status` and redirect if needed.
3. Workspace opens `/ws` and subscribes to tree changes.
4. Tree CRUD and metadata operations go through WS commands.
5. View list/editor operations persist canvas state through `/api/views`.
6. In play mode, graph bindings determine subscribed tag IDs; pushed values update widget live state.
