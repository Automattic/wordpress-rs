-- Add unique constraint on (list_metadata_id, entity_id) to prevent duplicate
-- entity IDs within the same list. This fixes an issue where pagination
-- instability could cause duplicate entity IDs to be inserted during
-- load_more operations.
--
-- Using UNIQUE INDEX instead of modifying the table schema to avoid
-- complex migration logic (SQLite doesn't support ADD CONSTRAINT).

-- First, remove any existing duplicates (keep the first occurrence by rowid)
DELETE FROM list_metadata_items
WHERE rowid NOT IN (
    SELECT MIN(rowid)
    FROM list_metadata_items
    GROUP BY list_metadata_id, entity_id
);

-- Drop the existing non-unique index
DROP INDEX IF EXISTS idx_list_metadata_items_entity;

-- Create the unique index
CREATE UNIQUE INDEX idx_list_metadata_items_entity ON list_metadata_items(list_metadata_id, entity_id);
