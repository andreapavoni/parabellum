CREATE TABLE IF NOT EXISTS rm_map_fields (
    id INTEGER PRIMARY KEY,
    village_id INTEGER NULL REFERENCES rm_village(village_id) ON DELETE SET NULL,
    player_id UUID NULL REFERENCES players(id) ON DELETE SET NULL,
    position JSONB NOT NULL,
    topology JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_rm_map_fields_village_id ON rm_map_fields (village_id);
