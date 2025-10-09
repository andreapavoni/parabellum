-- Add down migration script here
DROP TRIGGER IF EXISTS set_timestamp ON jobs;
DROP INDEX IF EXISTS jobs_lookup_idx;
DROP TABLE IF EXISTS jobs;
DROP TYPE IF EXISTS job_status;
