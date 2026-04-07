#!/usr/bin/env bash
# gps-start.sh — Start gpsource for a workspace if not already running.
# Usage: gps-start.sh [workspace_path]
# Output: JSON instance info when ready, or error.

set -euo pipefail

WORKSPACE="${1:-$(pwd)}"
WORKSPACE="$(cd "$WORKSPACE" && pwd -P)"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Check if already running
if "$SCRIPT_DIR/gps-discover.sh" "$WORKSPACE" 2>/dev/null; then
  exit 0
fi

# Check if gpsource is installed
if ! command -v gpsource &>/dev/null; then
  echo '{"error": "gpsource not found in PATH. Install with: cargo install --path /path/to/g-p-source"}' >&2
  exit 1
fi

# Start gpsource in background
LOG_FILE="/tmp/gps-$(echo "$WORKSPACE" | shasum -a 256 | cut -c1-16).log"
nohup gpsource --workspace-root "$WORKSPACE" > "$LOG_FILE" 2>&1 &
GPS_PID=$!
echo "Started gpsource (pid=$GPS_PID), log=$LOG_FILE" >&2

# Wait for discovery file to appear and status to become ready (max 120s)
MAX_WAIT=120
WAITED=0
while [ $WAITED -lt $MAX_WAIT ]; do
  sleep 1
  WAITED=$((WAITED + 1))

  # Try to discover
  RESULT=$("$SCRIPT_DIR/gps-discover.sh" "$WORKSPACE" 2>/dev/null) && {
    STATUS=$(echo "$RESULT" | python3 -c "import json,sys; print(json.load(sys.stdin).get('status',''))" 2>/dev/null || true)
    if [ "$STATUS" = "ready" ]; then
      echo "$RESULT"
      exit 0
    fi
    # Still indexing — keep waiting
    if [ $((WAITED % 10)) -eq 0 ]; then
      echo "Still indexing... (${WAITED}s)" >&2
    fi
  }

  # Check if process died
  if ! kill -0 "$GPS_PID" 2>/dev/null; then
    echo '{"error": "gpsource exited unexpectedly. Check '"$LOG_FILE"'"}' >&2
    exit 1
  fi
done

# Timed out but still running — return current state
"$SCRIPT_DIR/gps-discover.sh" "$WORKSPACE" 2>/dev/null || {
  echo '{"error": "timeout waiting for gpsource to become ready"}' >&2
  exit 1
}
