-- Alliance System Migration
-- Consolidates all alliance-related tables and player fields

-- Medal period type enum
CREATE TYPE medal_period_type AS ENUM ('Hour', 'Day', 'Week');

-- Main alliance table
CREATE TABLE alliance (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) UNIQUE NOT NULL,
    tag VARCHAR(10) UNIQUE NOT NULL,
    desc1 TEXT,
    desc2 TEXT,
    info1 TEXT,
    info2 TEXT,
    forum_link VARCHAR(255),
    max_members INTEGER NOT NULL DEFAULT 3,
    leader_id UUID,

    -- Battle Statistics
    total_attack_points BIGINT DEFAULT 0,
    total_defense_points BIGINT DEFAULT 0,
    current_attack_points BIGINT DEFAULT 0,
    current_defense_points BIGINT DEFAULT 0,
    current_robber_points BIGINT DEFAULT 0,

    -- Alliance Bonuses
    training_bonus_level INTEGER DEFAULT 0,
    training_bonus_contributions BIGINT DEFAULT 0,
    armor_bonus_level INTEGER DEFAULT 0,
    armor_bonus_contributions BIGINT DEFAULT 0,
    cp_bonus_level INTEGER DEFAULT 0,
    cp_bonus_contributions BIGINT DEFAULT 0,
    trade_bonus_level INTEGER DEFAULT 0,
    trade_bonus_contributions BIGINT DEFAULT 0,

    old_pop INTEGER DEFAULT 0
);

-- Alliance invitations
CREATE TABLE alliance_invite (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    from_player_id UUID NOT NULL,
    alliance_id UUID NOT NULL REFERENCES alliance(id) ON DELETE CASCADE,
    to_player_id UUID NOT NULL,
    UNIQUE(alliance_id, to_player_id)
);

-- Alliance event log
CREATE TABLE alliance_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alliance_id UUID NOT NULL REFERENCES alliance(id) ON DELETE CASCADE,
    type SMALLINT NOT NULL,
    data TEXT,
    time INTEGER NOT NULL
);

CREATE INDEX idx_alliance_log_alliance_id ON alliance_log(alliance_id);

-- Alliance diplomacy relationships
CREATE TABLE alliance_diplomacy (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alliance1_id UUID NOT NULL REFERENCES alliance(id) ON DELETE CASCADE,
    alliance2_id UUID NOT NULL REFERENCES alliance(id) ON DELETE CASCADE,
    type SMALLINT NOT NULL,
    accepted SMALLINT DEFAULT 0,
    UNIQUE(alliance1_id, alliance2_id)
);

-- Alliance medals (achievements for different time periods)
CREATE TABLE alliance_medal (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alliance_id UUID NOT NULL REFERENCES alliance(id) ON DELETE CASCADE,
    period_type medal_period_type NOT NULL,
    period_number INTEGER NOT NULL,
    rank INTEGER NOT NULL,
    category SMALLINT NOT NULL,
    count INTEGER DEFAULT 1,
    UNIQUE(alliance_id, period_type, period_number, category)
);

CREATE INDEX idx_alliance_medal_alliance_id ON alliance_medal(alliance_id);
CREATE INDEX idx_alliance_medal_period ON alliance_medal(period_type, period_number);

-- Alliance notifications
CREATE TABLE alliance_notification (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alliance_id UUID NOT NULL REFERENCES alliance(id) ON DELETE CASCADE,
    player_id UUID NOT NULL,
    type SMALLINT NOT NULL,
    data TEXT,
    time INTEGER NOT NULL,
    read BOOLEAN DEFAULT FALSE
);

CREATE INDEX idx_alliance_notification_player ON alliance_notification(player_id);

-- Alliance bonus upgrade queue
CREATE TABLE alliance_bonus_upgrade_queue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alliance_id UUID NOT NULL REFERENCES alliance(id) ON DELETE CASCADE,
    bonus_type SMALLINT NOT NULL,
    finish_time INTEGER NOT NULL
);

CREATE INDEX idx_alliance_bonus_queue_alliance_id ON alliance_bonus_upgrade_queue(alliance_id);

-- Alliance map flags/marks
CREATE TABLE alliance_map_flag (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alliance_id UUID NOT NULL REFERENCES alliance(id) ON DELETE CASCADE,
    x INTEGER NOT NULL,
    y INTEGER NOT NULL,
    type SMALLINT NOT NULL,
    description TEXT,
    created_by UUID NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX idx_alliance_map_flag_alliance_id ON alliance_map_flag(alliance_id);

-- Add alliance fields to players table
ALTER TABLE players ADD COLUMN alliance_id UUID REFERENCES alliance(id) ON DELETE SET NULL;
ALTER TABLE players ADD COLUMN alliance_role_name VARCHAR(255);
ALTER TABLE players ADD COLUMN alliance_role INTEGER;
ALTER TABLE players ADD COLUMN alliance_join_time INTEGER;
ALTER TABLE players ADD COLUMN alliance_contributions BIGINT DEFAULT 0;

-- Current Week Contributions
ALTER TABLE players ADD COLUMN current_alliance_training_contributions BIGINT DEFAULT 0;
ALTER TABLE players ADD COLUMN current_alliance_armor_contributions BIGINT DEFAULT 0;
ALTER TABLE players ADD COLUMN current_alliance_cp_contributions BIGINT DEFAULT 0;
ALTER TABLE players ADD COLUMN current_alliance_trade_contributions BIGINT DEFAULT 0;

-- Total Contributions
ALTER TABLE players ADD COLUMN total_alliance_training_contributions BIGINT DEFAULT 0;
ALTER TABLE players ADD COLUMN total_alliance_armor_contributions BIGINT DEFAULT 0;
ALTER TABLE players ADD COLUMN total_alliance_cp_contributions BIGINT DEFAULT 0;
ALTER TABLE players ADD COLUMN total_alliance_trade_contributions BIGINT DEFAULT 0;

-- Alliance Preferences
ALTER TABLE players ADD COLUMN alliance_notification_enabled BOOLEAN DEFAULT TRUE;
ALTER TABLE players ADD COLUMN alliance_settings TEXT;

-- Add created_at timestamp to players table
ALTER TABLE players ADD COLUMN created_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Add foreign key constraint for alliance leader (references players)
-- Must be done after players table is modified
ALTER TABLE alliance ADD CONSTRAINT fk_alliance_leader
    FOREIGN KEY (leader_id) REFERENCES players(id) ON DELETE SET NULL;

CREATE INDEX idx_alliance_leader_id ON alliance(leader_id);
