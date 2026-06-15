-- Replace the old multi-parameter follower risk config with the simple mirror
-- copy setup. Runtime code still accepts old keys for backwards compatibility.
UPDATE followers
SET risk_config = jsonb_build_object(
    'copy_mode', 'mirror',
    'sizing_mode', 'proportional_bankroll',
    'follower_bankroll', COALESCE(risk_config->'follower_bankroll', risk_config->'bankroll', '1000'::jsonb),
    'master_bankroll', COALESCE(risk_config->'master_bankroll', '50000'::jsonb)
),
updated_at = NOW()
WHERE status <> 'deleted';
