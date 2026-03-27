# LiRAYS‑SCADA Architecture

## Runtime Topology
```
Browser (SvelteKit static SPA: Svelte 5 + TypeScript)
    ↕  WebSocket (8245) – binary protobuf command/event stream
    ↕  HTTPS/HTTP  (8246) – static UI assets + REST resources
Rust runtime
    ├─ ws server   (tokio-tungstenite)
    ├─ http server (axum)
    ├─ sled        (runtime RT data)
    └─ SQLite via SeaORM (static resources + auth users)
```

## Processes & Ports
- Single Rust binary (`lirays-scada`).
- WebSocket: `BIND_HOST`/`BIND_SERVER_PORT` (default `0.0.0.0:8245`).
- HTTP/S: `BIND_HTTP_HOST`/`BIND_HTTP_PORT` (default `0.0.0.0:8246`).
- TLS: `WS_TLS_ENABLE` drives both WebSocket and HTTP listeners (shared cert/key or auto self-signed).
- Auth (optional): `AUTH_ENABLED` gates HTTP (SPA, Swagger, API) behind a session cookie; `AUTH_SECRET` signs the cookie.

## Data & Storage
- **sled** under `${DATA_DIR}/rt_data/` for variable tree and live values.
- **SQLite** file `${DATA_DIR}/static.db` for static resources served by the HTTP API (managed with SeaORM).
- Frontend assets compiled into the binary via `include_dir` (rebuilt when `frontend/build` changes).

## Backend Modules
- `src/rtdata/server`: WebSocket command handling, event broadcast, variable management, TLS acceptor builder.
- `src/rtdata/http`: Axum router for REST endpoints, OpenAPI/Swagger docs, and static SPA serving; optional HTTPS via shared TLS config.
- `src/http/model/resource`: SeaORM entity + service for static resources.
- `src/http/model/user`: SeaORM entity + service for `users` table (admin).
- `src/rtdata/namespace`: Protobuf-derived command/event types (see `proto/`).
- `src/rtdata/variable.rs`: sled-backed variable tree; supports constraints (min/max/options/max_len) and `EditMetaCommand` for metadata updates (name/type immutable).

## Frontend Notes
- Frontend stack: SvelteKit (adapter-static with SPA fallback), Svelte 5 runes, TypeScript, Tailwind CSS v4, and `@xyflow/svelte`.
- Route structure is app-shell style (`src/routes/+layout.svelte` + `src/routes/+page.svelte`) with a single operational SCADA screen.
- Main UI composition in `+page.svelte`:
  - left panel: namespace tree (`VariableTree`) with context menus, add/remove flows, and multi-selection delete workflow.
  - right panel: SvelteFlow graph canvas where tree nodes are dropped and mapped to plant assets (Tank, Pump, Valve, Fan, Slider, Typed Input, On/Off, Light, Label).
  - modal tooling: Namespace Template Builder (Monaco YAML -> `ADD_BULK`).
- Transport model:
  - one shared `TagStreamClient` handles socket lifecycle, reconnect/backoff, command correlation (`cmd_id`), and global snackbar errors.
  - realtime values are **subscription-based** (`SUB`/`UNSUB` for tracked graph tag IDs) and pushed as WS events.
  - tree synchronization across clients uses global tree-change event subscription and local reconcile/refresh in the tree store.
- WS endpoint behavior in current UI code:
  - page realtime provider uses `PUBLIC_DEMO_WS_ENDPOINT` (fallback `ws://127.0.0.1:8245`).
  - tree listing infers endpoint from page scheme/host (`ws`/`wss` + `location.hostname:8245`).
- Theme/UI state:
  - light/dark mode persisted in `localStorage` (`app-theme`) and applied after mount.
  - global snackbar store centralizes command timeout/error/success feedback.
- Built assets live in `frontend/build` and are embedded at compile time.

## Security/TLS Flow
- When `WS_TLS_ENABLE` is true:
  - If `WS_TLS_AUTO=1` a self-signed pair is generated under `${DATA_DIR}/certificates/`.
  - Otherwise uses `WS_TLS_CERT_PATH`/`WS_TLS_KEY_PATH`.
  - HTTP server serves HTTPS on `BIND_HTTP_PORT`; websocket upgrades to `wss` on `BIND_SERVER_PORT`.
- When false: plain HTTP + WS.
- When `AUTH_ENABLED` is true:
  - First access is redirected to `/auth/setup` to create the `admin` password if it does not exist.
  - Subsequent access requires `/auth/login`; on success a signed HttpOnly cookie (`lirays_session`, 24h) is issued (`Secure` when TLS is on).
  - Auth middleware protects SPA, Swagger, and API routes; only `/auth/*` is public.

## Build & Packaging
- Frontend: `npm install && npm run build` (Node 24 recommended).
- Backend: `cargo build` (proto generated via `build.rs`).
- Dockerfile ships the release binary and static assets; `make release` wraps Debian packaging targets.

## Data Flow (happy path)
1) Client opens WS connection (`ws`/`wss`).  
2) UI sends protobuf commands (`LIST`, `ADD`, `ADD_BULK`, `DEL`, `SET`, `SUB`, `UNSUB`) over the shared socket.  
3) Backend mutates runtime data and pushes value/tree change events over that same socket.  
4) HTTP API provides CRUD over static resources (SeaORM) and serves the SPA + OpenAPI doc routes.
