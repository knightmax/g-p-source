# gps-discover.ps1 — Find the running gpsource instance for a workspace.
# Usage: gps-discover.ps1 [-Workspace <path>]
# Output: JSON with {port, pid, status, workspace} or throws if not found.

param(
    [string]$Workspace = (Get-Location).Path
)

$ErrorActionPreference = 'Stop'

$Workspace = (Resolve-Path $Workspace).Path

$InstancesDir = Join-Path $env:USERPROFILE '.gps' 'instances'

if (-not (Test-Path $InstancesDir)) {
    Write-Error '{"error": "no instances directory"}'
    exit 1
}

$files = Get-ChildItem -Path $InstancesDir -Filter '*.json' -ErrorAction SilentlyContinue
if (-not $files) {
    Write-Error '{"error": "no instance found for workspace"}'
    exit 1
}

foreach ($f in $files) {
    $data = Get-Content $f.FullName -Raw | ConvertFrom-Json
    if ($data.workspace -eq $Workspace) {
        $pid = $data.pid
        try {
            $proc = Get-Process -Id $pid -ErrorAction Stop
            # PID is alive — return instance info
            Get-Content $f.FullName -Raw
            exit 0
        } catch {
            # PID is dead — stale instance
            Remove-Item $f.FullName -Force
            Write-Error ('{"error": "stale instance (pid dead)", "file": "' + $f.FullName + '"}')
            exit 1
        }
    }
}

Write-Error '{"error": "no instance found for workspace"}'
exit 1
