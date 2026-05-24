-- Reset all paper-trading data to a clean $1 000 slate.
--
-- Paper trades are identified by the sentinel follower account UUID
-- 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', and by raw_events with
-- source = 'paper_copy'.
--
-- Run this once to discard the stale data that was created while the wallet
-- balance bug was active (sells were subtracting proceeds instead of adding
-- them, so balances were permanently deflated and positions were skewed).

-- 1. Reset the paper wallet to $1 000.
INSERT INTO system_settings (key, value, updated_at)
VALUES ('paper_wallet_balance', '1000', NOW())
ON CONFLICT (key) DO UPDATE SET value = '1000', updated_at = NOW();

-- 2. Ensure Kalshi live execution is OFF (paper mode is the safe default).
INSERT INTO system_settings (key, value, updated_at)
VALUES ('kalshi_execution_enabled', 'false', NOW())
ON CONFLICT (key) DO UPDATE SET value = 'false', updated_at = NOW();

-- 3. Delete positions belonging to the paper follower account.
DELETE FROM positions
WHERE follower_account_id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';

-- 4. Delete raw events that were written by the paper-copy worker.
DELETE FROM raw_events
WHERE source = 'paper_copy';

-- 5. Delete signals that were created from paper trades.
--    (They reference master_ids but have no follower; deleting raw_events is
--    usually enough, but this clears the signals table too.)
DELETE FROM signals
WHERE platform = 'polymarket'
  AND status IN ('approved', 'rejected')
  AND id NOT IN (
      SELECT DISTINCT signal_id
      FROM copy_intents
      WHERE signal_id IS NOT NULL
  );

-- 6. Acknowledge / clear any open alerts that came from the broken trades.
--    (Optional — comment this out if you want to keep the alert history.)
-- UPDATE alerts SET status = 'acknowledged', acknowledged_at = NOW()
-- WHERE status = 'open' AND alert_type IN ('execution_failure', 'risk_rejection');
