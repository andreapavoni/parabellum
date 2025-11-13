-- Add down migration script here
ALTER TABLE armies DROP COLUMN hero_id;
DROP TABLE IF EXISTS heroes;
DROP INDEX IF EXISTS armies_unique_hero_idx;
