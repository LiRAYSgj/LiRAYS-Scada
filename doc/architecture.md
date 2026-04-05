# LiRAYS‑SCADA Architecture

## Runtime Topology
```
Browser (SvelteKit static SPA: Svelte 5 + TypeScript)
    ↕  WebSocket (same port as HTTP, default 8245) – binary protobuf command/event stream
    ↕  HTTPS/HTTP  (default 8245) – static UI assets + REST resources
Rust runtime
    ├─ unified axum server (HTTP + WS upgrade)
    ├─ sled        (runtime RT data)
    └─ SQLite via SeaORM (static resources + auth users)
```

## Processes & Ports
- Single Rust binary (`lirays-scada`).
- HTTP/S + WebSocket (same listener): `BIND_HOST`/`BIND_PORT` (default `0.0.0.0:8245`).
- TLS: `TLS_ENABLE` drives both WebSocket and HTTP listeners (shared cert/key or auto self-signed).
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
- Stack: SvelteKit (adapter-static, SPA fallback), Svelte 5 runes, TypeScript, Tailwind CSS v4, `@xyflow/svelte`.
- UI: variable tree on the left, SvelteFlow canvas on the right with plant nodes; Template Builder modal (YAML → `ADD_BULK`).
- Transport: one `TagStreamClient` handles socket lifecycle, reconnect/backoff, command correlation (`cmd_id`), and global snackbar.
- Realtime: `SUB/UNSUB` by tag IDs on canvas; push events for values and tree changes.
- WS endpoint: `/ws` on the same host/port; auto-switches to `wss` if the page is HTTPS.
- Theme: light/dark persisted in `localStorage`.
- Assets built into `frontend/build` and embedded in the binary.

## Security/TLS Flow
- With `TLS_ENABLE=true`: uses provided cert/key or generates self-signed under `${DATA_DIR}/certificates/` when `TLS_AUTO=1`.
- HTTP serves HTTPS on `BIND_PORT`; WebSocket upgrades to `wss` on the same port.
- With `AUTH_ENABLED=true`: first visit forces `/auth/setup` to set `admin`; then `/auth/login`. HttpOnly cookie `lirays_session`, 24h, `Secure` if TLS.

## Build & Packaging (overview)
- Frontend: `npm install && npm run build` (Node 24 recommended).
- Backend: `cargo build` (proto via `build.rs`).
- Distribution: `.deb` packages, Docker image, macOS binary/DMG.

## Data Flow (happy path)
1) Client opens WS (`ws/wss`).  
2) UI sends protobuf commands (`LIST`, `ADD`, `ADD_BULK`, `DEL`, `SET`, `SUB`, `UNSUB`) over the same socket.  
3) Backend mutates data and pushes value/tree change events.  
4) HTTP API offers CRUD over static resources and serves SPA + OpenAPI.

## Diagrams
- General schema: `general_schema.png`.
