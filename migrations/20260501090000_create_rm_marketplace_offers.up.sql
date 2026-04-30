DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_type WHERE typname = 'rm_marketplace_offer_status'
    ) THEN
        CREATE TYPE rm_marketplace_offer_status AS ENUM ('open', 'accepted', 'canceled');
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS rm_marketplace_offers (
    offer_id UUID PRIMARY KEY,
    owner_player_id UUID NOT NULL,
    owner_village_id INTEGER NOT NULL,
    offer_resources JSONB NOT NULL,
    seek_resources JSONB NOT NULL,
    merchants_reserved SMALLINT NOT NULL,
    status rm_marketplace_offer_status NOT NULL,
    accepted_by_player_id UUID NULL,
    accepted_by_village_id INTEGER NULL,
    created_at TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ NULL,
    canceled_at TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS idx_rm_marketplace_offers_owner_village
    ON rm_marketplace_offers(owner_village_id);

CREATE INDEX IF NOT EXISTS idx_rm_marketplace_offers_status
    ON rm_marketplace_offers(status);
