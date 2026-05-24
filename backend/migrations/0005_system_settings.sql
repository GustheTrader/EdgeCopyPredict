-- System settings key-value store

CREATE TABLE IF NOT EXISTS system_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed default settings
INSERT INTO system_settings (key, value) VALUES
    ('live_trading_enabled', 'false'),
    ('system_status', 'active')
ON CONFLICT (key) DO NOTHING;
