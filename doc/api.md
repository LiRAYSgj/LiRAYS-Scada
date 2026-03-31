# API and WebSocket

## HTTP
- Base: `http(s)://<host>:<BIND_PORT>`
- Swagger UI: `/swagger` (generated OpenAPI).
- CRUD examples for static resources:
```sh
curl -X POST http://localhost:8245/api/resources \
  -H "Content-Type: application/json" \
  -d '{"name":"Example Resource","description":"This is an example"}'

curl http://localhost:8245/api/resources
curl http://localhost:8245/api/resources/1
curl -X PUT http://localhost:8245/api/resources/1 \
  -H "Content-Type: application/json" \
  -d '{"name":"Updated Resource","description":"Updated description"}'
curl -X DELETE http://localhost:8245/api/resources/1
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
