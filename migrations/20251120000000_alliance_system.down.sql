-- Rollback: Alliance System Migration
-- Removes all alliance-related tables and player fields

-- Remove created_at column from players table
ALTER TABLE players DROP COLUMN IF EXISTS created_at;

-- Remove alliance fields from players table
ALTER TABLE players DROP COLUMN IF EXISTS alliance_settings;
ALTER TABLE players DROP COLUMN IF EXISTS alliance_notification_enabled;
ALTER TABLE players DROP COLUMN IF EXISTS total_alliance_trade_contributions;
ALTER TABLE players DROP COLUMN IF EXISTS total_alliance_cp_contributions;
ALTER TABLE players DROP COLUMN IF EXISTS total_alliance_armor_contributions;
ALTER TABLE players DROP COLUMN IF EXISTS total_alliance_training_contributions;
ALTER TABLE players DROP COLUMN IF EXISTS current_alliance_trade_contributions;
ALTER TABLE players DROP COLUMN IF EXISTS current_alliance_cp_contributions;
ALTER TABLE players DROP COLUMN IF EXISTS current_alliance_armor_contributions;
ALTER TABLE players DROP COLUMN IF EXISTS current_alliance_training_contributions;
ALTER TABLE players DROP COLUMN IF EXISTS alliance_contributions;
ALTER TABLE players DROP COLUMN IF EXISTS alliance_join_time;
ALTER TABLE players DROP COLUMN IF EXISTS alliance_role;
ALTER TABLE players DROP COLUMN IF EXISTS alliance_role_name;
ALTER TABLE players DROP COLUMN IF EXISTS alliance_id;

-- Drop all alliance tables
DROP TABLE IF EXISTS alliance_map_flag CASCADE;
DROP TABLE IF EXISTS alliance_bonus_upgrade_queue CASCADE;
DROP TABLE IF EXISTS alliance_notification CASCADE;
DROP TABLE IF EXISTS alliance_medal CASCADE;
DROP TABLE IF EXISTS alliance_diplomacy CASCADE;
DROP TABLE IF EXISTS alliance_log CASCADE;
DROP TABLE IF EXISTS alliance_invite CASCADE;
DROP TABLE IF EXISTS alliance CASCADE;

-- Drop the medal period type enum
DROP TYPE IF EXISTS medal_period_type;
