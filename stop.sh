#!/usr/bin/env bash
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
LOG_DIR="$SCRIPT_DIR/logs"

echo "=== arbitek: stopping all services ==="
for pid_file in "$LOG_DIR"/*.pid; do
  [ -f "$pid_file" ] || continue
  name=$(basename "$pid_file" .pid)
  pid=$(cat "$pid_file")
  if kill -0 "$pid" 2>/dev/null; then
    echo "  stopping $name (pid $pid)"
    kill "$pid" 2>/dev/null || true
  fi
  rm -f "$pid_file"
done
echo "Done."
