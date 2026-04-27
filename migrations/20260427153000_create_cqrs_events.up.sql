CREATE TABLE IF NOT EXISTS cqrs_events (
    id TEXT PRIMARY KEY,
    aggregate_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    version BIGINT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_cqrs_events_aggregate_version
    ON cqrs_events (aggregate_id, version);

CREATE INDEX IF NOT EXISTS idx_cqrs_events_aggregate_id
    ON cqrs_events (aggregate_id);
