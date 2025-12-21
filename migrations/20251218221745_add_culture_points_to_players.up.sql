-- Add culture_points column to players table
ALTER TABLE players
ADD COLUMN culture_points INTEGER NOT NULL DEFAULT 0;
