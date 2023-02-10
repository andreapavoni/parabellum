-- Add up migration script here
CREATE TABLE IF NOT EXISTS map_fields (
	id INTEGER PRIMARY KEY,
	player_id BLOB,
	village_id INTEGER,
	x INTEGER NOT NULL,
	y INTEGER NOT NULL,
	topology TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_map_fields_id ON map_fields (id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_map_fields_position ON map_fields (x, y);