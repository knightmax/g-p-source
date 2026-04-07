#!/usr/bin/env bash
# gps-discover.sh — Find the running gpsource instance for a workspace.
# Usage: gps-discover.sh [workspace_path]
# Output: JSON with {port, pid, status, workspace} or exit 1 if not found.

set -euo pipefail

WORKSPACE="${1:-$(pwd)}"
WORKSPACE="$(cd "$WORKSPACE" && pwd -P)"  # canonical path

INSTANCES_DIR="$HOME/.gps/instances"

if [ ! -d "$INSTANCES_DIR" ]; then
  echo '{"error": "no instances directory"}' >&2
  exit 1
fi

# Search all instance files for one matching this workspace
for f in "$INSTANCES_DIR"/*.json; do
  [ -f "$f" ] || continue
  FILE_WORKSPACE=$(python3 -c "import json,sys; print(json.load(open('$f')).get('workspace',''))" 2>/dev/null || true)
  if [ "$FILE_WORKSPACE" = "$WORKSPACE" ]; then
    PID=$(python3 -c "import json; print(json.load(open('$f')).get('pid',0))" 2>/dev/null || echo 0)
    # Check if PID is alive
    if kill -0 "$PID" 2>/dev/null; then
      cat "$f"
      exit 0
    else
      echo '{"error": "stale instance (pid dead)", "file": "'"$f"'"}' >&2
      rm -f "$f"
      exit 1
    fi
  fi
done

echo '{"error": "no instance found for workspace"}' >&2
exit 1
