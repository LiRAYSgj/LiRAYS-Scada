# Developer Guide

## Prerequisites
- Node 24 (nvm recommended).
- Stable Rust toolchain (1.74+; Dockerfile uses 1.94).
- `protobuf` CLI to generate messages (installed in Dockerfile and used by frontend build).

## Quick local flow
```sh
# Frontend + backend debug
cd frontend
npm install
npm run generate:proto
npm run build
cd ..
cargo run --bin lirays-scada
# http://localhost:8245
```

## Production build
```sh
# Frontend (Node 24)
(cd frontend && npm install && npm run generate:proto && npm run build)
# Backend
cargo build --release
```

## Dev server
- Frontend: `npm run dev`.
- Backend: `cargo run`.

## Tests
```sh
(cd frontend && npm run test)
cargo test
```

## Clients and demos
- Async Rust client: `clients/rust-client`.
- Demos: `cargo run --manifest-path clients/rust-client/Cargo.toml --bin demo <basic|tree_stress|data_stress|bulk_test>`.

## Frontend stack
- SvelteKit (adapter-static, SPA), Svelte 5 runes, TS, Tailwind CSS v4, `@xyflow/svelte`.
- Main UI: variable tree (left) + SvelteFlow canvas (right) with plant nodes.
- Template Builder: modal with YAML → `ADD_BULK`.

## Build/packaging notes
- Debian packages: `make` on Linux → `.deb` in `distributions/`.
- macOS: `make` on macOS → release binary and `.pkg` + `.dmg` in `distributions/`.
- Docker: `docker build --target production -t lirays:latest .`
