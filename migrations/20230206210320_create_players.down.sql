-- Add down migration script here
DROP TABLE IF EXISTS players;
DROP INDEX IF EXISTS idx_players_username;