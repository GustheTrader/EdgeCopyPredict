#!/usr/bin/env python3
"""Arbitek local development orchestrator.

Starts Postgres, Redis, runs migrations/seeds, and optionally launches
API server, worker processes, and frontend dev server.

Usage:
    python3 start_all.py              # Start infrastructure only
    python3 start_all.py --full       # Start infra + API + workers + frontend
    python3 start_all.py --api        # Start infra + API only
    python3 start_all.py --workers    # Start infra + workers only
"""

import argparse
import os
import signal
import subprocess
import sys
import time

# ---------------------------------------------------------------------------
# Configuration — loaded from .env
# ---------------------------------------------------------------------------

PROJECT_ROOT = os.path.dirname(os.path.abspath(__file__))
BACKEND_DIR = os.path.join(PROJECT_ROOT, "backend")
FRONTEND_DIR = os.path.join(PROJECT_ROOT, "frontend")


def load_env_file(path: str) -> dict:
    """Parse a .env file and return key/value pairs."""
    env = {}
    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            if "=" not in line:
                continue
            key, _, value = line.partition("=")
            key = key.strip()
            value = value.strip()
            # Strip optional surrounding quotes and unescape \n inside quoted values
            if (value.startswith('"') and value.endswith('"')) or \
               (value.startswith("'") and value.endswith("'")):
                value = value[1:-1].replace("\\n", "\n")
            env[key] = value
    return env


_dotenv = load_env_file(os.path.join(PROJECT_ROOT, ".env"))

ENV = {
    **os.environ,
    # Database & cache
    "DATABASE_URL":            _dotenv.get("DATABASE_URL", ""),
    "REDIS_URL":               _dotenv.get("REDIS_URL", ""),
    # Server
    "BIND_ADDR":               _dotenv.get("APP_BIND_ADDR", "127.0.0.1:8080"),
    "ENVIRONMENT":             _dotenv.get("APP_ENV", "development"),
    "LOG_LEVEL":               _dotenv.get("APP_LOG_LEVEL", "info"),
    # Auth
    "AUTH__JWT_SECRET":        _dotenv.get("AUTH__JWT_SECRET", "dev-secret-change-me-in-production-min-32-chars-long"),
    "AUTH__ADMIN_PASSWORD":    _dotenv.get("AUTH__ADMIN_PASSWORD", "secret"),
    "AUTH__TOKEN_EXPIRY_HOURS":"24",
    # Trading gates
    "LIVE_TRADING_ENABLED":    _dotenv.get("LIVE_TRADING_ENABLED", "false"),
    # Worker
    "WORKER__MODE":            _dotenv.get("WORKER_MODE", "execution-live-once"),
    "WORKER__RUN_MIGRATIONS_ON_STARTUP": _dotenv.get("WORKER_RUN_MIGRATIONS", "true"),
    "WORKER__EXECUTION_BATCH_SIZE":      _dotenv.get("EXECUTION_BATCH_SIZE", "10"),
    "WORKER__POLYMARKET_POLL_INTERVAL_SECONDS": _dotenv.get("POLYMARKET_POLL_INTERVAL_SECONDS", "30"),
    "WORKER__KALSHI_POLL_INTERVAL_SECONDS":     _dotenv.get("KALSHI_POLL_INTERVAL_SECONDS", "30"),
    # Polymarket
    "POLYMARKET__DATA_API_BASE_URL": _dotenv.get("POLY_DATA_API_BASE_URL", "https://data-api.polymarket.com"),
    "POLYMARKET__CLOB_API_BASE_URL": _dotenv.get("POLY_CLOB_API_BASE_URL", "https://clob.polymarket.com"),
    "POLYMARKET__PRIVATE_KEY":       _dotenv.get("POLY_PRIVATE_KEY", ""),
    "POLYMARKET__API_KEY":           _dotenv.get("POLY_API_KEY", ""),
    "POLYMARKET__API_SECRET":        _dotenv.get("POLY_API_SECRET", ""),
    "POLYMARKET__API_PASSPHRASE":    _dotenv.get("POLY_API_PASSPHRASE", ""),
    # Kalshi — demo URL for paper trading, production URL for live execution
    "KALSHI__REST_API_BASE_URL":      _dotenv.get("KAL_API_BASE_DEVELOPMENT", "https://external-api.demo.kalshi.co/trade-api/v2"),
    "KALSHI__REST_API_BASE_URL_LIVE": _dotenv.get("KAL_API_BASE_PRODUCTION", "https://external-api.kalshi.com/trade-api/v2"),
    "KALSHI__WEBSOCKET_BASE_URL":     _dotenv.get("KAL_API_WS", "wss://external-api-ws.demo.kalshi.co/trade-api/ws/v2"),
    "KALSHI__API_KEY":                _dotenv.get("KAL_API_KEY", ""),
    "KALSHI__API_SECRET":             _dotenv.get("KAL_API_SECRET", ""),
    "KALSHI__ENVIRONMENT":            "development",
    # Copy wallets
    "COPY_WALLETS":              _dotenv.get("COPY_WALLETS", ""),
    # Oddpool
    "ODDPOOL_API":               _dotenv.get("ODDPOOL_API", ""),
}

PROCESSES: list[subprocess.Popen] = []


def run(cmd: list[str], cwd: str | None = None, env: dict | None = None, **kwargs) -> subprocess.Popen:
    """Start a background process and track it for cleanup."""
    print(f"[START] {' '.join(cmd)}")
    proc = subprocess.Popen(
        cmd,
        cwd=cwd or PROJECT_ROOT,
        env=env or ENV,
        **kwargs,
    )
    PROCESSES.append(proc)
    return proc


def wait_for_postgres():
    """Verify Supabase/Postgres connection is reachable."""
    print("[WAIT] Checking Postgres connection...")
    psql_url = ENV["DATABASE_URL"].split("?")[0]
    for _ in range(10):
        result = subprocess.run(
            ["psql", psql_url, "-c", "SELECT 1"],
            capture_output=True,
        )
        if result.returncode == 0:
            print("[OK] Postgres is ready")
            return True
        time.sleep(2)
    print("[ERR] Postgres did not become ready in time")
    return False


def wait_for_redis():
    """Poll until Redis is accepting connections."""
    print("[WAIT] Waiting for Redis...")
    for _ in range(30):
        result = subprocess.run(
            ["redis-cli", "-u", ENV["REDIS_URL"], "ping"],
            capture_output=True,
        )
        if result.returncode == 0 and b"PONG" in result.stdout:
            print("[OK] Redis is ready")
            return True
        time.sleep(1)
    print("[ERR] Redis did not become ready in time")
    return False


def run_migrations():
    """Migrations run automatically on API startup via RUN_MIGRATIONS_ON_STARTUP."""
    print("[SKIP] Migrations will be applied by the API on startup")


def seed_database():
    """Seed the database with dev data."""
    seed_file = os.path.join(PROJECT_ROOT, "backend", "seeds", "dev_seed.sql")
    if os.path.exists(seed_file):
        print("[RUN] Seeding database...")
        psql_url = ENV["DATABASE_URL"].split("?")[0]
        result = subprocess.run(
            ["psql", psql_url, "-v", "ON_ERROR_STOP=1", "-f", seed_file],
            capture_output=True,
        )
        if result.returncode == 0:
            print("[OK] Database seeded")
        else:
            print(f"[WARN] Seed failed: {result.stderr.decode()}")
    else:
        print("[SKIP] No seed file found")


def shutdown(signum, frame):
    """Gracefully terminate all child processes."""
    print("\n[SHUTDOWN] Stopping all processes...")
    for proc in PROCESSES:
        try:
            proc.terminate()
        except ProcessLookupError:
            pass
    time.sleep(1)
    for proc in PROCESSES:
        if proc.poll() is None:
            proc.kill()
    sys.exit(0)


def main():
    parser = argparse.ArgumentParser(description="Arbitek dev orchestrator")
    parser.add_argument("--full", action="store_true", help="Start everything")
    parser.add_argument("--api", action="store_true", help="Start API only")
    parser.add_argument("--workers", action="store_true", help="Start workers only")
    parser.add_argument("--frontend", action="store_true", help="Start frontend dev server")
    parser.add_argument("--infra-only", action="store_true", help="Start only infrastructure")
    args = parser.parse_args()

    if not any([args.full, args.api, args.workers, args.frontend, args.infra_only]):
        args.infra_only = True

    signal.signal(signal.SIGINT, shutdown)
    signal.signal(signal.SIGTERM, shutdown)

    print(f"[INFO] Loaded .env from {PROJECT_ROOT}/.env")
    print(f"[INFO] DB  → {ENV['DATABASE_URL'].split('@')[-1]}")
    print(f"[INFO] Redis → {ENV['REDIS_URL'].split('@')[-1] if '@' in ENV['REDIS_URL'] else ENV['REDIS_URL']}")
    wallets = ENV.get("COPY_WALLETS", "")
    print(f"[INFO] Wallets → {len(wallets.split(',')) if wallets else 0} loaded")

    # -----------------------------------------------------------------------
    # Check infrastructure
    # -----------------------------------------------------------------------
    if not wait_for_postgres() or not wait_for_redis():
        sys.exit(1)

    run_migrations()
    seed_database()

    if args.infra_only:
        print("[INFO] Infrastructure ready. Press Ctrl+C to stop.")
        while True:
            time.sleep(1)

    # -----------------------------------------------------------------------
    # Build all Rust binaries once
    # -----------------------------------------------------------------------
    needs_build = args.full or args.api or args.workers
    if needs_build:
        bins = []
        if args.full or args.api:
            bins += ["--bin", "arbitek-api"]
        if args.full or args.workers:
            bins += ["--bin", "arbitek-worker"]
        print(f"[BUILD] Compiling: {' '.join(b for b in bins if not b.startswith('--'))}...")
        result = subprocess.run(
            ["cargo", "build"] + bins,
            cwd=BACKEND_DIR,
            env=ENV,
        )
        if result.returncode != 0:
            print("[ERR] Build failed")
            sys.exit(1)
        print("[OK] Build complete")

    BIN_DIR = os.path.join(BACKEND_DIR, "target", "debug")

    # -----------------------------------------------------------------------
    # Start API
    # -----------------------------------------------------------------------
    if args.full or args.api:
        print("[START] Starting API server...")
        run(
            [os.path.join(BIN_DIR, "arbitek-api")],
            cwd=BACKEND_DIR,
            env=ENV,
        )
        time.sleep(12)

    # -----------------------------------------------------------------------
    # Start workers
    # -----------------------------------------------------------------------
    if args.full or args.workers:
        worker_modes = [
            "oddpool-copy",        # Poly detection → Oddpool match → Kalshi demo execution
            "ingest-kalshi",       # Kalshi raw event ingestion
            "signal-normalize",    # Normalise any non-oddpool raw events
            "risk-evaluate",
            "execution-live",
            "reconcile",
        ]
        for mode in worker_modes:
            print(f"[START] Starting worker: {mode}")
            run(
                [os.path.join(BIN_DIR, "arbitek-worker")],
                cwd=BACKEND_DIR,
                env={**ENV, "WORKER__MODE": mode},
            )

    # -----------------------------------------------------------------------
    # Start frontend
    # -----------------------------------------------------------------------
    if args.full or args.frontend:
        print("[START] Starting frontend dev server...")
        run(
            ["pnpm", "run", "dev"],
            cwd=FRONTEND_DIR,
            env=os.environ,
        )

    print("[INFO] All services started. Press Ctrl+C to stop.")
    while True:
        time.sleep(1)


if __name__ == "__main__":
    main()
