-- Entity State Table
-- Tracks the state of individual entities (posts, categories, etc.) during fetch operations
-- States reset on app launch to prevent stuck "Fetching" states

CREATE TABLE entity_state (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id INTEGER NOT NULL,
    db_site_id INTEGER NOT NULL,
    entity_type TEXT NOT NULL,  -- 'posts_edit_context', 'categories_edit_context', etc.
    state INTEGER NOT NULL,     -- 0=Missing, 1=Fetching, 2=Cached, 3=Stale, 4=Failed
    error_message TEXT,
    updated_at TEXT NOT NULL,   -- ISO 8601 timestamp
    UNIQUE(entity_id, db_site_id, entity_type)
);

-- Optimize lookups by entity
CREATE INDEX idx_entity_state_lookup ON entity_state(entity_id, db_site_id, entity_type);

-- Optimize cleanup operations by state
CREATE INDEX idx_entity_state_cleanup ON entity_state(state);
