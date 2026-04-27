ALTER TABLE players
    DROP CONSTRAINT IF EXISTS players_tribe_check;

ALTER TABLE players
    ALTER COLUMN tribe TYPE BIGINT
    USING tribe::BIGINT;

ALTER TABLE players
    ADD CONSTRAINT players_tribe_check CHECK (tribe BETWEEN 1 AND 5);
