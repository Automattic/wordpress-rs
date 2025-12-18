-- Table 1: List header/pagination info
CREATE TABLE `list_metadata` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `db_site_id` INTEGER NOT NULL,
  `key` TEXT NOT NULL,              -- e.g., "edit:posts:publish"
  `total_pages` INTEGER,
  `total_items` INTEGER,
  `current_page` INTEGER NOT NULL DEFAULT 0,
  `per_page` INTEGER NOT NULL DEFAULT 20,
  `last_first_page_fetched_at` TEXT,
  `last_fetched_at` TEXT,
  `version` INTEGER NOT NULL DEFAULT 0,

  FOREIGN KEY (db_site_id) REFERENCES db_sites(id) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_list_metadata_unique_key ON list_metadata(db_site_id, key);

-- Table 2: List items (rowid = insertion order = display order)
CREATE TABLE `list_metadata_items` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `list_metadata_id` INTEGER NOT NULL,
  `entity_id` INTEGER NOT NULL,     -- post/comment/etc ID
  `modified_gmt` TEXT,              -- nullable for entities without it
  `parent` INTEGER,                 -- parent post ID (for hierarchical post types like pages)
  `menu_order` INTEGER,             -- menu order (for hierarchical post types)

  FOREIGN KEY (list_metadata_id) REFERENCES list_metadata(rowid) ON DELETE CASCADE
) STRICT;

CREATE INDEX idx_list_metadata_items_list ON list_metadata_items(list_metadata_id);
CREATE INDEX idx_list_metadata_items_entity ON list_metadata_items(list_metadata_id, entity_id);

-- Table 3: Sync state (FK to list_metadata, not duplicating key)
CREATE TABLE `list_metadata_state` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `list_metadata_id` INTEGER NOT NULL,
  `state` INTEGER NOT NULL DEFAULT 0,  -- 0=idle, 1=fetching_first_page, 2=fetching_next_page, 3=error
  `error_message` TEXT,
  `updated_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

  FOREIGN KEY (list_metadata_id) REFERENCES list_metadata(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_list_metadata_state_unique ON list_metadata_state(list_metadata_id);
