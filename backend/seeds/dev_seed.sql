-- Seed data for local development
-- NOTE: Master wallets are seeded automatically at API startup from the
--       COPY_WALLETS env var. Do NOT add fake placeholder wallets here
--       or signals will be fetched from the wrong addresses.

INSERT INTO followers (id, display_name, status, base_currency, wallet_address, platform, risk_config) VALUES
    ('aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'Kalshi Paper Slave', 'active', 'USD', '0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 'kalshi', '{"bankroll": 1000, "copy_scale": 1.0, "max_notional": 500, "min_notional": 0, "max_trade_bankroll_pct": 50}'),
    ('bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', 'Demo Follower 2', 'active', 'USD', '0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb', 'polymarket', '{"bankroll": 5000, "copy_scale": 0.3, "max_notional": 300}')
ON CONFLICT DO NOTHING;

INSERT INTO market_mappings (id, polymarket_condition_id, polymarket_slug, kalshi_ticker, kalshi_event_ticker, confidence, enabled) VALUES
    ('cccccccc-cccc-cccc-cccc-cccccccccccc', '0xbtc_condition_placeholder', 'btc-updown-5m', 'BTC-5MIN', 'BTC-5MIN-EVENT', 'manual', false),
    ('dddddddd-dddd-dddd-dddd-dddddddddddd', '0xeth_condition_placeholder', 'eth-updown-5m', 'ETH-5MIN', 'ETH-5MIN-EVENT', 'manual', false)
ON CONFLICT DO NOTHING;
