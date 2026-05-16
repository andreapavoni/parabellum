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
    culture_points INTEGER NOT NULL DEFAULT 0,
    culture_points_updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_players_user_id ON players (user_id);
