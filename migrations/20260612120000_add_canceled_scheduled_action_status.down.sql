UPDATE rm_scheduled_actions
SET status = 'failed'
WHERE status = 'canceled';

ALTER TYPE scheduled_action_status RENAME TO scheduled_action_status_old;

CREATE TYPE scheduled_action_status AS ENUM ('pending', 'processing', 'completed', 'failed');

ALTER TABLE rm_scheduled_actions
    ALTER COLUMN status DROP DEFAULT,
    ALTER COLUMN status TYPE scheduled_action_status
    USING status::text::scheduled_action_status;

ALTER TABLE rm_scheduled_actions
    ALTER COLUMN status SET DEFAULT 'pending';

DROP TYPE scheduled_action_status_old;
