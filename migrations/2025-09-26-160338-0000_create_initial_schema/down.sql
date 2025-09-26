-- This file should undo anything in `up.sql`
DROP TRIGGER IF EXISTS set_timestamp ON villages;
DROP FUNCTION IF EXISTS trigger_set_timestamp();

DROP TABLE IF EXISTS armies;
DROP TABLE IF EXISTS heroes;
DROP TABLE IF EXISTS villages;
DROP TABLE IF EXISTS players;
DROP TYPE IF EXISTS tribe_enum;
