ALTER TABLE auth_refresh_sessions
    DROP CONSTRAINT IF EXISTS auth_refresh_sessions_current_village_id_fkey;

ALTER TABLE auth_refresh_sessions
    ADD CONSTRAINT auth_refresh_sessions_current_village_id_fkey
    FOREIGN KEY (current_village_id)
    REFERENCES rm_village(village_id)
    ON DELETE CASCADE;
