$ErrorActionPreference = "Stop"

# Paths
$Base = Split-Path -Parent $MyInvocation.MyCommand.Definition
$Exe = Join-Path $Base "lirays-scada.exe"
$Nssm = Join-Path $Base "nssm.exe"
$ServiceName = "LiRaysScada"
$DataDir = "C:\ProgramData\LiRays-Scada\data_dir"
$LogDir = "C:\ProgramData\LiRays-Scada\logs"

if (!(Test-Path $Nssm)) {
    Write-Error "nssm.exe not found at $Nssm. Place nssm.exe in the same folder."
}

Write-Host "Creating data/log directories..."
New-Item -ItemType Directory -Force -Path $DataDir | Out-Null
New-Item -ItemType Directory -Force -Path $LogDir | Out-Null

Write-Host "Installing service $ServiceName..."
& $Nssm install $ServiceName $Exe | Out-Null
& $Nssm set $ServiceName AppDirectory $Base | Out-Null
& $Nssm set $ServiceName Start SERVICE_AUTO_START | Out-Null
& $Nssm set $ServiceName AppEnvironmentExtra "DATA_DIR=$DataDir" "BIND_PORT=8245" | Out-Null
& $Nssm set $ServiceName AppStdout "$LogDir\lirays-scada.out.log" | Out-Null
& $Nssm set $ServiceName AppStderr "$LogDir\lirays-scada.err.log" | Out-Null

Write-Host "Starting service..."
& $Nssm start $ServiceName | Out-Null

Write-Host "Done. Service $ServiceName running on port 8245."
