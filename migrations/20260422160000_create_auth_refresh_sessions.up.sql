CREATE TABLE auth_refresh_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    current_village_id INTEGER NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    user_agent TEXT,
    ip INET
);

CREATE INDEX idx_auth_refresh_sessions_user_id ON auth_refresh_sessions (user_id);
CREATE INDEX idx_auth_refresh_sessions_player_id ON auth_refresh_sessions (player_id);
CREATE INDEX idx_auth_refresh_sessions_expires_at ON auth_refresh_sessions (expires_at);
CREATE INDEX idx_auth_refresh_sessions_revoked_at ON auth_refresh_sessions (revoked_at);
