-- Add signal_id FK to copy_intents so list_for_signal and future queries work.
ALTER TABLE copy_intents
    ADD COLUMN IF NOT EXISTS signal_id UUID REFERENCES signals(id);

-- Seed kalshi_execution_enabled (migration 0008 never ran this because it failed).
INSERT INTO system_settings (key, value, updated_at)
VALUES ('kalshi_execution_enabled', 'false', NOW())
ON CONFLICT (key) DO NOTHING;

-- Redo the paper-signal cleanup that migration 0008 attempted but could not run.
-- Paper signals are created by oddpool_copy with status 'approved'/'rejected'.
-- Real signals always start as 'new', so this correctly targets only paper data.
DELETE FROM signals WHERE status IN ('approved', 'rejected');

-- Index for the list_for_signal lookup.
CREATE INDEX IF NOT EXISTS idx_copy_intents_signal_id
    ON copy_intents(signal_id) WHERE signal_id IS NOT NULL;
