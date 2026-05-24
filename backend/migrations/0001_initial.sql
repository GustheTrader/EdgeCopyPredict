CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Masters (signal sources)
CREATE TABLE IF NOT EXISTS masters (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    display_name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    wallet_address TEXT,
    master_weight NUMERIC(36, 18) NOT NULL DEFAULT 1.0,
    config JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Followers (copying accounts)
CREATE TABLE IF NOT EXISTS followers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    display_name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    wallet_address TEXT,
    base_currency TEXT NOT NULL DEFAULT 'USD',
    platform TEXT NOT NULL DEFAULT 'polymarket',
    risk_config JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Signals (detected master trades)
CREATE TABLE IF NOT EXISTS signals (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    platform TEXT NOT NULL,
    master_id UUID REFERENCES masters(id),
    market_id UUID,
    outcome TEXT NOT NULL,
    side TEXT NOT NULL,
    position_effect TEXT NOT NULL DEFAULT 'open',
    size_contracts NUMERIC(36, 18) NOT NULL DEFAULT 0,
    notional NUMERIC(36, 18) NOT NULL DEFAULT 0,
    currency TEXT NOT NULL DEFAULT 'USD',
    entry_price NUMERIC(36, 18) NOT NULL DEFAULT 0,
    event_timestamp TIMESTAMPTZ NOT NULL,
    detected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    confidence_score NUMERIC(36, 18),
    status TEXT NOT NULL DEFAULT 'new',
    source_event_ids JSONB,
    enrichment JSONB
);
CREATE INDEX IF NOT EXISTS idx_signals_detected_at ON signals(detected_at DESC);
CREATE INDEX IF NOT EXISTS idx_signals_status ON signals(status);
CREATE INDEX IF NOT EXISTS idx_signals_platform ON signals(platform);

-- Copy intents (approved signals ready for execution)
CREATE TABLE IF NOT EXISTS copy_intents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    follower_account_id UUID NOT NULL REFERENCES followers(id),
    platform TEXT NOT NULL,
    signal_platform_market_id TEXT NOT NULL,
    market_id UUID,
    outcome TEXT NOT NULL,
    action TEXT NOT NULL,
    target_contracts NUMERIC(36, 18) NOT NULL DEFAULT 0,
    price_limit NUMERIC(36, 18) NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status TEXT NOT NULL DEFAULT 'approved'
);
CREATE INDEX IF NOT EXISTS idx_copy_intents_status ON copy_intents(status);

-- Orders (submitted to platforms)
CREATE TABLE IF NOT EXISTS orders (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    copy_intent_id UUID REFERENCES copy_intents(id),
    platform TEXT NOT NULL,
    follower_account_id UUID NOT NULL REFERENCES followers(id),
    platform_order_id TEXT,
    client_order_id TEXT NOT NULL,
    market_id UUID,
    outcome TEXT NOT NULL,
    side TEXT NOT NULL,
    order_type TEXT NOT NULL DEFAULT 'market',
    price_limit NUMERIC(36, 18) NOT NULL DEFAULT 0,
    requested_contracts NUMERIC(36, 18) NOT NULL DEFAULT 0,
    filled_contracts NUMERIC(36, 18) NOT NULL DEFAULT 0,
    avg_fill_price NUMERIC(36, 18),
    status TEXT NOT NULL DEFAULT 'pending',
    submitted_at TIMESTAMPTZ,
    last_update_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status);
CREATE INDEX IF NOT EXISTS idx_orders_follower ON orders(follower_account_id);

-- Fills (execution reports from platforms)
CREATE TABLE IF NOT EXISTS fills (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    order_id UUID NOT NULL REFERENCES orders(id),
    platform TEXT NOT NULL,
    platform_fill_id TEXT NOT NULL,
    follower_account_id UUID NOT NULL REFERENCES followers(id),
    market_id UUID,
    market_title TEXT,
    outcome TEXT NOT NULL,
    side TEXT NOT NULL,
    price NUMERIC(36, 18) NOT NULL DEFAULT 0,
    contracts NUMERIC(36, 18) NOT NULL DEFAULT 0,
    notional NUMERIC(36, 18) NOT NULL DEFAULT 0,
    fee NUMERIC(36, 18) NOT NULL DEFAULT 0,
    is_taker BOOLEAN,
    filled_at TIMESTAMPTZ NOT NULL,
    payload JSONB
);
CREATE INDEX IF NOT EXISTS idx_fills_order_id ON fills(order_id);
CREATE INDEX IF NOT EXISTS idx_fills_filled_at ON fills(filled_at DESC);

-- Positions (current + historical)
CREATE TABLE IF NOT EXISTS positions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    follower_account_id UUID NOT NULL REFERENCES followers(id),
    platform TEXT NOT NULL,
    market_id UUID NOT NULL,
    market_title TEXT,
    market_status TEXT,
    close_time TIMESTAMPTZ,
    outcome TEXT NOT NULL,
    contracts NUMERIC(36, 18) NOT NULL DEFAULT 0,
    avg_price NUMERIC(36, 18) NOT NULL DEFAULT 0,
    realized_pnl NUMERIC(36, 18) NOT NULL DEFAULT 0,
    unrealized_pnl NUMERIC(36, 18) NOT NULL DEFAULT 0,
    exposure NUMERIC(36, 18) NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_positions_follower ON positions(follower_account_id);
CREATE INDEX IF NOT EXISTS idx_positions_market ON positions(market_id);

-- Alerts (system + risk alerts)
CREATE TABLE IF NOT EXISTS alerts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    severity TEXT NOT NULL DEFAULT 'info',
    type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',
    entity_type TEXT,
    entity_id UUID,
    message TEXT NOT NULL,
    payload JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    acknowledged_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_alerts_status ON alerts(status);
CREATE INDEX IF NOT EXISTS idx_alerts_created_at ON alerts(created_at DESC);

-- Market mappings (polymarket <-> kalshi)
CREATE TABLE IF NOT EXISTS market_mappings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    polymarket_condition_id TEXT NOT NULL UNIQUE,
    polymarket_slug TEXT,
    kalshi_ticker TEXT NOT NULL,
    kalshi_event_ticker TEXT,
    confidence TEXT NOT NULL DEFAULT 'manual',
    enabled BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
