# LiRAYS‑SCADA Architecture

## Runtime Topology
```
Browser (Svelte SPA)
    ↕  WebSocket (8245) – protobuf or JSON commands/events
    ↕  HTTPS/HTTP  (8246) – static SPA + REST resources
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

## Frontend Notes
- Svelte app connects to `ws(s)://<host>:8245` picked from the page scheme (uses `wss` when page is `https`).
- Built assets live in `frontend/build` and are embedded at compile time.

## Security/TLS Flow
- When `WS_TLS_ENABLE` is true:
  - Uses `WS_TLS_CERT_PATH`/`WS_TLS_KEY_PATH` if provided, otherwise generates a self-signed pair under `${DATA_DIR}/certificates/`.
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
2) Sends protobuf/JSON commands (LIST/GET/SET/ADD/DEL/…); backend mutates sled.  
3) Backend publishes events over the same socket to subscribed clients.  
4) HTTP API provides CRUD over static resources (SeaORM) and serves the SPA + OpenAPI doc routes.
