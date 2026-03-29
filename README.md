# LiRAYS‑SCADA

Rust WebSocket/HTTP server with an embedded Svelte frontend for browsing and controlling a SCADA‑style variable tree.

## Components

- Rust backend (WebSocket on 8245, HTTP/HTTPS on 8246).
- Embedded SQLite (via SeaORM) for static resources and sled for runtime data.
- Svelte frontend bundled into the binary; optional standalone frontend dev workflow.

## Quick Start (local)

1. Build the frontend (needed only if you change UI code):
   ```sh
   cd frontend
   npm install
   npm run generate:proto
   npm run build
   cd ..
   ```
2. Run the server:
   ```sh
   cargo run --bin lirays-scada
   ```
   Visit `http://localhost:8246` (or `https://localhost:8246` when TLS is on).
   API docs: `http(s)://localhost:8246/swagger` (serves Swagger UI backed by generated OpenAPI).

## Configuration (env vars)

- `BIND_HOST` / `BIND_SERVER_PORT` – WebSocket bind (default `0.0.0.0:8245`).
- `BIND_HTTP_HOST` / `BIND_HTTP_PORT` – HTTP bind (default `0.0.0.0:8246`).
- `DATA_DIR` – data root (default `./data_dir`); contains `rt_data/` (sled) and `static.db` (SQLite).
- TLS:
  - `WS_TLS_ENABLE` – when true (`1/true/yes/on`), both WS and HTTP serve over TLS.
  - `WS_TLS_AUTO` – when true, auto‑generate a self‑signed cert/key under `${DATA_DIR}/certificates/` and use them (ignores provided paths).
  - `WS_TLS_CERT_PATH` / `WS_TLS_KEY_PATH` – PEM paths to use when TLS is on and auto is off.
- Auth (optional):
  - `AUTH_ENABLED` – when true, all HTTP (SPA, Swagger, API) is gated by a session cookie.
  - `AUTH_SECRET` – optional HMAC secret for signing the session cookie; if missing a random one is generated at startup (sessions invalidate on restart).
  - First visit with auth on: redirected to `/auth/setup` to set the `admin` password (user is created then). Subsequent visits: `/auth/login`.
  - Session: HttpOnly cookie `lirays_session`, 24h TTL, `Secure` when TLS is enabled.

## Variable metadata & constraints

- Variables (ItemMeta/VarInfo) carry optional `unit`, numeric `min`/`max`, text `options`, and text `max_len`.
- Backend rejects writes outside constraints (numeric range, text length/options).
- WS command to edit metadata: `EditMetaCommand { var_id, unit?, min?, max?, options[], max_len[] }` (no rename/type change).
- UI: right‑click a variable → “Edit” opens dialog to update metadata; tree shows Type → Value → Unit columns; hover shows tooltip with constraints.

## Docker

```sh
docker build --target production -t lirays:latest .
docker run --rm \
  -p 8245:8245 -p 8246:8246 \
  -v $(pwd)/data_dir:/data \
  --name lirays-scada lirays:latest
```

## API / WebSocket usage

- WebSocket endpoint: `ws://<host>:8245` or `wss://<host>:8245` (matches TLS setting).
- HTTP API examples (default ports):

  ```sh
  curl -X POST http://localhost:8246/api/resources \
    -H "Content-Type: application/json" \
    -d '{"name":"Example Resource","description":"This is an example"}'

  curl http://localhost:8246/api/resources
  curl http://localhost:8246/api/resources/1
  curl -X PUT http://localhost:8246/api/resources/1 \
    -H "Content-Type: application/json" \
    -d '{"name":"Updated Resource","description":"Updated description"}'
  curl -X DELETE http://localhost:8246/api/resources/1
  ```

- WebSocket command shapes (protobuf/JSON) remain as in `proto/` and `src/rtdata/server`.
- Metadata edit command response: resolves to `EditMetaResponse`; frontend refreshes the parent folder to display new metadata.
- Observability (optional):
  - `METRICS_DIR` – optional directory; when set, metrics are emitted every 5s. A live snapshot with ANSI colors is written to `metrics_rt.txt` (overwritten each interval) and a history is appended to `metrics_hist.csv` with timestamps. If empty/unset, metrics collection stays off.
  - Live view of the realtime file with colors:
    - macOS / shells without `watch`:
      ```bash
      while true; do clear; cat "$METRICS_DIR/metrics_rt.txt"; sleep 1; done
      ```
    - Linux with `watch` installed:
      ```bash
      watch -n1 cat "$METRICS_DIR/metrics_rt.txt"
      ```
    - Windows (PowerShell):
      ```powershell
      while ($true) { cls; Get-Content "$env:METRICS_DIR/metrics_rt.txt"; Start-Sleep -Seconds 1 }
      ```

## Development tips

- Node 24 recommended for frontend tasks (`nvm use 24`).
- `cargo check` / `cargo test` for backend.
- The frontend chooses `wss` automatically when the page is loaded over `https` to avoid mixed content.

## Diagrams

- General schema: `general_schema.png`.
