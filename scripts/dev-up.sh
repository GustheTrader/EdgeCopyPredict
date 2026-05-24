#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo "Starting infrastructure (Postgres + Redis)..."
docker compose up -d db redis

echo "Waiting for Postgres to be ready..."
until docker compose exec -T db pg_isready -U arbitek -d arbitek; do
  sleep 1
done

echo "Running migrations..."
cd backend
cargo run --bin arbitek-migrate 2>/dev/null || \
  cargo run --bin arbitek-cli -- migrate 2>/dev/null || \
  echo "Migration binary not found, skipping..."

echo "Done! Infrastructure is ready."
echo ""
echo "Next steps:"
echo "  cargo run --bin arbitek-api       # Start API"
echo "  cargo run --bin arbitek-worker    # Start worker"
echo "  cd frontend && pnpm run dev      # Start frontend"
