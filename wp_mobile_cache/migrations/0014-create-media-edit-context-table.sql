CREATE TABLE `media_edit_context` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `db_site_id` INTEGER NOT NULL REFERENCES db_sites(id) ON DELETE CASCADE,

  -- Top-level fields (mirror MediaWithEditContext)
  `id` INTEGER NOT NULL,
  `date` TEXT NOT NULL,
  `date_gmt` TEXT NOT NULL,
  `link` TEXT NOT NULL,
  `modified` TEXT NOT NULL,
  `modified_gmt` TEXT NOT NULL,
  `slug` TEXT NOT NULL,
  `status` TEXT NOT NULL,
  `post_type` TEXT NOT NULL,
  `password` TEXT,
  `permalink_template` TEXT NOT NULL,
  `generated_slug` TEXT NOT NULL,
  `author` INTEGER NOT NULL,
  `comment_status` TEXT NOT NULL,
  `ping_status` TEXT NOT NULL,
  `template` TEXT NOT NULL,
  `alt_text` TEXT NOT NULL,
  `media_type` TEXT NOT NULL,
  `mime_type` TEXT NOT NULL,
  `source_url` TEXT NOT NULL,
  `post_id` INTEGER,

  -- Required list field (stored as JSON to keep migrations simple)
  `missing_image_sizes` TEXT NOT NULL,

  -- Nested guid (raw is optional, rendered is required in edit context)
  `guid_raw` TEXT,
  `guid_rendered` TEXT NOT NULL,

  -- Nested title (rendered is required, raw is optional)
  `title_raw` TEXT,
  `title_rendered` TEXT NOT NULL,

  -- Nested caption / description (both inner fields are non-optional Strings in edit context)
  `caption_raw` TEXT NOT NULL,
  `caption_rendered` TEXT NOT NULL,
  `description_raw` TEXT NOT NULL,
  `description_rendered` TEXT NOT NULL,

  -- Opaque media_details payload, stored as raw JSON, parsed lazily on read.
  `media_details` TEXT NOT NULL,

  -- Cache metadata
  `last_fetched_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  `additional_fields` TEXT,

  FOREIGN KEY (db_site_id) REFERENCES db_sites(id) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_media_edit_context_unique_db_site_id_and_id ON media_edit_context(db_site_id, id);
CREATE INDEX idx_media_edit_context_db_site_id ON media_edit_context(db_site_id);
