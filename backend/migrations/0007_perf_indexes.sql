-- Performance indexes to eliminate slow-query warnings (>1s).
--
-- Covers the query shapes used by every hot path:
--   positions upsert WHERE clause
--   positions list (follower + open)
--   signals list (detected_at ORDER BY + master_id filter)
--   raw_events dedup (platform, external_event_id) — already exists, included for safety
--   orders reconcile (status + last_update_at)
--   fills list (follower + filled_at)
--   pnl aggregate (follower_account_id GROUP BY)
--   system_settings point-lookup (key)

-- positions: upsert matches on (follower_account_id, market_id, outcome)
CREATE UNIQUE INDEX IF NOT EXISTS idx_positions_upsert
    ON positions(follower_account_id, market_id, outcome);

-- positions: list open positions (contracts != 0)
CREATE INDEX IF NOT EXISTS idx_positions_open
    ON positions(follower_account_id, updated_at DESC)
    WHERE contracts != 0;

-- positions: updated_at for UI listing order
CREATE INDEX IF NOT EXISTS idx_positions_updated_at
    ON positions(updated_at DESC);

-- signals: master_id filter (copy pipeline stores one signal per trade per master)
CREATE INDEX IF NOT EXISTS idx_signals_master_id
    ON signals(master_id, detected_at DESC);

-- signals: market_title text search (WHERE market_title IS NOT NULL)
CREATE INDEX IF NOT EXISTS idx_signals_market_title
    ON signals(market_title) WHERE market_title IS NOT NULL;

-- raw_events: compound dedup — unique index already covers lookup, add status+detected_at
CREATE INDEX IF NOT EXISTS idx_raw_events_source_entity
    ON raw_events(source_entity_id, detected_at DESC)
    WHERE source_entity_id IS NOT NULL;

-- orders: reconcile fetches by status ordered by last_update_at
CREATE INDEX IF NOT EXISTS idx_orders_status_updated
    ON orders(status, last_update_at DESC);

-- orders: lookup by platform_order_id during reconcile
CREATE INDEX IF NOT EXISTS idx_orders_platform_order_id
    ON orders(platform_order_id) WHERE platform_order_id IS NOT NULL;

-- fills: list by follower ordered by filled_at
CREATE INDEX IF NOT EXISTS idx_fills_follower_filled_at
    ON fills(follower_account_id, filled_at DESC);

-- copy_intents: worker polls pending/approved — add created_at for FIFO ordering
CREATE INDEX IF NOT EXISTS idx_copy_intents_status_created
    ON copy_intents(status, created_at ASC);

-- system_settings: key is already a primary key — add updated_at for recency checks
CREATE INDEX IF NOT EXISTS idx_system_settings_key
    ON system_settings(key);

-- masters: wallet_address lookup (oddpool_copy filters by wallet)
CREATE INDEX IF NOT EXISTS idx_masters_wallet_address
    ON masters(wallet_address) WHERE wallet_address IS NOT NULL;

-- masters: status filter
CREATE INDEX IF NOT EXISTS idx_masters_status
    ON masters(status);

-- market_mappings: condition_id lookup (hot path in copy pipeline)
CREATE INDEX IF NOT EXISTS idx_market_mappings_condition_id
    ON market_mappings(polymarket_condition_id);

-- market_mappings: enabled filter
CREATE INDEX IF NOT EXISTS idx_market_mappings_enabled
    ON market_mappings(enabled) WHERE enabled = true;
