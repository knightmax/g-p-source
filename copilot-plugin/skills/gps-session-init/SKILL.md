---
name: gps-session-init
description: >
  Initialize the gpsource indexing engine at the start of a coding session.
  Use this skill at the very beginning of any session that involves navigating,
  reading, or editing code. Also use it when the user opens a new workspace,
  switches projects, asks "is GPS running?", mentions indexing, or when any
  other GPS skill fails with a connection error. Without this initialization,
  all code navigation falls back to slow, naive file traversal — so always
  run this first.
---

# GPS Session Init

The gpsource engine maintains a pre-built symbol index that makes code navigation
10-100x faster than scanning files. This skill ensures the engine is running and
the index is ready before you start working. Skipping this step means every
`locate`, `neighborhood`, and `summary` call will fail.

## Step 1: Check if gpsource is running

Read the discovery directory to find an instance for the current workspace:

```bash
cat ~/.gps/instances/*.json 2>/dev/null
```

Each JSON file describes a running instance. Find the one whose `workspace` field
matches the current directory. Then verify:

1. **Is the PID alive?** — `kill -0 <pid> 2>/dev/null && echo "ALIVE" || echo "DEAD"`
2. **Is the status `ready`?** — Check the `status` field.

If both checks pass, skip to Step 3.

## Step 2: Start if not running

If no instance exists or the PID is dead, start the engine:

```bash
nohup gpsource --workspace-root "$(pwd)" > /tmp/gps-$(pwd | shasum -a 256 | cut -c1-16).log 2>&1 &
```

Then poll until the discovery file shows `ready` (up to 60 seconds):

```bash
for i in $(seq 1 60); do
  STATUS=$(cat ~/.gps/instances/*.json 2>/dev/null | python3 -c "
import sys, json
for line in sys.stdin:
    try:
        d = json.loads(line)
        if '$(pwd)' in d.get('workspace', ''):
            print(d.get('status', 'unknown'))
    except: pass
" 2>/dev/null)
  [ "$STATUS" = "ready" ] && break
  sleep 1
done
```

The engine indexes the full workspace on first start. Subsequent starts reuse the
existing sled database and only re-index changed files.

## Step 3: Verify via API

Read the port from the discovery JSON file and the auth token from `~/.gps/auth-token`,
then call the `status` method:

```bash
curl -s http://127.0.0.1:<port> \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $(cat ~/.gps/auth-token)" \
  -d '{"jsonrpc":"2.0","method":"status","params":{},"id":1}'
```

**Example response:**
```json
{"result": {"status": "ready", "indexed": true, "workspace": "/home/user/project", "port": 54321}}
```

Confirm `indexed: true`. If `indexed: false`, the initial crawl is still running —
queries will work but may return incomplete results.

## Step 4: Confirm to the user

Report: **"GPS engine active — using structural index for this session."**

Store the port and token for use by other GPS skills during this session.

## Error handling

- **gpsource not installed**: tell the user to install it with `cargo install --path .`
  from the g-p-source repository, or `brew install knightmax/tap/gpsource`.
- **Connection refused after start**: check the log at `/tmp/gps-<hash>.log` for errors.
- **Stale discovery file**: if the PID is dead, delete `~/.gps/instances/<hash>.json`
  and restart from Step 2.
