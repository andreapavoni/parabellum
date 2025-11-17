-- Add down migration script here
ALTER TABLE players DROP COLUMN user_id;
DROP TABLE IF EXISTS users;
