# LiRAYS‑SCADA

Compact SCADA server with a web interface that runs comfortably on low-resource hardware. It provides real-time tag handling, visualization, and control through a single service and installs as a native systemd package (amd64/x86_64 and arm64/aarch64).

## What’s included

- Web UI, REST API, Swagger docs, and WebSocket on the same port (default 8245).
- Systemd unit installed and enabled automatically.
- Optional TLS (your certs or auto self‑signed) and optional authentication with session cookies.

## Download (beta 0.1.0)

**Debian / Ubuntu**

- amd64: https://github.com/LiRAYSgj/LiRAYS-Scada/releases/download/v0.1.0/lirays-scada_0.1.0-1_amd64.deb
- arm64: https://github.com/LiRAYSgj/LiRAYS-Scada/releases/download/v0.1.0/lirays-scada_0.1.0-1_arm64.deb

**RHEL / Rocky / Alma / Fedora (EL9)**

- x86_64: https://github.com/LiRAYSgj/LiRAYS-Scada/releases/download/v0.1.0/lirays-scada-0.1.0-1.el9.x86_64.rpm
- aarch64: https://github.com/LiRAYSgj/LiRAYS-Scada/releases/download/v0.1.0/lirays-scada-0.1.0-1.el9.aarch64.rpm

## Install

### Debian / Ubuntu

```sh
sudo dpkg -i lirays-scada_0.1.0-1_amd64.deb   # or _arm64
sudo apt-get -f install                       # pulls any missing deps
```

### RHEL / Rocky / Alma / Fedora

```sh
sudo dnf install -y lirays-scada-0.1.0-1.el9.x86_64.rpm   # or .aarch64.rpm
```

## Verify the service

```sh
systemctl status lirays-scada          # service health
journalctl -u lirays-scada -f          # live logs
```

Default listener: `http://<host>:8245` (auto-switches to HTTPS/WSS when TLS is enabled).

## Configure

- Main config: `/etc/lirays-scada/settings.yaml`
- Data dir: `/var/lib/lirays-scada/data_dir` (sled + SQLite + auto TLS certs)
- Common toggles (YAML or env vars):
  - `TLS_ENABLE=true` and `TLS_AUTO=true` to auto-generate a self-signed cert, or provide `TLS_CERT_PATH` / `TLS_KEY_PATH`.
  - `AUTH_ENABLED=true` to require login; first visit `/auth/setup` to set the admin password, then `/auth/login`.
- Apply changes:

```sh
sudo systemctl restart lirays-scada
```

## Use

- Web UI: `/`
- API docs: `/swagger` (OpenAPI at `/api-docs/openapi.json`)
- WebSocket endpoint: `/ws`

## Remove

- Debian/Ubuntu: `sudo apt remove lirays-scada` (use `apt purge` to also drop `/etc`), then `sudo rm -rf /var/lib/lirays-scada` if you want all data gone.
- RHEL/Rocky/Alma/Fedora: `sudo dnf remove lirays-scada`, optional `sudo rm -rf /var/lib/lirays-scada`.
  The systemd unit `lirays-scada.service` stops and disables automatically on removal.

## More docs

- Architecture: [doc/architecture.md](doc/architecture.md)
- Configuration & operations: [doc/configuration.md](doc/configuration.md)
- API & WebSocket: [doc/api.md](doc/api.md)
- Developer guide: [doc/development.md](doc/development.md)
