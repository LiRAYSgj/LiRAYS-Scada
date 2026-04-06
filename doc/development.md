# Developer Guide

## Prerequisites
- Node 24 (use nvm).
- Rust stable 1.74+ (packaging uses 1.94).
- `protobuf` compiler available on PATH.

## Repo layout (high level)
- `src/` — Rust server (axum, sled, SQLite/SeaORM, TLS, auth, metrics).
- `frontend/` — SvelteKit SPA; assets embedded via `include_dir`.
- `proto/` — protobuf definitions used by backend and frontend.
- `packaging/debian` and `packaging/rpm` — systemd units, default settings, specs.
- `distributions/` — build outputs (.deb/.rpm).
- `clients/rust-client` — demos and reference client.

## Local dev (hot iteration)
```sh
# Backend only
cargo run --bin lirays-scada
# Frontend dev server (with live reload)
cd frontend && npm install && npm run dev
```

## Full rebuild (release-like)
```sh
(cd frontend && npm install && npm run generate:proto && npm run build)
cargo build --release
```

## Tests
```sh
(cd frontend && npm run test)
cargo test
```

## Packaging (via Makefile)
Requirements on the host:
- Docker with multi-arch support (Docker Desktop or dockerd + binfmt/qemu) — Makefile runs builds inside containers.
- `make`, `bash`, internet access for base images.
- Optional: `nvm` for local frontend build if you bypass Docker.

Commands:
- Debian/Ubuntu packages (.deb): `make deb` (builds amd64 + arm64) or `make deb-amd64` / `make deb-arm64` for a single arch.
- RHEL/Rocky/Alma/Fedora packages (.rpm): `make rpm` (x86_64 + aarch64) or `make rpm-amd64` / `make rpm-arm64`.

What happens:
- Frontend is built once per run (`frontend/.frontend-built` stamp).
- Backend is compiled in containerized Rust toolchains (1.94) per arch.
- Artifacts are placed in `distributions/`.
- Systemd units and default configs are injected from `packaging/debian/deb-files` and `packaging/rpm/SOURCES`.

## Release checklist (manual)
1) Bump `version` in `Cargo.toml`.
2) Tag release `vX.Y.Z` and build packages: `make deb rpm`.
3) Upload `.deb`/`.rpm` from `distributions/` to GitHub Releases with matching tag.
4) Ensure README download links point to the new tag/filenames.

## Clients and demos
- Rust client: `clients/rust-client`.
- Demos: `cargo run --manifest-path clients/rust-client/Cargo.toml --bin demo <basic|tree_stress|data_stress|bulk_test>`.

## Frontend stack (quick notes)
- SvelteKit (adapter-static), Svelte 5 runes, TS, Tailwind v4, `@xyflow/svelte`.
- Variable tree + SvelteFlow canvas; Template Builder supports YAML → `ADD_BULK`.
