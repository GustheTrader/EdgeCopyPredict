#!/usr/bin/env bash
# Quick start — runs pre-built binaries directly, skips cargo build.
# Use this for daily restarts. Run start_all.py --full only after code changes.
set -e

ROOT="$(cd "$(dirname "$0")" && pwd)"
BIN="$ROOT/backend/target/debug"
ENV_FILE="$ROOT/.env"

if [ ! -f "$BIN/arbitek-api" ] || [ ! -f "$BIN/arbitek-worker" ]; then
  echo "[ERR] Binaries not built yet. Run: python3 start_all.py --full"
  exit 1
fi

# Load .env into environment
set -a
# shellcheck disable=SC1090
source <(grep -v '^#' "$ENV_FILE" | grep '=' | sed 's/\r//')
set +a

export BIND_ADDR="${APP_BIND_ADDR:-127.0.0.1:8080}"
export ENVIRONMENT="${APP_ENV:-development}"
export LOG_LEVEL="${APP_LOG_LEVEL:-info}"
export AUTH__JWT_SECRET="${AUTH__JWT_SECRET:-dev-secret-change-me-in-production-min-32-chars-long}"
export AUTH__ADMIN_PASSWORD="${AUTH__ADMIN_PASSWORD:-secret}"
export AUTH__TOKEN_EXPIRY_HOURS="24"
export WORKER__RUN_MIGRATIONS_ON_STARTUP="true"
export WORKER__POLYMARKET_POLL_INTERVAL_SECONDS="${POLYMARKET_POLL_INTERVAL_SECONDS:-5}"
export POLYMARKET__DATA_API_BASE_URL="${POLY_DATA_API_BASE_URL:-https://data-api.polymarket.com}"
export POLYMARKET__CLOB_API_BASE_URL="${POLY_CLOB_API_BASE_URL:-https://clob.polymarket.com}"
export POLYMARKET__PRIVATE_KEY="$POLY_PRIVATE_KEY"
export POLYMARKET__API_KEY="$POLY_API_KEY"
export POLYMARKET__API_SECRET="$POLY_API_SECRET"
export POLYMARKET__API_PASSPHRASE="$POLY_API_PASSPHRASE"
export KALSHI__REST_API_BASE_URL="${KAL_API_BASE_DEVELOPMENT:-https://external-api.demo.kalshi.co/trade-api/v2}"
export KALSHI__REST_API_BASE_URL_LIVE="${KAL_API_BASE_PRODUCTION:-https://external-api.kalshi.com/trade-api/v2}"
export KALSHI__WEBSOCKET_BASE_URL="${KAL_API_WS:-wss://external-api-ws.demo.kalshi.co/trade-api/ws/v2}"
export KALSHI__API_KEY="$KAL_API_KEY"
export KALSHI__API_SECRET="$KAL_API_SECRET"
export KALSHI__ENVIRONMENT="development"
export COPY_WALLETS="$COPY_WALLETS"

PIDS=()

cleanup() {
  echo ""
  echo "[STOP] Shutting down..."
  for pid in "${PIDS[@]}"; do
    kill "$pid" 2>/dev/null || true
  done
  exit 0
}
trap cleanup INT TERM

echo "[START] API..."
"$BIN/arbitek-api" &
PIDS+=($!)
sleep 4

echo "[START] Workers..."
for mode in oddpool-copy ingest-kalshi signal-normalize risk-evaluate execution-live reconcile; do
  WORKER__MODE="$mode" "$BIN/arbitek-worker" &
  PIDS+=($!)
done

echo "[START] Frontend..."
cd "$ROOT/frontend" && pnpm run dev &
PIDS+=($!)

echo "[OK] All services started. Ctrl+C to stop."
echo "[OK] App: http://localhost:3000"
wait
