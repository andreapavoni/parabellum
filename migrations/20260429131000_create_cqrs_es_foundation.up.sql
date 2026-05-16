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
CREATE TYPE movement_type AS ENUM ('Attack', 'Raid', 'Scout', 'Reinforcement', 'Return', 'FoundVillage');
CREATE TYPE scheduled_action_status AS ENUM ('pending', 'processing', 'completed', 'failed');
CREATE TYPE scheduled_action_type AS ENUM (
    'ReinforcementArrival',
    'SettlersArrival',
    'AttackArrival',
    'ArmyReturn',
    'ScoutArrival',
    'MerchantArrival',
    'MerchantReturn',
    'AddBuilding',
    'UpgradeBuilding',
    'DowngradeBuilding',
    'TrainUnit',
    'ResearchAcademy',
    'ResearchSmithy',
    'HeroRevival'
);
CREATE TYPE rm_marketplace_offer_status AS ENUM ('open', 'accepted', 'canceled');

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
    culture_points_production INTEGER NOT NULL DEFAULT 0,
    smithy_upgrades JSONB NOT NULL DEFAULT '{}'::jsonb,
    academy_research JSONB NOT NULL DEFAULT '{"researches":{}}'::jsonb,
    parent_village_id INTEGER NULL,
    army JSONB NOT NULL DEFAULT 'null'::jsonb,
    reinforcements JSONB NOT NULL DEFAULT '[]'::jsonb,
    deployed_armies JSONB NOT NULL DEFAULT '[]'::jsonb,
    total_merchants SMALLINT NOT NULL DEFAULT 0,
    busy_merchants SMALLINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE rm_village_movements (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    village_id INTEGER NOT NULL REFERENCES rm_village(village_id) ON DELETE CASCADE,
    movement_id UUID NOT NULL,
    direction movement_direction NOT NULL,
    movement_type movement_type NOT NULL,
    source_village_id INTEGER NOT NULL REFERENCES rm_village(village_id) ON DELETE CASCADE,
    target_village_id INTEGER NOT NULL REFERENCES rm_village(village_id) ON DELETE CASCADE,
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

CREATE TABLE rm_marketplace_offers (
    offer_id UUID PRIMARY KEY,
    owner_player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    owner_village_id INTEGER NOT NULL REFERENCES rm_village(village_id) ON DELETE CASCADE,
    offer_resources JSONB NOT NULL,
    seek_resources JSONB NOT NULL,
    merchants_reserved SMALLINT NOT NULL,
    status rm_marketplace_offer_status NOT NULL,
    accepted_by_player_id UUID NULL REFERENCES players(id) ON DELETE SET NULL,
    accepted_by_village_id INTEGER NULL REFERENCES rm_village(village_id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    accepted_at TIMESTAMPTZ NULL,
    canceled_at TIMESTAMPTZ NULL
);

CREATE INDEX idx_rm_marketplace_offers_owner_village
    ON rm_marketplace_offers (owner_village_id);
CREATE INDEX idx_rm_marketplace_offers_status
    ON rm_marketplace_offers (status, created_at DESC);

CREATE TABLE rm_reports (
    id UUID PRIMARY KEY,
    report_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    actor_player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    actor_village_id INTEGER NULL REFERENCES rm_village(village_id) ON DELETE SET NULL,
    target_player_id UUID NULL REFERENCES players(id) ON DELETE SET NULL,
    target_village_id INTEGER NULL REFERENCES rm_village(village_id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE rm_report_reads (
    report_id UUID NOT NULL REFERENCES rm_reports(id) ON DELETE CASCADE,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    read_at TIMESTAMPTZ NULL,
    PRIMARY KEY (report_id, player_id)
);

CREATE INDEX idx_rm_report_reads_player_created
    ON rm_report_reads (player_id, read_at);
CREATE INDEX idx_rm_reports_created_at
    ON rm_reports (created_at DESC);

CREATE TABLE rm_armies (
    army_id UUID PRIMARY KEY,
    village_id INTEGER NOT NULL REFERENCES rm_village(village_id) ON DELETE CASCADE,
    current_village_id INTEGER NOT NULL REFERENCES rm_village(village_id) ON DELETE CASCADE,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    state TEXT NOT NULL CHECK (state IN ('home', 'stationed', 'moving')),
    payload JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_rm_armies_village_id ON rm_armies(village_id);
CREATE INDEX idx_rm_armies_current_village_id ON rm_armies(current_village_id);
CREATE INDEX idx_rm_armies_player_id ON rm_armies(player_id);
CREATE INDEX idx_rm_armies_state ON rm_armies(state);

CREATE TABLE rm_heroes (
    hero_id UUID PRIMARY KEY,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    home_village_id INTEGER NOT NULL REFERENCES rm_village(village_id) ON DELETE CASCADE,
    current_village_id INTEGER NOT NULL REFERENCES rm_village(village_id) ON DELETE CASCADE,
    state TEXT NOT NULL CHECK (state IN ('home', 'stationed', 'moving')),
    tribe tribe NOT NULL,
    level SMALLINT NOT NULL,
    health SMALLINT NOT NULL,
    experience INTEGER NOT NULL,
    resource_focus JSONB NOT NULL,
    strength_points SMALLINT NOT NULL,
    off_bonus_points SMALLINT NOT NULL,
    def_bonus_points SMALLINT NOT NULL,
    regeneration_points SMALLINT NOT NULL,
    resources_points SMALLINT NOT NULL,
    unassigned_points SMALLINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_rm_heroes_player_id ON rm_heroes(player_id);
CREATE INDEX idx_rm_heroes_home_village_id ON rm_heroes(home_village_id);
CREATE INDEX idx_rm_heroes_current_village_id ON rm_heroes(current_village_id);
CREATE INDEX idx_rm_heroes_state ON rm_heroes(state);

CREATE TABLE rm_map_fields (
    id INTEGER PRIMARY KEY,
    village_id INTEGER NULL REFERENCES rm_village(village_id) ON DELETE SET NULL,
    player_id UUID NULL REFERENCES players(id) ON DELETE SET NULL,
    position JSONB NOT NULL,
    topology JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_rm_map_fields_village_id ON rm_map_fields (village_id);
