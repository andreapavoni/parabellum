-- Add up migration script here

CREATE TABLE heroes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    village_id INTEGER NOT NULL REFERENCES villages(id) ON DELETE CASCADE,
    tribe tribe NOT NULL,
    level SMALLINT NOT NULL DEFAULT 0,
    health SMALLINT NOT NULL DEFAULT 100,
    experience INTEGER NOT NULL DEFAULT 0,
    resource_focus JSONB NOT NULL,
    strength_points SMALLINT NOT NULL DEFAULT 0,
    resources_points SMALLINT NOT NULL DEFAULT 0,
    off_bonus_points SMALLINT NOT NULL DEFAULT 0,
    def_bonus_points SMALLINT NOT NULL DEFAULT 0,
    regeneration_points SMALLINT NOT NULL DEFAULT 0,
    unassigned_points SMALLINT NOT NULL DEFAULT 5
);

ALTER TABLE armies ADD hero_id UUID REFERENCES heroes(id) ON DELETE SET NULL;

CREATE UNIQUE INDEX IF NOT EXISTS armies_unique_hero_idx
  ON armies(hero_id)
  WHERE hero_id IS NOT NULL;
