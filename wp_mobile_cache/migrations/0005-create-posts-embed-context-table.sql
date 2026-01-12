CREATE TABLE `posts_embed_context` (
  -- Internal DB field (auto-incrementing)
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,

  -- Site identifier (foreign key to db_sites table)
  `db_site_id` INTEGER NOT NULL REFERENCES db_sites(id) ON DELETE CASCADE,

  -- Top-level non-nullable fields (minimal set for embed context)
  `id` INTEGER NOT NULL,
  `date` TEXT NOT NULL,
  `link` TEXT NOT NULL,
  `slug` TEXT NOT NULL,
  `post_type` TEXT NOT NULL,

  -- Nested: title (only rendered field in embed context)
  `title_rendered` TEXT,

  -- Top-level optional fields
  `author` INTEGER,

  -- Nested: excerpt (entire struct is optional)
  `excerpt_raw` TEXT,
  `excerpt_rendered` TEXT,
  `excerpt_protected` INTEGER,

  -- Featured media
  `featured_media` INTEGER,

  -- Client-side cache metadata: when this post was last fetched from the WordPress API
  `last_fetched_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

  FOREIGN KEY (db_site_id) REFERENCES db_sites(id) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_posts_embed_context_unique_db_site_id_and_id ON posts_embed_context(db_site_id, id);
CREATE INDEX idx_posts_embed_context_db_site_id ON posts_embed_context(db_site_id);
