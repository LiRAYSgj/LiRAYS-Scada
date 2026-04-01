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

### macOS (arm64, x86_64)

- Descarga:
  - DMG/PKG arm64:
    - [lirays-scada-0.1.0-arm64.dmg](https://github.com/LiRAYSgj/LiRAYS-Scada/releases/download/v0.1.0/lirays-scada-0.1.0-arm64.dmg)
    - [lirays-scada-0.1.0-arm64.pkg](https://github.com/LiRAYSgj/LiRAYS-Scada/releases/download/v0.1.0/lirays-scada-0.1.0-arm64.pkg)
  - DMG/PKG x86_64:
    - [lirays-scada-0.1.0-x86_64.dmg](https://github.com/LiRAYSgj/LiRAYS-Scada/releases/download/v0.1.0/lirays-scada-0.1.0-x86_64.dmg)
    - [lirays-scada-0.1.0-x86_64.pkg](https://github.com/LiRAYSgj/LiRAYS-Scada/releases/download/v0.1.0/lirays-scada-0.1.0-x86_64.pkg)
- Abre el `.dmg` y ejecuta el `.pkg`, o bien por CLI:

```sh
sudo installer -pkg lirays-scada-0.1.0-*.pkg -target /
```

- LaunchDaemon service: `sudo launchctl list | grep com.lirays.scada` (port 8245).

Uninstall (removes service and files):

```sh
sudo launchctl bootout system/com.lirays.scada 2>/dev/null || true
sudo rm -f /Library/LaunchDaemons/com.lirays.scada.plist
sudo rm -f /usr/local/bin/lirays-scada
sudo rm -rf /usr/local/var/lirays-scada
sudo rm -f /usr/local/var/log/lirays-scada.log /usr/local/var/log/lirays-scada.err.log
```

### Windows (x86_64)

- Descarga:
  - [Instalador NSIS](https://github.com/LiRAYSgj/LiRAYS-Scada/releases/download/v0.1.0/LiRays-Scada-0.1.0-x86_64-Setup.exe)
- Instalación con el instalador:
  - Ejecuta el `.exe` y sigue el asistente. Crea el servicio `LiRaysScada` y datos en `C:\ProgramData\LiRays-Scada`.

### Docker

- Public image (Not available yet): `docker pull registry.example.com/lirays-scada:0.1.0`
- Or build from the repo:

```sh
docker build --target production -t lirays:latest .
docker run --rm -p 8245:8245 -v $(pwd)/data_dir:/data lirays:latest
```

## Build desde código fuente

- `make` (en macOS, Debian/Ubuntu o Windows) genera el instalador para el SO/arquitectura local sin Docker.
- `make release` (sólo en macOS) genera todos los artefactos: mac arm64/x86_64, deb amd64/arm64 y Windows x86_64.
- Objetivos útiles:
  - `make mac-all`, `make deb-local`, `make windows-local`
  - `make deb-docker-amd64` / `make deb-docker-arm64` (desde macOS, requiere Docker Desktop)
  - `make clean`, `make test`

## Documentation

- Architecture: [doc/architecture.md](doc/architecture.md)
- Configuration & operations: [doc/configuration.md](doc/configuration.md)
- API & WebSocket: [doc/api.md](doc/api.md)
- Developer guide: [doc/development.md](doc/development.md)
