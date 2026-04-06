# Configuration & Operations

Applies to both package installs (systemd-managed) and manual runs.

## Paths & service (packages)

- Binary: `/usr/bin/lirays-scada`
- Config: `/etc/lirays-scada/settings.yaml` (loaded automatically unless `--config` points elsewhere)
- Data root: `/var/lib/lirays-scada/data` (sled + SQLite + certificates)
- Systemd service: `lirays-scada.service`
  - Status/logs: `systemctl status lirays-scada`, `journalctl -u lirays-scada`

## Runtime flags

- `--config /path/to/settings.yaml` – override default config file
- No other CLI flags; everything else is env or YAML.

## Environment variables (override YAML)

- Server:
  - `BIND_HOST` / `BIND_PORT` — unified listener (default `0.0.0.0:8245`)
  - `DATA_DIR` — data root (default `./data` when running manually; `/var/lib/lirays-scada/data` in packages)
- TLS:
  - `TLS_ENABLE` — enable TLS for HTTP and WS
  - `TLS_AUTO` — generate self-signed under `${DATA_DIR}/certificates` when no cert/key
  - `TLS_CERT_PATH` / `TLS_KEY_PATH` — PEM files when `TLS_ENABLE=true` and `TLS_AUTO=false`
- Auth (optional):
  - `AUTH_ENABLED` — gate UI/API
  - `AUTH_SECRET` — HMAC for access/refresh tokens; random per start if unset
- Observability:
  - `METRICS_DIR` — writes `metrics_rt.txt` (snapshot) + `metrics_hist.csv` (history, 5s cadence)
- Persistence:
  - `PERSIST_FLUSH_MS` — interval to flush sled values (default 15000)

## Directories & files

- `${DATA_DIR}/rt_data/` — sled trees (`mainTree`, `valuesTree`)
- `${DATA_DIR}/static.db` — SQLite for static resources + users
- `${DATA_DIR}/certificates/` — self-signed pair when `TLS_AUTO=true`
- Metrics (optional): `${METRICS_DIR}/metrics_rt.txt`, `${METRICS_DIR}/metrics_hist.csv`

## Authentication

- Enable with `AUTH_ENABLED=true`.
- First visit redirects to `/auth/setup` to create admin password.
- Logins at `/auth/login`; session via HttpOnly `lirays_session` (1h) + `lirays_refresh` (24h), `Secure` when TLS is on.

## TLS

- Unified for HTTP + WS on the same port.
- Choose one:
  - Provide `TLS_CERT_PATH` and `TLS_KEY_PATH`
  - Or set `TLS_AUTO=true` to auto-generate self-signed under `${DATA_DIR}/certificates`
- WebSocket switches to `wss` automatically when TLS is active.

## Metrics

- Enable by setting `METRICS_DIR` to a writable folder.
- Watch snapshot: `watch -n1 cat "$METRICS_DIR/metrics_rt.txt"` (Linux) or `while true; do clear; cat "$METRICS_DIR/metrics_rt.txt"; sleep 1; done`.

## Persistence & shutdown

- Dirty values flush every `PERSIST_FLUSH_MS` ms and again on SIGINT/SIGTERM.
- sled provides local durability; SQLite stores static resources and users.
