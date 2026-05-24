-- Add source entity tracking to raw events

ALTER TABLE raw_events
    ADD COLUMN IF NOT EXISTS source_entity_id UUID,
    ADD COLUMN IF NOT EXISTS source_entity_type TEXT;
