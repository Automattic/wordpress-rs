CREATE TABLE `post_types_edit_context` (
  -- Internal DB field (auto-incrementing)
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,

  -- Site identifier (foreign key to db_sites table)
  `db_site_id` INTEGER NOT NULL REFERENCES db_sites(id) ON DELETE CASCADE,

  -- Post type slug (e.g., 'post', 'page', 'wp_block')
  `slug` TEXT NOT NULL,

  -- Full post type data as JSON (PostTypeDetailsWithEditContext)
  `data` TEXT NOT NULL,

  -- Client-side cache metadata: when this post type was last fetched from the WordPress API
  `last_fetched_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

  FOREIGN KEY (db_site_id) REFERENCES db_sites(id) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_post_types_edit_context_unique_db_site_id_and_slug ON post_types_edit_context(db_site_id, slug);
CREATE INDEX idx_post_types_edit_context_db_site_id ON post_types_edit_context(db_site_id);
