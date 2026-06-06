ALTER TABLE rm_village
ADD COLUMN IF NOT EXISTS loyalty_updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

UPDATE rm_village
SET loyalty_updated_at = NOW()
WHERE loyalty_updated_at IS NULL;
