# LiRAYS‑SCADA Architecture

## Runtime Topology
```
Browser (SvelteKit static SPA: Svelte 5 + TypeScript)
    ↕  WebSocket (same port as HTTP, default 8245) – protobuf commands/events
    ↕  HTTPS/HTTP (default 8245) – static UI assets + REST resources + Swagger
Systemd service: lirays-scada (runs single Rust binary)
Rust runtime
    ├─ unified axum server (HTTP + WS upgrade)
    ├─ sled (runtime RT data) under ${DATA_DIR}/rt_data
    └─ SQLite via SeaORM (static resources + auth users) at ${DATA_DIR}/static.db
```

## Processes & Ports
- Single Rust binary (`lirays-scada`) managed by systemd in packaged installs.
- HTTP/S + WebSocket (same listener): `BIND_HOST`/`BIND_PORT` (default `0.0.0.0:8245`).
- TLS: `TLS_ENABLE` covers both HTTP and WebSocket; uses supplied cert/key or auto-generates self-signed into `${DATA_DIR}/certificates` when `TLS_AUTO=true`.
- Auth (optional): `AUTH_ENABLED` gates SPA/Swagger/API; `AUTH_SECRET` signs the session cookie and refresh token.

## Data & Storage
- **sled** under `${DATA_DIR}/rt_data/` for variable tree and live values.
- **SQLite** `${DATA_DIR}/static.db` for static resources and users.
- Frontend assets compiled into the binary via `include_dir` (rebuilt when `frontend/build` changes).
- Metrics (optional): `${METRICS_DIR}/metrics_rt.txt` snapshot + `${METRICS_DIR}/metrics_hist.csv` history.

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
- With `TLS_ENABLE=true`: uses provided cert/key or generates self-signed under `${DATA_DIR}/certificates/` when `TLS_AUTO=true`.
- HTTP serves HTTPS on `BIND_PORT`; WebSocket upgrades to `wss` on the same port.
- With `AUTH_ENABLED=true`: first visit forces `/auth/setup` to set `admin`; then `/auth/login`. HttpOnly cookies `lirays_session` (access) and `lirays_refresh`; `Secure` flag when TLS is on.

## Build & Packaging (overview)
- Frontend: `npm install && npm run generate:proto && npm run build` (Node 24).
- Backend: `cargo build` (proto via `build.rs`).
- Distribution: `.deb` and `.rpm` packages for amd64/x86_64 and arm64/aarch64. Docker image build target available. macOS/Windows not supported yet.

## Data Flow (happy path)
1) Client opens WS (`ws/wss`).  
2) UI sends protobuf commands (`LIST`, `ADD`, `ADD_BULK`, `DEL`, `SET`, `SUB`, `UNSUB`) over the same socket.  
3) Backend mutates data and pushes value/tree change events.  
4) HTTP API offers CRUD over static resources and serves SPA + OpenAPI.

## Diagrams
- General schema: `general_schema.png`.
