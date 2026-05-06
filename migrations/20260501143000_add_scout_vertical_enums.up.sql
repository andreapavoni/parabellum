ALTER TYPE movement_type
    ADD VALUE IF NOT EXISTS 'Scout';

ALTER TYPE scheduled_action_type
    ADD VALUE IF NOT EXISTS 'ScoutArrival';

ALTER TYPE scheduled_action_type
    ADD VALUE IF NOT EXISTS 'ScoutReturn';
