-- Add up migration script here
CREATE TABLE IF NOT EXISTS players (
	id BLOB PRIMARY KEY,
	username TEXT NOT NULL UNIQUE,
	tribe TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_players_username ON players (username);