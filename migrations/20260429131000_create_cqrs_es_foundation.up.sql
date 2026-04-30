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

CREATE TYPE movement_direction AS ENUM ('Incoming', 'Outgoing');
CREATE TYPE movement_type AS ENUM ('Attack', 'Raid', 'Reinforcement', 'Return', 'FoundVillage');
CREATE TYPE scheduled_action_status AS ENUM ('pending', 'processing', 'completed', 'failed');
CREATE TYPE scheduled_action_type AS ENUM ('ReinforcementArrival', 'AddBuilding', 'UpgradeBuilding', 'DowngradeBuilding', 'TrainUnit');

CREATE TABLE rm_village (
    village_id INTEGER PRIMARY KEY,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    village_name TEXT NOT NULL,
    position JSONB NOT NULL,
    tribe tribe NOT NULL,
    buildings JSONB NOT NULL DEFAULT '[]'::jsonb,
    production JSONB NOT NULL DEFAULT '{}'::jsonb,
    stocks JSONB NOT NULL DEFAULT '{}'::jsonb,
    population INTEGER NOT NULL DEFAULT 2,
    loyalty SMALLINT NOT NULL DEFAULT 100 CHECK (loyalty >= 0 AND loyalty <= 100),
    is_capital BOOLEAN NOT NULL DEFAULT FALSE,
    culture_points INTEGER NOT NULL DEFAULT 0,
    culture_points_production INTEGER NOT NULL DEFAULT 0,
    parent_village_id INTEGER NULL,
    stationed_army JSONB NOT NULL DEFAULT '{}'::jsonb,
    total_merchants SMALLINT NOT NULL DEFAULT 0,
    busy_merchants SMALLINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE rm_village_movements (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    village_id INTEGER NOT NULL REFERENCES villages(id) ON DELETE CASCADE,
    movement_id UUID NOT NULL,
    direction movement_direction NOT NULL,
    movement_type movement_type NOT NULL,
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
    action_type scheduled_action_type NOT NULL,
    execute_at TIMESTAMPTZ NOT NULL,
    payload JSONB NOT NULL,
    status scheduled_action_status NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_rm_scheduled_actions_execute_at
    ON rm_scheduled_actions (status, execute_at);
