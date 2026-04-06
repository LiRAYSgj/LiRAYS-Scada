# Configuration & Operations

## Environment variables
- `BIND_HOST` / `BIND_PORT` – unified host/port for HTTP + WS (default `0.0.0.0:8245`).
- `DATA_DIR` – data root (default `./data_dir`); contains `rt_data/` (sled) and `static.db` (SQLite).
- TLS:
  - `TLS_ENABLE` – enable TLS for HTTP and WS.
  - `TLS_AUTO` – when true, generates self-signed cert/key in `${DATA_DIR}/certificates/` if no cert/key provided.
  - `TLS_CERT_PATH` / `TLS_KEY_PATH` – PEM paths when TLS is on and AUTO is off.
- Auth (optional):
  - `AUTH_ENABLED` – gate SPA, Swagger, API.
  - `AUTH_SECRET` – HMAC for session cookie; if missing, a random one is generated (sessions invalidated on restart).
- Observability:
  - `METRICS_DIR` – optional; if set, emits `metrics_rt.txt` (snapshot) and `metrics_hist.csv` (history) every 5s.
- Persistence:
  - `PERSIST_FLUSH_MS` – interval (ms) to flush values to sled (`valuesTree`); default 1000 ms.

## Directories & files
- `${DATA_DIR}/rt_data/` – sled trees (`mainTree`, `valuesTree`).
- `${DATA_DIR}/static.db` – SQLite for static resources and users.
- `${DATA_DIR}/certificates/` – self-signed pair when `TLS_AUTO=1`.
- Service logs (packages): `/var/log/lirays-scada*.log`.

## Authentication
- First visit with `AUTH_ENABLED=true`: redirect to `/auth/setup` to create `admin` password.
- Next visits: `/auth/login`.
- Cookie: `lirays_session`, HttpOnly, TTL 24h, `Secure` if TLS.

## TLS
- With `TLS_ENABLE=true`:
  - Use `TLS_CERT_PATH`/`TLS_KEY_PATH` if provided.
  - Otherwise, with `TLS_AUTO=1`, generate self-signed in `${DATA_DIR}/certificates/`.
  - HTTP serves HTTPS on `BIND_PORT`, WS upgrades to `wss` on `/ws`.

## Metrics
- If `METRICS_DIR` is set:
  - Colored snapshot: `${METRICS_DIR}/metrics_rt.txt` (overwrites).
  - CSV history: `${METRICS_DIR}/metrics_hist.csv` (append).
- Quick view:
  - `watch -n1 cat "$METRICS_DIR/metrics_rt.txt"` (Linux) or `while true; do clear; cat ...; sleep 1; done`.

## Persistence & shutdown
- `values_cache` keeps values in memory; dirty keys flush every `PERSIST_FLUSH_MS` or on SIGINT/SIGTERM.
- sled provides local durability; SQLite handles static resources and users.
