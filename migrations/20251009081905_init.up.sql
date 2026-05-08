CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TYPE tribe AS ENUM ('Roman', 'Gaul', 'Teuton', 'Natar', 'Nature');

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL
);

CREATE TABLE players (
    id UUID PRIMARY KEY,
    username VARCHAR(255) NOT NULL UNIQUE,
    tribe tribe NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    culture_points INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_players_user_id ON players (user_id);

CREATE TABLE map_fields (
    id INTEGER PRIMARY KEY,
    village_id INTEGER NULL,
    player_id UUID NULL REFERENCES players(id) ON DELETE SET NULL,
    position JSONB NOT NULL,
    topology JSONB NOT NULL,
    UNIQUE(position)
);

CREATE INDEX idx_map_fields_village_id ON map_fields (village_id);
CREATE INDEX idx_map_fields_player_id ON map_fields (player_id);
