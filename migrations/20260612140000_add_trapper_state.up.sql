ALTER TABLE rm_armies
    DROP CONSTRAINT rm_armies_state_check,
    ADD CONSTRAINT rm_armies_state_check CHECK (state IN ('home', 'stationed', 'moving', 'trapped'));

ALTER TYPE scheduled_action_type ADD VALUE IF NOT EXISTS 'TrapBuild';

ALTER TABLE rm_village
    ADD COLUMN trapper_active_traps INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN trapper_broken_traps INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN trapper_queued_traps INTEGER NOT NULL DEFAULT 0;

CREATE INDEX idx_rm_armies_current_village_trapped
    ON rm_armies(current_village_id)
    WHERE state = 'trapped';
