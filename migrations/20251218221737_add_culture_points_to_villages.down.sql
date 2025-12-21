-- Remove culture_points and culture_points_production columns from villages table
ALTER TABLE villages
DROP COLUMN culture_points,
DROP COLUMN culture_points_production;
