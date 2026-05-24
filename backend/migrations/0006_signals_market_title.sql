-- Add market_title column to signals for human-readable market/pair name
ALTER TABLE signals ADD COLUMN IF NOT EXISTS market_title TEXT;

-- Backfill market_title from raw event payloads where possible
UPDATE signals s
SET market_title = raw.raw_payload->>'title'
FROM raw_events raw
WHERE raw.source_entity_id = s.master_id
  AND raw.raw_payload->>'title' IS NOT NULL
  AND s.market_title IS NULL
  AND raw.platform = s.platform;
