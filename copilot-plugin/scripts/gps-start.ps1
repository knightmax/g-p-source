# gps-start.ps1 — Start gpsource for a workspace if not already running.
# Usage: gps-start.ps1 [-Workspace <path>]
# Output: JSON instance info when ready, or error.

param(
    [string]$Workspace = (Get-Location).Path
)

$ErrorActionPreference = 'Stop'

$Workspace = (Resolve-Path $Workspace).Path
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

# Check if already running
try {
    $result = & "$ScriptDir\gps-discover.ps1" -Workspace $Workspace 2>$null
    if ($result) {
        Write-Output $result
        exit 0
    }
} catch { }

# Check if gpsource is installed
$gpsExe = Get-Command gpsource -ErrorAction SilentlyContinue
if (-not $gpsExe) {
    Write-Error '{"error": "gpsource not found in PATH. Install with: cargo install --path /path/to/g-p-source"}'
    exit 1
}

# Start gpsource in background
$hash = [System.BitConverter]::ToString(
    [System.Security.Cryptography.SHA256]::Create().ComputeHash(
        [System.Text.Encoding]::UTF8.GetBytes($Workspace)
    )
).Replace('-','').Substring(0,16).ToLower()
$LogFile = Join-Path $env:TEMP "gps-$hash.log"

$proc = Start-Process -FilePath 'gpsource' `
    -ArgumentList "--workspace-root `"$Workspace`"" `
    -RedirectStandardOutput $LogFile `
    -RedirectStandardError "$LogFile.err" `
    -PassThru -WindowStyle Hidden

Write-Host "Started gpsource (pid=$($proc.Id)), log=$LogFile" -ForegroundColor Cyan

# Wait for discovery file to appear and status to become ready (max 120s)
$maxWait = 120
$waited = 0

while ($waited -lt $maxWait) {
    Start-Sleep -Seconds 1
    $waited++

    try {
        $result = & "$ScriptDir\gps-discover.ps1" -Workspace $Workspace 2>$null
        if ($result) {
            $data = $result | ConvertFrom-Json
            if ($data.status -eq 'ready') {
                Write-Output $result
                exit 0
            }
            if ($waited % 10 -eq 0) {
                Write-Host "Still indexing... (${waited}s)" -ForegroundColor Yellow
            }
        }
    } catch { }

    # Check if process died
    if ($proc.HasExited) {
        Write-Error ('{"error": "gpsource exited unexpectedly. Check ' + $LogFile + '"}')
        exit 1
    }
}

# Timed out — return current state
try {
    $result = & "$ScriptDir\gps-discover.ps1" -Workspace $Workspace 2>$null
    if ($result) { Write-Output $result; exit 0 }
} catch { }

Write-Error '{"error": "timeout waiting for gpsource to become ready"}'
exit 1
