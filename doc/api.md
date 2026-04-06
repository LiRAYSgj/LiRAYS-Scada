# API and WebSocket

## HTTP
- Base: `http(s)://<host>:<BIND_PORT>`
- Swagger UI: `/swagger` (generated OpenAPI).
- Auth endpoints (when `AUTH_ENABLED=true`):
  - `GET /auth/setup` (first admin password)
  - `POST /auth/login` (form) / `POST /auth/token` (JSON) → access & refresh tokens
  - `POST /auth/refresh` (with refresh token)
  - `GET /auth/logout`
- CRUD examples for views:
```sh
curl -X POST http://localhost:8245/api/views \
  -H "Content-Type: application/json" \
  -d '{"name":"Main View","description":"Primary dashboard","canvas_json":"{}"}'

curl http://localhost:8245/api/views
curl http://localhost:8245/api/views/<view-id>
curl -X PUT http://localhost:8245/api/views/<view-id> \
  -H "Content-Type: application/json" \
  -d '{"name":"Updated View","description":"Updated description","canvas_json":"{}"}'
curl -X DELETE http://localhost:8245/api/views/<view-id>
curl http://localhost:8245/api/views/entry-point
curl -X POST http://localhost:8245/api/views/<view-id>/entry-point
```

### Authenticated requests
When auth is enabled, include the bearer token (or rely on the HttpOnly cookie issued by the login form):
```sh
TOKEN=$(curl -s -X POST http://localhost:8245/auth/token \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"secret"}' | jq -r .data.token)
curl -H "Authorization: Bearer $TOKEN" http://localhost:8245/api/views
```

## WebSocket
- Endpoint: `ws(s)://<host>:<BIND_PORT>/ws`
- Protocol: binary protobuf (see `proto/`).
- Supported commands: `Add`, `AddBulk`, `List`, `Set`, `Get`, `Del`, `Sub`, `Unsub`, `EditMeta`.
- Responses correlate by `cmd_id` (see Rust client in `clients/rust-client`).
- Rust client demos: `cargo run --manifest-path clients/rust-client/Cargo.toml --bin demo <basic|tree_stress|data_stress|bulk_test>`.

## Variable metadata & constraints
- Each variable (`ItemMeta/VarInfo`) may have: `unit`, `min`, `max`, `options[]`, `max_len`.
- Backend validation:
  - numeric out of range → rejected
  - text outside options or length → rejected
- WS command to edit metadata: `EditMetaCommand { var_id, unit?, min?, max?, options[], max_len[] }` (no rename/type change).
- UI: right-click variable → “Edit”; tree refreshes to show updated metadata.

## Subscriptions and realtime
- `SUB` / `UNSUB` by tag IDs; backend pushes value and tree-change events.
- Client is backpressure-tolerant: slow clients drop batches.

## Useful endpoints/UI
- SPA: `/`
- Swagger: `/swagger`
- WebSocket: `/ws`
