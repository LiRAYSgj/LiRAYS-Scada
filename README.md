# LiRAYS‑SCADA

Rust-based SCADA server with an embedded Svelte UI, HTTP + WebSocket on the same port, and local persistence (sled + SQLite).

## Download & Install

Descargas: usa la versión publicada en GitHub Releases (reemplaza `v0.1.0` por la nueva versión cuando corresponda).

### Ubuntu / Debian (amd64, arm64)

- Descarga:
  - [lirays-scada_0.1.0-1_amd64.deb](https://github.com/LiRAYSgj/LiRAYS-Scada/releases/download/v0.1.0/lirays-scada_0.1.0-1_amd64.deb)
  - [lirays-scada_0.1.0-1_arm64.deb](https://github.com/LiRAYSgj/LiRAYS-Scada/releases/download/v0.1.0/lirays-scada_0.1.0-1_arm64.deb)
- Instala:

```sh
sudo dpkg -i lirays-scada_0.1.0_*.deb
# if dependencies are missing:
sudo apt-get install -f
```

- Service: `systemctl status lirays-scada.service` (listens on 8245 by default).

Uninstall:

```sh
# prerm already stops/disables the service
sudo apt-get remove -y lirays-scada
```

Optional cleanup (data/logs):

```sh
sudo rm -rf /var/lib/lirays-scada /var/log/lirays-scada.log
```

### macOS

Not supported yet.

### Windows

Not supported yet.

### Docker

- Public image (Not available yet): `docker pull registry.example.com/lirays-scada:0.1.0`
- Or build from the repo:

```sh
docker build --target production -t lirays:latest .
docker run --rm -p 8245:8245 -v $(pwd)/data_dir:/data lirays:latest
```

## Build desde código fuente

- `make` (en Debian/Ubuntu) genera el instalador .deb local.
- `make deb-docker-amd64` / `make deb-docker-arm64` (desde macOS/Linux, requiere Docker Desktop)
  - `make clean`, `make test`

## Documentation

- Architecture: [doc/architecture.md](doc/architecture.md)
- Configuration & operations: [doc/configuration.md](doc/configuration.md)
- API & WebSocket: [doc/api.md](doc/api.md)
- Developer guide: [doc/development.md](doc/development.md)
