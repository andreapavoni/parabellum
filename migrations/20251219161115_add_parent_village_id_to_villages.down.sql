-- Rollback: remove parent_village_id column and index
DROP INDEX IF EXISTS idx_villages_parent_village_id;
ALTER TABLE villages DROP COLUMN parent_village_id;
