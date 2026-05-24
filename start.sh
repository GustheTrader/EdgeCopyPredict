#!/usr/bin/env bash
# Start all arbitek services against Supabase + external Redis.
# Logs go to logs/<service>.log. Run ./stop.sh to kill everything.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BIN="$SCRIPT_DIR/backend/target/release"
ENV_FILE="$SCRIPT_DIR/backend/.env"
LOG_DIR="$SCRIPT_DIR/logs"
mkdir -p "$LOG_DIR"

# Load env from backend/.env
set -a; source "$ENV_FILE"; set +a

start() {
  local name=$1; shift
  echo "  starting $name"
  "$@" >"$LOG_DIR/$name.log" 2>&1 &
  echo $! >"$LOG_DIR/$name.pid"
}

echo "=== arbitek: starting all services ==="

# API
start api \
  "$BIN/arbitek-api"

# Workers
WORKER="$BIN/arbitek-worker"

start worker-ingest-poly   env WORKER__MODE=ingest-polymarket  WORKER__POLYMARKET_POLL_INTERVAL_SECONDS=30 "$WORKER"
start worker-ingest-kalshi env WORKER__MODE=ingest-kalshi      WORKER__KALSHI_POLL_INTERVAL_SECONDS=30    "$WORKER"
start worker-signal        env WORKER__MODE=signal-normalize                                              "$WORKER"
start worker-risk          env WORKER__MODE=risk-evaluate                                                 "$WORKER"
start worker-reconcile     env WORKER__MODE=reconcile          WORKER__POLYMARKET_POLL_INTERVAL_SECONDS=30 WORKER__KALSHI_POLL_INTERVAL_SECONDS=30 "$WORKER"
start worker-execution     env WORKER__MODE=execution-live     WORKER__EXECUTION_BATCH_SIZE=10 WORKER__POLYMARKET_POLL_INTERVAL_SECONDS=10 WORKER__KALSHI_POLL_INTERVAL_SECONDS=10 "$WORKER"
start worker-paper         env WORKER__MODE=paper-execution    WORKER__EXECUTION_BATCH_SIZE=10 WORKER__POLYMARKET_POLL_INTERVAL_SECONDS=10 "$WORKER"
start worker-pnl           env WORKER__MODE=pnl-refresh        WORKER__POLYMARKET_POLL_INTERVAL_SECONDS=60 "$WORKER"

echo ""
echo "All services started. Logs in $LOG_DIR/"
echo "Run ./stop.sh to stop everything."
echo ""
echo "API:      http://localhost:8080"
echo "Frontend: start separately with  cd frontend && npm run dev"
