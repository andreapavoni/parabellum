DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM pg_type t
        JOIN pg_enum e ON t.oid = e.enumtypid
        WHERE t.typname = 'scheduled_action_type'
          AND e.enumlabel IN ('MerchantArrival', 'MerchantReturn')
    ) THEN
        CREATE TYPE scheduled_action_type_new AS ENUM (
            'ReinforcementArrival',
            'AddBuilding',
            'UpgradeBuilding',
            'DowngradeBuilding',
            'TrainUnit',
            'ResearchAcademy',
            'ResearchSmithy'
        );

        ALTER TABLE rm_scheduled_actions
            ALTER COLUMN action_type
            TYPE scheduled_action_type_new
            USING action_type::text::scheduled_action_type_new;

        DROP TYPE scheduled_action_type;
        ALTER TYPE scheduled_action_type_new RENAME TO scheduled_action_type;
    END IF;
END$$;
