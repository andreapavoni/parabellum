ALTER TABLE armies
    ADD COLUMN tribe_code BIGINT;

UPDATE armies
SET tribe_code = CASE tribe
    WHEN 'Roman' THEN 1
    WHEN 'Gaul' THEN 2
    WHEN 'Teuton' THEN 3
    WHEN 'Natar' THEN 4
    WHEN 'Nature' THEN 5
END;

ALTER TABLE armies
    DROP COLUMN tribe;

ALTER TABLE armies
    RENAME COLUMN tribe_code TO tribe;

ALTER TABLE armies
    ALTER COLUMN tribe SET NOT NULL;

ALTER TABLE armies
    ADD CONSTRAINT armies_tribe_check CHECK (tribe BETWEEN 1 AND 5);

ALTER TABLE heroes
    ADD COLUMN tribe_code BIGINT;

UPDATE heroes
SET tribe_code = CASE tribe
    WHEN 'Roman' THEN 1
    WHEN 'Gaul' THEN 2
    WHEN 'Teuton' THEN 3
    WHEN 'Natar' THEN 4
    WHEN 'Nature' THEN 5
END;

ALTER TABLE heroes
    DROP COLUMN tribe;

ALTER TABLE heroes
    RENAME COLUMN tribe_code TO tribe;

ALTER TABLE heroes
    ALTER COLUMN tribe SET NOT NULL;

ALTER TABLE heroes
    ADD CONSTRAINT heroes_tribe_check CHECK (tribe BETWEEN 1 AND 5);
