-- Durable platform market identifier (Polymarket condition_id / Kalshi ticker) on
-- positions. Previously the settlement worker recovered the condition_id by LIKE-
-- matching the position's market_id UUID prefix against raw_events. That lookup
-- silently failed whenever the originating raw_event had been pruned, leaving the
-- position open forever even after its market resolved. Storing the identifier
-- directly makes settlement self-sufficient and the lookup deterministic.
ALTER TABLE positions
    ADD COLUMN IF NOT EXISTS platform_market_id TEXT;

-- One-time backfill for existing rows: recover the condition_id from raw_events
-- using the same market_id-hex-prefix match the worker used. market_id::text is a
-- dashed lowercase UUID; stripping dashes yields the first 32 hex chars (16 bytes)
-- of the 0x-prefixed 64-hex condition_id, so LIKE '0x{32hex}%' matches it.
UPDATE positions p
SET platform_market_id = (
    SELECT COALESCE(
               r.raw_payload->>'condition_id',
               r.raw_payload->'raw'->>'conditionId'
           )
    FROM raw_events r
    WHERE LOWER(COALESCE(
              r.raw_payload->>'condition_id',
              r.raw_payload->'raw'->>'conditionId'
          )) LIKE '0x' || REPLACE(LOWER(p.market_id::text), '-', '') || '%'
    LIMIT 1
)
WHERE p.platform = 'polymarket'
  AND p.platform_market_id IS NULL;
