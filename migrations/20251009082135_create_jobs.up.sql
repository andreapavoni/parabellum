-- Add up migration script here
-- Enum for job status
CREATE TYPE job_status AS ENUM ('Pending', 'Processing', 'Completed', 'Failed');

CREATE TABLE jobs (
    id UUID PRIMARY KEY,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    village_id INTEGER NOT NULL REFERENCES villages(id) ON DELETE CASCADE,

    task JSONB NOT NULL,
    status job_status NOT NULL DEFAULT 'Pending',

    completed_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index on `completed_at` + `status` is MANDATORY for worker's performances
CREATE INDEX jobs_lookup_idx ON jobs (status, completed_at);

-- Trigger to automatically update `updated_at`
CREATE TRIGGER set_jobs_timestamp
BEFORE UPDATE ON jobs
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();
