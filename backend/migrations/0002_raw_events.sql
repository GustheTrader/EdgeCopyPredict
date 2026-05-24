-- Raw events table for ingestion pipeline
CREATE TABLE IF NOT EXISTS raw_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    platform TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'rest_api',
    external_event_id TEXT NOT NULL,
    raw_payload JSONB NOT NULL,
    detected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    status TEXT NOT NULL DEFAULT 'new'
);
CREATE INDEX IF NOT EXISTS idx_raw_events_platform ON raw_events(platform);
CREATE INDEX IF NOT EXISTS idx_raw_events_status ON raw_events(status);
CREATE INDEX IF NOT EXISTS idx_raw_events_detected_at ON raw_events(detected_at DESC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_raw_events_external_id ON raw_events(platform, external_event_id);
