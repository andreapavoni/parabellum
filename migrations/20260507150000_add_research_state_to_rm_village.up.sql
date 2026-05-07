ALTER TABLE rm_village
    ADD COLUMN IF NOT EXISTS smithy_upgrades JSONB NOT NULL DEFAULT '[0,0,0,0,0,0,0,0]'::jsonb,
    ADD COLUMN IF NOT EXISTS academy_research JSONB NOT NULL DEFAULT '[true,false,false,false,false,false,false,false,false,true]'::jsonb;
