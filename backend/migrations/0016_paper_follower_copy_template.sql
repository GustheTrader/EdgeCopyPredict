-- Refresh the paper follower copy config to the new default template without
-- modifying an already-applied migration.
UPDATE followers
SET risk_config = '{
  "buy_method": "percentage",
  "buy_percentage": 2,
  "copy_scale": 0.02,
  "slippage_pct": 5,
  "max_slippage_pct": 5,
  "min_trigger": 1,
  "max_trigger": null,
  "stop_loss_type": "percentage",
  "stop_loss_pct": 30,
  "take_profit_pct": 50,
  "trade_behavior": "buys_sells",
  "max_spend_total": 800,
  "max_spend_per_market": 50,
  "min_buy": 1,
  "max_buy": 20,
  "bankroll": 1000,
  "min_notional": 1,
  "max_notional": 20,
  "max_trade_bankroll_pct": 2,
  "mute_notifications": false
}'::jsonb,
    updated_at = NOW()
WHERE id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
