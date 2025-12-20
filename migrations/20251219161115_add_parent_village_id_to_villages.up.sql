-- Add parent_village_id to track parent-child village relationships for settlement slots
ALTER TABLE villages
ADD COLUMN parent_village_id INTEGER REFERENCES villages(id) ON DELETE SET NULL;

-- Index for efficient querying of child villages
CREATE INDEX idx_villages_parent_village_id ON villages(parent_village_id);
