CREATE TABLE `posts_edit_context` (
  -- Internal DB field (auto-incrementing)
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,

  -- Site identifier (foreign key to sites table)
  `db_site_id` INTEGER NOT NULL REFERENCES sites(id) ON DELETE CASCADE,

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
  `password` TEXT NOT NULL,
  `template` TEXT NOT NULL,

  -- Top-level optional fields
  `permalink_template` TEXT,
  `generated_slug` TEXT,
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
  `categories` TEXT,
  `tags` TEXT,

  -- Nested: guid (guid is non-optional, but guid.raw is optional)
  `guid_raw` TEXT,
  `guid_rendered` TEXT NOT NULL,

  -- Nested: title (title is non-optional, but title.raw is optional)
  `title_raw` TEXT,
  `title_rendered` TEXT NOT NULL,

  -- Nested: content (content is non-optional, but some fields are optional)
  `content_raw` TEXT,
  `content_rendered` TEXT NOT NULL,
  `content_protected` INTEGER,
  `content_block_version` INTEGER,

  -- Nested: excerpt (entire struct is optional)
  `excerpt_raw` TEXT,
  `excerpt_rendered` TEXT,
  `excerpt_protected` INTEGER,

  -- Client-side cache metadata: when this post was last fetched from the WordPress API
  `last_fetched_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

  FOREIGN KEY (db_site_id) REFERENCES sites(id) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_posts_edit_context_unique_db_site_id_and_id ON posts_edit_context(db_site_id, id);
CREATE INDEX idx_posts_edit_context_db_site_id ON posts_edit_context(db_site_id);
