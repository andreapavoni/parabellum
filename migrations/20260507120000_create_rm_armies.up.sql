CREATE TABLE rm_armies (
    army_id UUID PRIMARY KEY,
    home_village_id INTEGER NOT NULL REFERENCES rm_village(village_id) ON DELETE CASCADE,
    current_village_id INTEGER NOT NULL REFERENCES rm_village(village_id) ON DELETE CASCADE,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    state TEXT NOT NULL CHECK (state IN ('home', 'stationed', 'moving')),
    payload JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_rm_armies_home_village_id ON rm_armies(home_village_id);
CREATE INDEX idx_rm_armies_current_village_id ON rm_armies(current_village_id);
CREATE INDEX idx_rm_armies_player_id ON rm_armies(player_id);
CREATE INDEX idx_rm_armies_state ON rm_armies(state);
