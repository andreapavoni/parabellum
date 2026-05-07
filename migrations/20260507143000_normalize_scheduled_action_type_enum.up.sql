ALTER TYPE scheduled_action_type RENAME TO scheduled_action_type_old;

CREATE TYPE scheduled_action_type AS ENUM (
    'ReinforcementArrival',
    'SettlersArrival',
    'AttackArrival',
    'ArmyReturn',
    'ScoutArrival',
    'MerchantArrival',
    'MerchantReturn',
    'AddBuilding',
    'UpgradeBuilding',
    'DowngradeBuilding',
    'TrainUnit',
    'ResearchAcademy',
    'ResearchSmithy'
);

ALTER TABLE rm_scheduled_actions
    ALTER COLUMN action_type TYPE TEXT
    USING action_type::text;

UPDATE rm_scheduled_actions
SET action_type = 'ArmyReturn'
WHERE action_type IN ('AttackReturn', 'ScoutReturn', 'ReinforcementReturn');

ALTER TABLE rm_scheduled_actions
    ALTER COLUMN action_type TYPE scheduled_action_type
    USING action_type::scheduled_action_type;

DROP TYPE scheduled_action_type_old;
