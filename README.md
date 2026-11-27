# LiRAYS-SCADA

[![Docker Release](https://github.com/LiRAYSgj/LiRAYS-Scada/actions/workflows/publish-and-release.yml/badge.svg)](https://github.com/LiRAYSgj/LiRAYS-Scada/actions/workflows/publish-and-release.yml)
[![Frontend tests](https://img.shields.io/github/actions/workflow/status/LiRAYSgj/LiRAYS-Scada/frontend-tests.yml?branch=main&label=frontend%20tests)](https://github.com/LiRAYSgj/LiRAYS-Scada/actions/workflows/frontend-tests.yml)
[![Backend tests](https://img.shields.io/github/actions/workflow/status/LiRAYSgj/LiRAYS-Scada/backend-tests.yml?branch=main&label=backend%20tests)](https://github.com/LiRAYSgj/LiRAYS-Scada/actions/workflows/backend-tests.yml)
[![GitHub release](https://img.shields.io/github/v/release/LiRAYSgj/LiRAYS-Scada?display_name=tag&sort=semver)](https://github.com/LiRAYSgj/LiRAYS-Scada/releases)
[![Docs](https://img.shields.io/badge/docs-help.lirays.com-0A7B83)](https://help.lirays.com)
[![Docker image](https://img.shields.io/badge/ghcr.io-lirays--scada-2496ED?logo=docker&logoColor=white)](https://ghcr.io/liraysgj/lirays-scada)
[![PyPI SDK](https://img.shields.io/pypi/v/lirays?logo=pypi&logoColor=white)](https://pypi.org/project/lirays)
[![crates.io SDK](https://img.shields.io/crates/v/lirays?logo=rust&logoColor=white)](https://crates.io/crates/lirays)
Frontend coverage: [![Coverage](https://codecov.io/gh/LiRAYSgj/LiRAYS-Scada/graph/badge.svg?flag=frontend)](https://app.codecov.io/gh/LiRAYSgj/LiRAYS-Scada?flags%5B0%5D=frontend)
Backend coverage: [![Coverage](https://codecov.io/gh/LiRAYSgj/LiRAYS-Scada/graph/badge.svg?flag=backend)](https://app.codecov.io/gh/LiRAYSgj/LiRAYS-Scada?flags%5B0%5D=backend)

LiRAYS-SCADA is a compact SCADA platform built to run on low-resource hardware while providing a friendly interface to model, visualize, and control industrial processes.

<img width="960" height="485" alt="LiRAYS-SCADA preview" src="https://github.com/user-attachments/assets/d0c77c34-0952-4852-b04f-a4b2c109c0d9" /><br>

This repository contains the SCADA backend and frontend.

SDK/client packages, proto definitions, and full documentation are maintained in separate repositories.

Visit https://help.lirays.com for full documentation.

## Database

Users, access tokens, views, ingestors, and egressors are stored in a relational
database. By default it is the embedded SQLite file `{data_dir}/static.db`, so a
single binary runs without external services. Larger deployments can point the
`database` settings section at PostgreSQL, MySQL, or MariaDB instead:

```yaml
database:
  url: postgres://db.example.com:5432/lirays?sslmode=require
  user: lirays
  password: null          # prefer DATABASE_PASSWORD
  max_connections: 10
```

Use `mysql://host:3306/lirays` for MySQL and MariaDB. Each key has an environment
override: `DATABASE_URL`, `DATABASE_USER`, `DATABASE_PASSWORD`, and
`DATABASE_MAX_CONNECTIONS`. The user and password are inserted into the URL with
percent-encoding, so special characters need no escaping. TLS is configured with
the driver's URL options, for example `sslmode=verify-full&sslrootcert=/path/ca.pem`
for PostgreSQL or `ssl-mode=VERIFY_IDENTITY&ssl-ca=/path/ca.pem` for MySQL/MariaDB.

The database must exist before startup and the user needs permission to create
tables; LiRAYS creates its tables when they are missing. Real-time values,
the namespace, and sessions always stay in `data_dir`, stored with the embedded
fjall engine. Existing SQLite data is not migrated automatically when switching
to an external server.

## Ingestion plugins

The backend includes a compile-time `simulator` plugin. Open `/swagger` on the
running server to create and manage instances; the OpenAPI document is available
at `/api-docs/openapi.json`. Swagger UI currently loads its assets from a CDN,
so the browser needs internet access. With authentication enabled, sign in as an
admin in the same browser, or use Swagger's **Authorize** button with an admin
personal access token.

Use `POST /api/ingestors` with:

```json
{
  "name": "Waveform simulator",
  "plugin_type": "simulator",
  "enabled": true,
  "config": {
    "interval_ms": 100,
    "variables": [
      {
        "path": "/plant/random",
        "waveform": "random",
        "min": 0,
        "max": 100,
        "decimal_places": 2
      },
      {
        "path": "/plant/sine",
        "waveform": "sine",
        "min": 0,
        "max": 100,
        "period_ms": 5000,
        "phase_degrees": 0,
        "decimal_places": 2
      },
      {
        "path": "/plant/square",
        "waveform": "square",
        "min": 0,
        "max": 100,
        "period_ms": 5000,
        "phase_degrees": 90
      },
      {
        "path": "/plant/ramp",
        "waveform": "ramp",
        "min": 0,
        "max": 100,
        "period_ms": 10000,
        "phase_degrees": 0,
        "decimal_places": 2
      }
    ]
  }
}
```

The simulator emits one batch containing all configured variables immediately,
then every `interval_ms` milliseconds (a positive integer, default `1000`). The
interval is shared by the entire instance. Each variable defines its own absolute
namespace `path`, `waveform`, inclusive `min` and `max`, and optional `decimal_places`.
Equal bounds generate a constant. The list must be nonempty; paths must be
canonical, unique within the instance, and cannot be parents of other variables.
Missed ticks are skipped instead of replayed in a burst.

A `path` can expand into many variables with the namespace bulk ADD syntax in
any segment: a numeric range `[start:end]` or `[start:end:step]` (inclusive,
`start` defaults to 1) or a list of options `[A,B,C]`, with optional text before
and after the brackets. The expansion is the combination of every segment, and
each resulting variable shares the rest of the entry's configuration. For
example, `"/plant/line[1:4]/ramp[1:100]"` creates folders `line1` to `line4`
with variables `ramp1` to `ramp100` in each one. The rules above apply to the
expanded paths, and an instance can expand to at most 100,000 variables.

Bounds control generation, not constraints on the SCADA variable. Missing folders
and variables are created with an inferred type and no optional metadata.
Existing variables retain their metadata and use the normal SCADA value
validation/conversion rules. Removing a path from the configuration stops updates
to that path but does not delete its variable. Earlier flat simulator configurations
must be updated to the `variables` list format.

Supported waveforms are `random` (the default when omitted), `sine`, `square`,
and `ramp` (an ascending sawtooth). Periodic variables require a positive integer
`period_ms` and accept `phase_degrees` (default `0`; negative angles are allowed).
The sampling interval is independent of each variable's waveform period.

At phase zero, sine starts at the midpoint and rises, square starts at `max`
with a fixed 50% duty cycle, and ramp starts at `min`, rises toward `max`, then
resets to `min`. Positive phase advances the waveform. All variables share an
elapsed-time origin at the first sample; restarting an instance resets that
origin. Delayed samples use actual elapsed monotonic time, so the waveform does
not slow down when ticks are missed. Choose an interval much shorter than the
period to observe the waveform; sampling once per period can appear constant.

Optional `decimal_places` (0–15) selects values at that decimal precision within
`min` and `max`; omit it to retain unrounded floating-point generation. Periodic outputs are
rounded to the closest allowed value at that precision. Ranges
with no representable value at that precision, or whose scaled bounds exceed
exact integer precision in a float, are rejected. Values remain floats, including
when `decimal_places` is zero. Trailing zeros are a display concern, not stored
numeric precision.

- `GET /api/ingestors/plugins`: compiled-in plugin types.
- `GET /api/ingestors` and `GET /api/ingestors/{id}`: configuration and status.
- `PUT /api/ingestors/{id}`: merge only the supplied fields; set `enabled` to
  `false` to stop or `true` to start/restart the instance.
- `DELETE /api/ingestors/{id}`: stop and delete the instance. Generated namespace
  variables and their last values remain available.

Updates are generic across all ingestion plugins. For example,
`{"config":{"interval_ms":500}}` changes only the sampling interval, and
`{"enabled":false}` stops an instance without resending its configuration.
Configuration objects merge recursively; arrays such as `variables` are replaced
as a whole (they are not merged by index or path). A `null` value removes a key
inside `config`. Required instance fields cannot be null or removed. An empty
update `{}` is a no-op. The selected plugin validates the complete merged
configuration before anything is saved or its worker is replaced; invalid
updates leave the stored configuration and running worker unchanged.

All ingestion endpoints require admin access when authentication is enabled.
Instances persist in the configured database and enabled instances resume on startup. A source or
batch-processing error stops that instance and appears in `status.last_error`;
a valid update restarts it. Plugins omitted from a subsequent build leave their
instances persisted with an error status.

The plugin contract is in `src/ingestion/mod.rs`: implement `IngestorPlugin` to
validate configuration and create an `Ingestor` whose asynchronous `next_batch`
returns paths and typed values. Register factories in `builtin_plugins()` (or
supply a registry to `IngestorManager`). The engine applies one batch before
requesting the next, without an unbounded queue. Pending reads must support
cancellation. Plugins are trusted code compiled into the server, not runtime
loads. Build with `rtk cargo build --no-default-features` to omit the simulator;
the ingestion interface and endpoints remain available.

## Egress plugins

Egress instances publish namespace values to external destinations. The backend
includes a compile-time `udp` plugin as an example. Use `POST /api/egressors` with:

```json
{
  "name": "UDP stream",
  "plugin_type": "udp",
  "enabled": true,
  "config": {
    "variables": ["/plant/**"],
    "mode": { "type": "subscription" },
    "target": { "host": "127.0.0.1", "port": 9000 }
  }
}
```

`variables` lists canonical paths or patterns. `*` matches any characters within
one path segment (`/plant/*/temp`, `/line*/temp`) and a whole `**` segment matches
any depth (`/plant/**`, or `/**` for the entire namespace). Patterns resolve to
existing variables by scanning only the stored subtree below their literal prefix,
and they are resolved again when folders under that prefix change, so new
variables are picked up automatically.

`mode` selects how values are read:

- `{"type": "subscription"}` sends the current values once, then every change
  of a selected variable.
- `{"type": "polling", "interval_ms": 1000}` sends the current values of all
  selected variables every interval. Missed ticks are skipped.

Values are collected while a publish is in progress and only the latest value of
each variable is kept, so a slow destination delays values without unbounded
memory. A failed publish sets `status.state` to `error` with `status.last_error`,
keeps the instance running, and retries the unsent values with a backoff of up to
30 seconds. On shutdown, ingestors stop first and egress instances get one last
short attempt to publish pending values.

`target` belongs to the plugin. The `udp` plugin sends JSON datagrams of at most
1400 bytes, splitting larger batches, without authentication or delivery
guarantees. Listen with `nc -ul 9000` to see them:

```json
{"ts":1764211200000,"values":[{"path":"/plant/temp","value":21.4}]}
```

The endpoints mirror the ingestion ones (`/api/egressors`, `/api/egressors/plugins`,
`/api/egressors/{id}`), including partial updates, admin-only access, persistence
in the configured database, and restoration on startup.

The plugin contract is in `src/egress/mod.rs`: implement `EgressPlugin` to validate
`target` and create an `Egressor` whose asynchronous `publish` sends one batch.
The engine handles variable selection, read mode, batching, and retries, and runs
at most one publish per instance at a time. Register factories in
`builtin_plugins()`. Build with `--no-default-features` to omit the `udp` example.
