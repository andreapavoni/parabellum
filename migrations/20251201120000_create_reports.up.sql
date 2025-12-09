CREATE TABLE reports (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    report_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    actor_player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    actor_village_id INTEGER REFERENCES villages(id) ON DELETE SET NULL,
    target_player_id UUID REFERENCES players(id) ON DELETE CASCADE,
    target_village_id INTEGER REFERENCES villages(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE report_reads (
    report_id UUID NOT NULL REFERENCES reports(id) ON DELETE CASCADE,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    read_at TIMESTAMPTZ,
    PRIMARY KEY (report_id, player_id)
);

CREATE INDEX idx_reports_actor ON reports (actor_player_id);
CREATE INDEX idx_reports_target ON reports (target_player_id);
CREATE INDEX idx_report_reads_player ON report_reads (player_id, read_at);
