-- Add up migration script here

CREATE TABLE marketplace_offers (
    id UUID PRIMARY KEY,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    village_id INTEGER NOT NULL REFERENCES villages(id) ON DELETE CASCADE,

    offer_resources JSONB NOT NULL,
    seek_resources JSONB NOT NULL,

    merchants_required SMALLINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_marketplace_offers_village_id ON marketplace_offers(village_id);
