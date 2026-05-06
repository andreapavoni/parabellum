CREATE TABLE rm_reports (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    report_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    actor_player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    actor_village_id INTEGER,
    target_player_id UUID REFERENCES players(id) ON DELETE CASCADE,
    target_village_id INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE rm_report_reads (
    report_id UUID NOT NULL REFERENCES rm_reports(id) ON DELETE CASCADE,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    read_at TIMESTAMPTZ,
    PRIMARY KEY (report_id, player_id)
);

CREATE INDEX idx_rm_reports_actor ON rm_reports (actor_player_id);
CREATE INDEX idx_rm_reports_target ON rm_reports (target_player_id);
CREATE INDEX idx_rm_report_reads_player ON rm_report_reads (player_id, read_at);
