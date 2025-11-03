CREATE TABLE `posts_view_context` (
  -- Internal DB field (auto-incrementing)
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,

  -- Site identifier (foreign key to db_sites table)
  `db_site_id` INTEGER NOT NULL REFERENCES db_sites(id) ON DELETE CASCADE,

  -- Top-level non-nullable fields
  `id` INTEGER NOT NULL,
  `date` TEXT NOT NULL,
  `date_gmt` TEXT NOT NULL,
  `link` TEXT NOT NULL,
  `modified` TEXT NOT NULL,
  `modified_gmt` TEXT NOT NULL,
  `slug` TEXT NOT NULL,
  `status` TEXT NOT NULL,
  `post_type` TEXT NOT NULL,
  `template` TEXT NOT NULL,

  -- Top-level optional fields
  `author` INTEGER,
  `featured_media` INTEGER,
  `sticky` INTEGER,
  `parent` INTEGER,
  `menu_order` INTEGER,

  -- Optional enums (stored as TEXT)
  `comment_status` TEXT,
  `ping_status` TEXT,
  `format` TEXT,

  -- Complex optional fields (JSON)
  `meta` TEXT,

  -- Nested: guid (only rendered field in view context)
  `guid_rendered` TEXT NOT NULL,

  -- Nested: title (only rendered field in view context)
  `title_rendered` TEXT NOT NULL,

  -- Nested: content (only rendered and protected fields in view context)
  `content_rendered` TEXT NOT NULL,
  `content_protected` INTEGER,

  -- Nested: excerpt (entire struct is optional)
  `excerpt_raw` TEXT,
  `excerpt_rendered` TEXT,
  `excerpt_protected` INTEGER,

  -- Client-side cache metadata: when this post was last fetched from the WordPress API
  `last_fetched_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

  FOREIGN KEY (db_site_id) REFERENCES db_sites(id) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_posts_view_context_unique_db_site_id_and_id ON posts_view_context(db_site_id, id);
CREATE INDEX idx_posts_view_context_db_site_id ON posts_view_context(db_site_id);
