# macOS packaging plan

- Universal2 PKG: installs service binary, config, launchd daemon, GUI app.
- Payload layout:
  - `/usr/local/bin/lirays-scada` (universal2)
  - `/Library/Application Support/LiRAYS-Scada/settings.yaml` (+ data_dir/)
  - `/Library/LaunchDaemons/com.lirays.scada.plist`
  - `/Applications/LiRAYS Scada.app` (universal2 GUI)
  - Logs: `/Library/Logs/LiRAYS-Scada/`
- Scripts: `preinstall` bootout existing daemon; `postinstall` seeds config, loads/enables daemon; `postupgrade` restarts daemon.
- Icon source: `frontend/static/android-chrome-512x512.png` -> `.icns` via `sips` + `iconutil`.
