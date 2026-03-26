# LiRAYS‑SCADA

Rust WebSocket/HTTP server with an embedded Svelte frontend for browsing and controlling a SCADA‑style variable tree.

## Components
- Rust backend (WebSocket on 8245, HTTP/HTTPS on 8246).
- Embedded SQLite (SQLx) for static resources and sled for runtime data.
- Svelte frontend bundled into the binary; optional standalone frontend dev workflow.

## Quick Start (local)
1. Build the frontend (needed only if you change UI code):
   ```sh
   cd frontend
   npm install
   npm run build
   cd ..
   ```
2. Run the server:
   ```sh
   cargo run --bin lirays-scada
   ```
   Visit `http://localhost:8246` (or `https://localhost:8246` when TLS is on).

## Configuration (env vars)
- `BIND_HOST` / `BIND_SERVER_PORT` – WebSocket bind (default `0.0.0.0:8245`).
- `BIND_HTTP_HOST` / `BIND_HTTP_PORT` – HTTP bind (default `0.0.0.0:8246`).
- `DATA_DIR` – data root (default `./data_dir`); contains `rt_data/` and `static.db`.
- TLS:
  - `WS_TLS_ENABLE` – when true (`1/true/yes/on`), both WS and HTTP serve over TLS.
  - `WS_TLS_CERT_PATH` / `WS_TLS_KEY_PATH` – PEM paths. If omitted while TLS is enabled, a self‑signed pair is generated under `DATA_DIR/certificates/`.
  - Browser will prompt to trust the self‑signed cert.

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

## Development tips
- Node 24 recommended for frontend tasks (`nvm use 24`).
- `cargo check` / `cargo test` for backend.
- The frontend chooses `wss` automatically when the page is loaded over `https` to avoid mixed content.

## Diagrams
- General schema: `general_schema.png`.
