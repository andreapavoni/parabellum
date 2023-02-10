-- Add down migration script here
DROP TABLE IF EXISTS map_fields;
DROP INDEX IF EXISTS idx_map_fields_id;
DROP INDEX IF EXISTS idx_map_fields_position;