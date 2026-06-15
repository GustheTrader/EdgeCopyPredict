-- Seed the recommended copy-trading configuration onto every follower's risk_config.
--
-- The defaults are merged with `defaults || risk_config` so that ANY value already
-- present on a follower wins (existing bankroll / copy_scale / notional limits are
-- preserved) and only the new keys are filled in. Odds are stored as fractions (0..1)
-- to match the master entry price (implied probability). Stop-loss / take-profit and
-- the category/liquidity/volume filters are stored now but only enforced in Phase 2.
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
}'::jsonb || risk_config;
