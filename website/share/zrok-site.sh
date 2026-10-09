#!/bin/bash
# Public HTTPS share of the Basalt website via zrok.
set -uo pipefail
SHARE_LOG=/root/basalt/website/share/zrok-share.log
URL_FILE=/root/basalt/website/share/zrok-url.txt
: > "$SHARE_LOG"
zrok2 share public --headless --backend-mode proxy http://127.0.0.1:8899 >>"$SHARE_LOG" 2>&1 &
for _ in $(seq 1 60); do
  HOST=$(grep -oE '[a-z0-9-]+\.shares\.zrok\.io' "$SHARE_LOG" | head -1)
  [ -n "$HOST" ] && break
  sleep 1
done
if [ -z "$HOST" ]; then echo "ERROR: no hostname" >&2; exit 1; fi
echo "https://$HOST" > "$URL_FILE"
echo "URL: https://$HOST"
