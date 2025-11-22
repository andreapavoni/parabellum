-- Alliance System Migration
-- Consolidates all alliance-related tables and player fields with proper TIMESTAMPTZ support

-- Create function to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Medal period type enum
CREATE TYPE medal_period_type AS ENUM ('Hour', 'Day', 'Week');

-- Medal category type enum
CREATE TYPE medal_category AS ENUM ('Attack', 'Defense', 'Climbers', 'Robber');

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
    total_robber_points BIGINT DEFAULT 0,
    total_climber_points BIGINT DEFAULT 0,
    current_attack_points BIGINT DEFAULT 0,
    current_defense_points BIGINT DEFAULT 0,
    current_robber_points BIGINT DEFAULT 0,
    current_climber_points BIGINT DEFAULT 0,

    -- Alliance Bonuses
    recruitment_bonus_level INTEGER DEFAULT 0,
    recruitment_bonus_contributions BIGINT DEFAULT 0,
    metallurgy_bonus_level INTEGER DEFAULT 0,
    metallurgy_bonus_contributions BIGINT DEFAULT 0,
    philosophy_bonus_level INTEGER DEFAULT 0,
    philosophy_bonus_contributions BIGINT DEFAULT 0,
    commerce_bonus_level INTEGER DEFAULT 0,
    commerce_bonus_contributions BIGINT DEFAULT 0
);

-- Alliance invitations
CREATE TABLE alliance_invite (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    from_player_id UUID NOT NULL,
    alliance_id UUID NOT NULL REFERENCES alliance(id) ON DELETE CASCADE,
    to_player_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(alliance_id, to_player_id)
);

-- Alliance event log
CREATE TABLE alliance_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alliance_id UUID NOT NULL REFERENCES alliance(id) ON DELETE CASCADE,
    type SMALLINT NOT NULL,
    data TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_alliance_log_alliance_id ON alliance_log(alliance_id);

-- Alliance diplomacy relationships
CREATE TABLE alliance_diplomacy (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alliance1_id UUID NOT NULL REFERENCES alliance(id) ON DELETE CASCADE,
    alliance2_id UUID NOT NULL REFERENCES alliance(id) ON DELETE CASCADE,
    type SMALLINT NOT NULL,
    accepted SMALLINT DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(alliance1_id, alliance2_id)
);

-- Create trigger for alliance_diplomacy
CREATE TRIGGER update_alliance_diplomacy_updated_at
    BEFORE UPDATE ON alliance_diplomacy
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Alliance medals (achievements for different time periods)
CREATE TABLE alliance_medal (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alliance_id UUID NOT NULL REFERENCES alliance(id) ON DELETE CASCADE,
    period_type medal_period_type NOT NULL,
    period_number INTEGER NOT NULL,
    rank INTEGER NOT NULL,
    category medal_category NOT NULL,
    points INTEGER DEFAULT 1, -- Number of medals
    UNIQUE(alliance_id, period_type, period_number, category)
);

CREATE INDEX idx_alliance_medal_alliance_id ON alliance_medal(alliance_id);
CREATE INDEX idx_alliance_medal_period ON alliance_medal(period_type, period_number);

-- Flag type enum for map flags
CREATE TYPE flag_type_enum AS ENUM ('PlayerMark', 'AllianceMark', 'CustomFlag');

-- Map flags/marks (unified table for both player and alliance marks)
--   Type 'PlayerMark': Player marks (track specific players)
--   Type 'AllianceMark': Alliance marks (track entire alliances)
--   Type 'CustomFlag': Custom flags (labeled markers on map tiles)
CREATE TABLE map_flag (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alliance_id UUID REFERENCES alliance(id) ON DELETE CASCADE,
    player_id UUID REFERENCES players(id) ON DELETE CASCADE,
    target_id UUID,
    position JSONB,
    flag_type flag_type_enum NOT NULL,
    color SMALLINT NOT NULL,
    text VARCHAR(50),             
    created_by UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_ownership CHECK (
        (alliance_id IS NOT NULL AND player_id IS NULL) OR
        (alliance_id IS NULL AND player_id IS NOT NULL)
    ),
    CONSTRAINT chk_target_or_position CHECK (
        (flag_type IN ('PlayerMark', 'AllianceMark') AND target_id IS NOT NULL) OR
        (flag_type = 'CustomFlag' AND position IS NOT NULL)
    )
);

CREATE INDEX idx_map_flag_alliance_id ON map_flag(alliance_id) WHERE alliance_id IS NOT NULL;
CREATE INDEX idx_map_flag_player_id ON map_flag(player_id) WHERE player_id IS NOT NULL;
CREATE INDEX idx_map_flag_target_id ON map_flag(target_id) WHERE target_id IS NOT NULL;
CREATE INDEX idx_map_flag_position ON map_flag USING gin(position) WHERE position IS NOT NULL;
CREATE INDEX idx_map_flag_type ON map_flag(flag_type);

-- Create trigger for map_flag
CREATE TRIGGER update_map_flag_updated_at
    BEFORE UPDATE ON map_flag
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Add alliance fields to players table
ALTER TABLE players ADD COLUMN alliance_id UUID REFERENCES alliance(id) ON DELETE SET NULL;
ALTER TABLE players ADD COLUMN alliance_role INTEGER;
ALTER TABLE players ADD COLUMN alliance_join_time TIMESTAMPTZ DEFAULT NOW();

-- Current Week Contributions
ALTER TABLE players ADD COLUMN current_alliance_recruitment_contributions BIGINT DEFAULT 0;
ALTER TABLE players ADD COLUMN current_alliance_metallurgy_contributions BIGINT DEFAULT 0;
ALTER TABLE players ADD COLUMN current_alliance_philosophy_contributions BIGINT DEFAULT 0;
ALTER TABLE players ADD COLUMN current_alliance_commerce_contributions BIGINT DEFAULT 0;

-- Total Contributions
ALTER TABLE players ADD COLUMN total_alliance_recruitment_contributions BIGINT DEFAULT 0;
ALTER TABLE players ADD COLUMN total_alliance_metallurgy_contributions BIGINT DEFAULT 0;
ALTER TABLE players ADD COLUMN total_alliance_philosophy_contributions BIGINT DEFAULT 0;
ALTER TABLE players ADD COLUMN total_alliance_commerce_contributions BIGINT DEFAULT 0;

-- Add created_at timestamp to players table
ALTER TABLE players ADD COLUMN created_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Add foreign key constraint for alliance leader (references players)
-- Must be done after players table is modified
ALTER TABLE alliance ADD CONSTRAINT fk_alliance_leader
    FOREIGN KEY (leader_id) REFERENCES players(id) ON DELETE SET NULL;

CREATE INDEX idx_alliance_leader_id ON alliance(leader_id);
