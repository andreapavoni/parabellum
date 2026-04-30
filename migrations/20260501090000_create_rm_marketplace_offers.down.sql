DROP TABLE IF EXISTS rm_marketplace_offers;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_type WHERE typname = 'rm_marketplace_offer_status'
    ) THEN
        DROP TYPE rm_marketplace_offer_status;
    END IF;
END $$;
