CREATE TABLE es_events (
    global_seq BIGSERIAL PRIMARY KEY,
    event_id TEXT NOT NULL UNIQUE,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    stream_version BIGINT NOT NULL,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    metadata JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (aggregate_type, aggregate_id, stream_version)
);

CREATE INDEX idx_es_events_aggregate ON es_events (aggregate_type, aggregate_id, stream_version);
CREATE INDEX idx_es_events_event_type ON es_events (event_type);

CREATE TABLE es_snapshots (
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    stream_version BIGINT NOT NULL,
    state JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (aggregate_type, aggregate_id)
);

CREATE TABLE es_projector_offsets (
    projector_name TEXT PRIMARY KEY,
    last_global_seq BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE rm_village_overview (
    village_id INTEGER PRIMARY KEY,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    village_name TEXT NOT NULL,
    position JSONB NOT NULL,
    resources JSONB NOT NULL DEFAULT '{}'::jsonb,
    stationed_army JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE rm_village_movements (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    village_id INTEGER NOT NULL REFERENCES villages(id) ON DELETE CASCADE,
    movement_id UUID NOT NULL,
    direction TEXT NOT NULL,
    movement_type TEXT NOT NULL,
    source_village_id INTEGER NOT NULL REFERENCES villages(id) ON DELETE CASCADE,
    target_village_id INTEGER NOT NULL REFERENCES villages(id) ON DELETE CASCADE,
    eta TIMESTAMPTZ NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (village_id, movement_id, direction)
);

CREATE INDEX idx_rm_village_movements_village_eta
    ON rm_village_movements (village_id, eta);

CREATE TABLE rm_scheduled_actions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    action_type TEXT NOT NULL,
    execute_at TIMESTAMPTZ NOT NULL,
    payload JSONB NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_rm_scheduled_actions_execute_at
    ON rm_scheduled_actions (status, execute_at);
