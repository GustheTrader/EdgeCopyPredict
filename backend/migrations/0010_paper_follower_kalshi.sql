-- Update the paper follower to Kalshi platform so the paper execution
-- worker receives cross-platform copy intents (Polymarket signal → Kalshi paper trade).
UPDATE followers
SET platform     = 'kalshi',
    display_name = 'Kalshi Paper Slave',
    risk_config  = '{"bankroll": 1000, "copy_scale": 1.0, "max_notional": 500, "min_notional": 0, "max_trade_bankroll_pct": 50}'::jsonb,
    updated_at   = NOW()
WHERE id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
