ALTER TABLE auth_refresh_sessions
    DROP CONSTRAINT IF EXISTS auth_refresh_sessions_current_village_id_fkey;
