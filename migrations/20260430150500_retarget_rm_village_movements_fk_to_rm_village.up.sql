ALTER TABLE rm_village_movements
    DROP CONSTRAINT IF EXISTS rm_village_movements_village_id_fkey,
    DROP CONSTRAINT IF EXISTS rm_village_movements_source_village_id_fkey,
    DROP CONSTRAINT IF EXISTS rm_village_movements_target_village_id_fkey;

ALTER TABLE rm_village_movements
    ADD CONSTRAINT rm_village_movements_village_id_fkey
        FOREIGN KEY (village_id) REFERENCES rm_village(village_id) ON DELETE CASCADE,
    ADD CONSTRAINT rm_village_movements_source_village_id_fkey
        FOREIGN KEY (source_village_id) REFERENCES rm_village(village_id) ON DELETE CASCADE,
    ADD CONSTRAINT rm_village_movements_target_village_id_fkey
        FOREIGN KEY (target_village_id) REFERENCES rm_village(village_id) ON DELETE CASCADE;
