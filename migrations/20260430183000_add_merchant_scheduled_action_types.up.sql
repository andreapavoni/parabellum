ALTER TYPE scheduled_action_type
    ADD VALUE IF NOT EXISTS 'MerchantArrival';

ALTER TYPE scheduled_action_type
    ADD VALUE IF NOT EXISTS 'MerchantReturn';
