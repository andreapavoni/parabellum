-- Add up migration script here
CREATE TABLE users (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    email       VARCHAR NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE players
    ADD user_id UUID NOT NULL REFERENCES users(id) ON DELETE SET NULL;
