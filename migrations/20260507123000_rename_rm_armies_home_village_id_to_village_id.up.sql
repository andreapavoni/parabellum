DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'rm_armies'
          AND column_name = 'home_village_id'
    ) THEN
        ALTER TABLE rm_armies RENAME COLUMN home_village_id TO village_id;
    END IF;
END $$;

DROP INDEX IF EXISTS idx_rm_armies_home_village_id;
CREATE INDEX IF NOT EXISTS idx_rm_armies_village_id ON rm_armies(village_id);
