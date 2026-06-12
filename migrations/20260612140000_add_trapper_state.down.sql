DROP INDEX IF EXISTS idx_rm_armies_current_village_trapped;

ALTER TABLE rm_village
    DROP COLUMN IF EXISTS trapper_queued_traps,
    DROP COLUMN IF EXISTS trapper_broken_traps,
    DROP COLUMN IF EXISTS trapper_active_traps;

ALTER TABLE rm_armies
    DROP CONSTRAINT rm_armies_state_check,
    ADD CONSTRAINT rm_armies_state_check CHECK (state IN ('home', 'stationed', 'moving'));
