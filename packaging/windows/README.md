# Windows installer pack

Contents expected in the zip:
- `lirays-scada.exe` (built for x86_64-pc-windows-gnu or msvc)
- `nssm.exe` (drop in manually; not bundled here)
- `install.ps1` / `uninstall.ps1`

Usage:
```powershell
Set-ExecutionPolicy Bypass -Scope Process -Force
.\install.ps1   # installs as Windows service "LiRaysScada"
# to uninstall
.\uninstall.ps1
```

Defaults:
- Service: `LiRaysScada`
- DATA_DIR: `C:\ProgramData\LiRays-Scada\data_dir`
- Port: 8245
- Logs: `C:\ProgramData\LiRays-Scada\logs\lirays-scada.(out|err).log`

Note: place `nssm.exe` alongside the scripts before zipping.***
