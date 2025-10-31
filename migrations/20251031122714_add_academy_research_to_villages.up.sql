-- Add up migration script here
ALTER TABLE villages
ADD COLUMN academy_research JSONB NOT NULL;
