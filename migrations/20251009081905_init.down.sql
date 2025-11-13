-- Add down migration script here
-- This file should undo anything in `up.sql`
DROP TRIGGER IF EXISTS set_villages_timestamp ON villages;
DROP FUNCTION IF EXISTS trigger_set_timestamp();

DROP TABLE IF EXISTS map_fields;
DROP TABLE IF EXISTS armies;
DROP TABLE IF EXISTS villages;
DROP TABLE IF EXISTS players;
DROP TYPE IF EXISTS tribe;
