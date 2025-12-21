-- Add culture_points and culture_points_production columns to villages table
ALTER TABLE villages
ADD COLUMN culture_points INTEGER NOT NULL DEFAULT 0,
ADD COLUMN culture_points_production INTEGER NOT NULL DEFAULT 0;
