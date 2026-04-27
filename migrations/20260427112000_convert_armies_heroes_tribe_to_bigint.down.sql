ALTER TABLE armies
    ADD COLUMN tribe_enum tribe;

UPDATE armies
SET tribe_enum = CASE tribe
    WHEN 1 THEN 'Roman'::tribe
    WHEN 2 THEN 'Gaul'::tribe
    WHEN 3 THEN 'Teuton'::tribe
    WHEN 4 THEN 'Natar'::tribe
    WHEN 5 THEN 'Nature'::tribe
END;

ALTER TABLE armies
    DROP CONSTRAINT IF EXISTS armies_tribe_check;

ALTER TABLE armies
    DROP COLUMN tribe;

ALTER TABLE armies
    RENAME COLUMN tribe_enum TO tribe;

ALTER TABLE armies
    ALTER COLUMN tribe SET NOT NULL;

ALTER TABLE heroes
    ADD COLUMN tribe_enum tribe;

UPDATE heroes
SET tribe_enum = CASE tribe
    WHEN 1 THEN 'Roman'::tribe
    WHEN 2 THEN 'Gaul'::tribe
    WHEN 3 THEN 'Teuton'::tribe
    WHEN 4 THEN 'Natar'::tribe
    WHEN 5 THEN 'Nature'::tribe
END;

ALTER TABLE heroes
    DROP CONSTRAINT IF EXISTS heroes_tribe_check;

ALTER TABLE heroes
    DROP COLUMN tribe;

ALTER TABLE heroes
    RENAME COLUMN tribe_enum TO tribe;

ALTER TABLE heroes
    ALTER COLUMN tribe SET NOT NULL;
