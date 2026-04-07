#!/usr/bin/env bash
# gps-status.sh — Check gpsource health via the JSON-RPC API.
# Usage: gps-status.sh [workspace_path]
# Output: JSON status response.

set -euo pipefail

WORKSPACE="${1:-$(pwd)}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Discover instance
INSTANCE=$("$SCRIPT_DIR/gps-discover.sh" "$WORKSPACE" 2>/dev/null) || {
  echo '{"error": "gpsource not running"}' >&2
  exit 1
}

PORT=$(echo "$INSTANCE" | python3 -c "import json,sys; print(json.load(sys.stdin).get('port',0))")
TOKEN=$(cat ~/.gps/auth-token 2>/dev/null || echo "")

if [ -z "$TOKEN" ]; then
  echo '{"error": "auth token not found at ~/.gps/auth-token"}' >&2
  exit 1
fi

# Call status endpoint
curl -sf "http://127.0.0.1:${PORT}" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${TOKEN}" \
  -d '{"jsonrpc":"2.0","method":"status","params":{},"id":1}'
