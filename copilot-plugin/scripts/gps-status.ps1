# gps-status.ps1 — Check gpsource health via the JSON-RPC API.
# Usage: gps-status.ps1 [-Workspace <path>]
# Output: JSON status response.

param(
    [string]$Workspace = (Get-Location).Path
)

$ErrorActionPreference = 'Stop'

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

# Discover instance
try {
    $instance = & "$ScriptDir\gps-discover.ps1" -Workspace $Workspace 2>$null
} catch {
    Write-Error '{"error": "gpsource not running"}'
    exit 1
}

if (-not $instance) {
    Write-Error '{"error": "gpsource not running"}'
    exit 1
}

$data = $instance | ConvertFrom-Json
$port = $data.port

$tokenPath = Join-Path $env:USERPROFILE '.gps' 'auth-token'
if (-not (Test-Path $tokenPath)) {
    Write-Error '{"error": "auth token not found at ~/.gps/auth-token"}'
    exit 1
}
$token = Get-Content $tokenPath -Raw

$body = '{"jsonrpc":"2.0","method":"status","params":{},"id":1}'

$response = Invoke-RestMethod -Uri "http://127.0.0.1:$port" `
    -Method Post `
    -ContentType 'application/json' `
    -Headers @{ Authorization = "Bearer $($token.Trim())" } `
    -Body $body

$response | ConvertTo-Json -Depth 10
