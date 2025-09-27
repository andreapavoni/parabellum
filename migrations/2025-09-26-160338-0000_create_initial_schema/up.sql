-- ENUM for tribes
CREATE TYPE tribe AS ENUM ('Roman', 'Gaul', 'Teuton', 'Natar', 'Nature');

-- Extension to use UUID as primary key
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Players
CREATE TABLE IF NOT EXISTS players (
    id UUID PRIMARY KEY,
    username VARCHAR(255) NOT NULL UNIQUE,
    tribe tribe NOT NULL
);

-- Villages
CREATE TABLE IF NOT EXISTS villages (
    id SERIAL PRIMARY KEY,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    pos_x INTEGER NOT NULL,
    pos_y INTEGER NOT NULL,

    buildings JSONB NOT NULL,
    production JSONB NOT NULL,
    stocks JSONB NOT NULL,
    smithy_upgrades JSONB NOT NULL,

    population INTEGER NOT NULL DEFAULT 2,
    loyalty SMALLINT NOT NULL DEFAULT 100 CHECK (loyalty >= 0 AND loyalty <= 100),
    is_capital BOOLEAN NOT NULL DEFAULT FALSE,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(pos_x, pos_y)
);

-- Heroes
CREATE TABLE heroes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,

    health SMALLINT NOT NULL DEFAULT 100,
    experience INTEGER NOT NULL DEFAULT 0,
    attack_points INTEGER NOT NULL DEFAULT 0,
    defense_points INTEGER NOT NULL DEFAULT 0,
    off_bonus SMALLINT NOT NULL DEFAULT 0,
    def_bonus SMALLINT NOT NULL DEFAULT 0
);

-- Armies
CREATE TABLE armies (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    village_id INTEGER NOT NULL REFERENCES villages(id) ON DELETE CASCADE,
    current_map_field_id INTEGER NOT NULL,
    hero_id UUID REFERENCES heroes(id) ON DELETE SET NULL,
    units JSONB NOT NULL,
    smithy JSONB NOT NULL,
    tribe Tribe NOT NULL,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE
);

-- Trigger to automatically update `updated_at`
CREATE OR REPLACE FUNCTION trigger_set_timestamp()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON villages
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();
