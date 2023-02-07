-- Add up migration script here

CREATE TABLE IF NOT EXISTS villages (
	id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
	player_id BLOB NOT NULL,
	valley_id INTEGER,
	tribe TEXT NOT NULL,
  buildings TEXT NOT NULL,
  oases TEXT NOT NULL,
  population INTEGER DEFAULT 0,
  army TEXT NOT NULL,
  reinforcements TEXT NOT NULL,
  loyalty INTEGER NOT NULL DEFAULT 100,
  production TEXT NOT NULL,
  is_capital INTEGER NOT NULL DEFAULT 0,
  smithy TEXT NOT NULL,
  stocks TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_villages_id ON villages (id);