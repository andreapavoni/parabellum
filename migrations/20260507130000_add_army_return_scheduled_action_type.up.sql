DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_enum e
        JOIN pg_type t ON t.oid = e.enumtypid
        WHERE t.typname = 'scheduled_action_type'
          AND e.enumlabel = 'ArmyReturn'
    ) THEN
        ALTER TYPE scheduled_action_type ADD VALUE 'ArmyReturn';
    END IF;
END $$;
