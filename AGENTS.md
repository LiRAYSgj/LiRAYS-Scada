# Agent Guide — LiRAYS-SCADA

## Scope

This file guides backend work. Another developer maintains the frontend, which
has its own instructions in `frontend/AGENTS.md` and architecture map in
`frontend/ARCHITECTURE.md`. Do not review or modify the frontend unless the task
explicitly requires it.

Also follow `/Users/alejandro/.codex/RTK.md` in this environment: prefix shell
commands with `rtk`; use `rtk proxy` when a command needs to run without filtering.

## Language

Use English for all future project work: code identifiers, comments, docstrings,
documentation, test descriptions, log and error messages, commit messages, pull
request titles and descriptions, and agent updates and responses. Follow this
convention even when a task is requested in Spanish, unless the user explicitly
requests another language. Preserve existing external contracts and identifiers
when translating would introduce a breaking change.

## Backend Architecture

LiRAYS-SCADA is a SCADA platform designed for hardware with limited resources.
The backend uses **Rust (edition 2024)**, **Tokio** for asynchronous execution,
and **Axum** for HTTP and WebSocket. The Cargo package is named `scada`.

| Location | Responsibility |
| --- | --- |
| `src/main.rs`, `src/app/mod.rs` | Entry point for the `lirays-scada` server, configuration, logging, and data directories. |
| `src/http/mod.rs`, `src/http/state.rs` | Storage and service initialization, shared state, route mounting, TLS, and graceful shutdown. |
| `src/http/resources/` | HTTP resources for authentication, views, WebSocket, documentation, and static files; also user and personal access token models and services. |
| `src/rtdata/variable.rs` | `VariableManager`: namespace tree, metadata, values, commands, cache, persistence, and event distribution. |
| `src/rtdata/{parser,utils,events,metrics}.rs` | Name parsing, namespace utilities, event construction, and metrics. |
| `src/ingestion/` | Compile-time ingestor traits, registry, and the optional waveform simulator. |
| `src/http/resources/ingestors/` | SQLite instances, admin CRUD, worker lifecycle, and runtime status. |
| `src/settings/` | YAML configuration and environment variables; precedence: environment, file, defaults. |
| `src/tls/` | TLS configuration and certificates. |
| `src/bin/liraysctl.rs`, `src/liraysctl/` | The `lirays` administration binary: users, tokens, configuration, service management, and metrics. |
| `Makefile`, `Dockerfile`, `packaging/`, `.github/workflows/` | Builds, distribution, installers, and CI. |

### Data and Communication

- The server handles HTTP and WebSocket on the same port (8245 by default).
  WebSocket is exposed at `/ws`; authentication and views have HTTP resources.
- Namespace commands pass from the WebSocket service to
  `VariableManager::exec_cmd`. Subscriptions receive value events and tree
  changes; do not assume a polling-only model.
- Messages use types from `lirays-scada-proto` and Protobuf encoding through
  `prost`. SDKs, proto definitions, and full documentation are maintained in
  separate repositories.
- **sled** stores the namespace and values in `data_dir/rt_data`. Values are
  cached in memory, and pending changes are persisted periodically. Graceful
  shutdown also requests a flush.
- **SQLite + SeaORM** stores users, personal access tokens, views, and ingestors in
  `data_dir/static.db`. Schema initialization lives in the corresponding
  resources through `ensure_schema`.
- Sessions have separate storage in `data_dir/sessions`. Optional metrics are
  written to `data_dir/metrics`.
- `AppState` shares managers through `Arc`. Authentication includes sessions,
  personal access tokens, and roles; preserve authorization checks in both
  routes and namespace commands.
- The backend embeds `frontend/build` at compile time through `include_dir!`
  and serves those files. This is a build dependency even for backend-only work.

- Ingestors yield typed values at canonical namespace paths. `VariableManager::ingest`
  validates a whole batch, creates missing folders/variables, and uses existing
  persistence and events. Stop ingestors before the shutdown flush.
- Swagger UI at `/swagger` documents ingestion CRUD. The simulator is selected
  by the default `simulator` Cargo feature; no frontend changes are needed.

## Programming Style

These are the maintainer's preferences, especially for Rust:

- Keep code clean, direct, and readable. Prefer clear names and explicit control
  flow over unnecessary abstractions.
- Add comments only when strictly necessary to explain decisions, constraints,
  or behavior that the code does not explain on its own. Do not comment on
  obvious operations or narrate each step.
- Add documentation (`///` in Rust) when it provides a useful contract,
  especially for public functions used in multiple places. Do not document
  every function by default or repeat its name or signature.
- Do not extract one- or two-line helpers used in a single place merely for
  organization. Extraction makes sense when the function is reused or
  encapsulates a specific, well-defined behavior and meaningfully improves
  readability. Length alone does not justify creating a function.
- Do not add layers, traits, wrappers, or generalizations for hypothetical
  requirements. Follow the existing structure and keep changes focused on
  the task.
- Add tests when they provide real value: regressions, domain rules, validation,
  permissions, persistence, or relevant asynchronous behavior. Avoid trivial
  tests that merely reproduce the implementation, and do not automatically
  test every helper. Extend existing tests when sufficient.
- When changing asynchronous logic, consider lock duration and blocking work.
  Preserve consistency between the namespace, cache, persistence, and events,
  and keep slow clients isolated during event distribution.

## Workflow and Validation

- Start with the README, this map, and the affected modules; there is no need
  to read the entire repository for every task. If documentation differs from
  the code, verify the implementation and update the relevant documentation.
- Run `rtk git status --short` before editing and preserve others' changes.
- To run the server from the repository root:
  `rtk cargo run --bin lirays-scada -- --config settings-dev.yaml`.
  Use local development data; do not reuse production databases for tests.
- Before compiling or testing Rust, check that `frontend/build` exists.
  If missing, the UI must be built: using Node 24, run `rtk npm ci` and
  `rtk npm run build` from `frontend/`. Do not modify frontend code for a
  backend task; if this dependency fails, report the specific blocker.
- Depending on the change, use `rtk cargo fmt --check`, `rtk cargo check`, and
  `rtk cargo test <filter>`; run `rtk cargo test` when the scope warrants the
  full suite. Existing Rust tests live alongside the code in `#[cfg(test)]`
  modules. CI also collects coverage with `cargo llvm-cov`.
- Documentation-only changes do not require builds or tests. Report what
  changed and which checks were actually performed.
