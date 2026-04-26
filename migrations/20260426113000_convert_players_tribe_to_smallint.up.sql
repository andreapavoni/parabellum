ALTER TABLE players
    ADD COLUMN tribe_code SMALLINT;

UPDATE players
SET tribe_code = CASE tribe
    WHEN 'Roman' THEN 1
    WHEN 'Gaul' THEN 2
    WHEN 'Teuton' THEN 3
    WHEN 'Natar' THEN 4
    WHEN 'Nature' THEN 5
END;

ALTER TABLE players
    DROP COLUMN tribe;

ALTER TABLE players
    RENAME COLUMN tribe_code TO tribe;

ALTER TABLE players
    ALTER COLUMN tribe SET NOT NULL;

ALTER TABLE players
    ADD CONSTRAINT players_tribe_check CHECK (tribe BETWEEN 1 AND 5);
