$ErrorActionPreference = "Stop"

$Base = Split-Path -Parent $MyInvocation.MyCommand.Definition
$Nssm = Join-Path $Base "nssm.exe"
$ServiceName = "LiRaysScada"

if (!(Test-Path $Nssm)) {
    Write-Error "nssm.exe not found at $Nssm. Place nssm.exe in the same folder."
}

Write-Host "Stopping and removing service $ServiceName..."
& $Nssm stop $ServiceName  | Out-Null
& $Nssm remove $ServiceName confirm | Out-Null

Write-Host "Service removed. Data/logs left untouched in C:\ProgramData\LiRays-Scada."
