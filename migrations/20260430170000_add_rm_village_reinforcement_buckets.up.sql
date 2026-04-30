ALTER TABLE rm_village
    ADD COLUMN IF NOT EXISTS reinforcements JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS deployed_armies JSONB NOT NULL DEFAULT '{}'::jsonb;
