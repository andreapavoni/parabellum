ALTER TABLE players
    ADD COLUMN tribe_enum tribe;

UPDATE players
SET tribe_enum = CASE tribe
    WHEN 1 THEN 'Roman'::tribe
    WHEN 2 THEN 'Gaul'::tribe
    WHEN 3 THEN 'Teuton'::tribe
    WHEN 4 THEN 'Natar'::tribe
    WHEN 5 THEN 'Nature'::tribe
END;

ALTER TABLE players
    DROP CONSTRAINT IF EXISTS players_tribe_check;

ALTER TABLE players
    DROP COLUMN tribe;

ALTER TABLE players
    RENAME COLUMN tribe_enum TO tribe;

ALTER TABLE players
    ALTER COLUMN tribe SET NOT NULL;
