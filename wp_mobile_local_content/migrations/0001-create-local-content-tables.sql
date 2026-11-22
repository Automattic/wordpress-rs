-- Fields shared by every kind of working copy. Each row has exactly one
-- matching row in the fields table for its `kind`, and one in
-- `working_copy_remote_targets`. The repository writes all of them in one transaction.
CREATE TABLE `working_copies` (
  -- Internal DB field (auto-incrementing)
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,

  -- Public identifier, a v4 UUID generated on insert. Unlike `rowid`, it
  -- stays unique across databases.
  `uuid` TEXT NOT NULL UNIQUE,

  `kind` TEXT NOT NULL CHECK (`kind` IN ('post', 'page')),

  `title` TEXT NOT NULL,
  `content` TEXT NOT NULL,
  `excerpt` TEXT NOT NULL,
  -- NULL: the user hasn't set a slug
  `slug` TEXT,

  -- The status as WordPress names it, including statuses registered by plugins
  `status` TEXT NOT NULL CHECK (`status` <> ''),

  -- NULL: the post has no password. Never set for a private post, matching
  -- WordPress, which clears the password when saving a private post.
  `password` TEXT CHECK (`status` <> 'private' OR `password` IS NULL),

  -- In the site's timezone, as WordPress's `date`, such as
  -- 2024-01-15T10:30:00. NULL: publish immediately
  `publish_date` TEXT,

  -- NULL: the user hasn't chosen an author
  `author_id` INTEGER,

  -- NULL: no featured image
  `featured_media_id` INTEGER,

  -- NULL: the user hasn't chosen whether comments and pings are open
  `comment_status` TEXT,
  `ping_status` TEXT,

  `created_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  `updated_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

  -- NULL: the default template
  `template` TEXT,
  -- NULL: the post type has no custom order
  `menu_order` INTEGER
) STRICT;

CREATE TABLE `post_working_copy_fields` (
  `working_copy_rowid` INTEGER PRIMARY KEY REFERENCES `working_copies`(`rowid`) ON DELETE CASCADE,

  -- JSON array of term IDs
  `category_ids` TEXT NOT NULL CHECK (json_valid(`category_ids`) AND json_type(`category_ids`) = 'array'),
  -- JSON array of term IDs
  `tag_ids` TEXT NOT NULL CHECK (json_valid(`tag_ids`) AND json_type(`tag_ids`) = 'array'),
  `format` TEXT NOT NULL,
  `sticky` INTEGER NOT NULL CHECK (`sticky` IN (0, 1)),
  -- Post meta, as a JSON object with one value per meta key
  `meta` TEXT NOT NULL CHECK (json_valid(`meta`) AND json_type(`meta`) = 'object'),
  -- The post's fields that have no column of their own, such as fields plugins
  -- add, as a JSON object with one value per key
  `additional_fields` TEXT NOT NULL CHECK (json_valid(`additional_fields`) AND json_type(`additional_fields`) = 'object')
) STRICT;

CREATE TABLE `page_working_copy_fields` (
  `working_copy_rowid` INTEGER PRIMARY KEY REFERENCES `working_copies`(`rowid`) ON DELETE CASCADE,

  -- NULL: a top-level page
  `parent_id` INTEGER
) STRICT;

-- The sites working copies belong to, one row per site. A site with a
-- WordPress.com site ID is matched by it, whatever its URL. A site without one
-- is matched by its URL.
CREATE TABLE `sites` (
  -- Internal DB field (auto-incrementing)
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,

  -- NULL: no WordPress.com site ID is known for the site
  `wordpress_com_site_id` INTEGER UNIQUE,
  -- The URL given when a working copy for the site was last created, as
  -- `ParsedUrl` serializes it
  `url` TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX `idx_sites_url_without_wordpress_com_site_id`
  ON `sites`(`url`) WHERE `wordpress_com_site_id` IS NULL;

-- Where a working copy goes when it's pushed: its site, and the post on that
-- site the push updates. A row is deleted with its working copy, and can't
-- exist without one.
CREATE TABLE `working_copy_remote_targets` (
  `working_copy_rowid` INTEGER PRIMARY KEY REFERENCES `working_copies`(`rowid`) ON DELETE CASCADE,

  -- No ON DELETE action: a site can't be deleted while a working copy belongs to it
  `site_rowid` INTEGER NOT NULL REFERENCES `sites`(`rowid`),

  -- NULL: the working copy is a new post that isn't on the server yet
  `remote_post_id` INTEGER,
  -- The base version: the remote post's `modified_gmt` when the working copy
  -- was created from it, or the one its last recorded push returned. A Unix
  -- timestamp in seconds, set exactly when `remote_post_id` is.
  `base_modified_gmt` INTEGER,

  CHECK ((`remote_post_id` IS NULL) = (`base_modified_gmt` IS NULL)),

  -- At most one working copy per remote post. New posts have a NULL
  -- `remote_post_id`, and SQLite treats NULLs as distinct, so a site can have
  -- any number of them.
  UNIQUE (`site_rowid`, `remote_post_id`)
) STRICT;

-- One stretch of editing a working copy in an editor, from opening the editor
-- to closing it. A session that ended normally has an `ended_at`. One without
-- it either is still open or didn't end normally.
CREATE TABLE `editor_sessions` (
  -- Internal DB field (auto-incrementing)
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,

  -- Public identifier, a v4 UUID generated on insert
  `uuid` TEXT NOT NULL UNIQUE,

  `working_copy_rowid` INTEGER NOT NULL REFERENCES `working_copies`(`rowid`) ON DELETE CASCADE,

  -- Unix timestamps in seconds
  `started_at` INTEGER NOT NULL,
  -- NULL: no change has been recorded in the session
  `last_recorded_at` INTEGER,
  -- NULL: the session hasn't ended normally
  `ended_at` INTEGER
) STRICT;

-- A push sent during an editor session that has no answer yet, at most one
-- per session. It's deleted once the push gets an answer, or once a later
-- fetch shows whether it landed.
CREATE TABLE `editor_session_unconfirmed_pushes` (
  `editor_session_rowid` INTEGER PRIMARY KEY REFERENCES `editor_sessions`(`rowid`) ON DELETE CASCADE,

  -- The post the push updates, and the base version its fields started from,
  -- a Unix timestamp in seconds. Both NULL: the push creates the post.
  `remote_post_id` INTEGER,
  `base_modified_gmt` INTEGER,

  -- Unix timestamp in seconds
  `sent_at` INTEGER NOT NULL,

  -- The request body the push sent
  `body` TEXT NOT NULL CHECK (json_valid(`body`) AND json_type(`body`) = 'object'),

  CHECK ((`remote_post_id` IS NULL) = (`base_modified_gmt` IS NULL))
) STRICT;

-- The fields in which the version of the post an editor saved differed from
-- the server's version wordpress-rs has, both with the same modified date, at
-- most one per session. Only something other than the server can change the
-- editor's version that way.
CREATE TABLE `editor_session_base_mismatches` (
  `editor_session_rowid` INTEGER PRIMARY KEY REFERENCES `editor_sessions`(`rowid`) ON DELETE CASCADE,

  -- Unix timestamp in seconds
  `detected_at` INTEGER NOT NULL,

  -- The REST API keys of the differing fields
  `fields` TEXT NOT NULL CHECK (json_valid(`fields`) AND json_type(`fields`) = 'array')
) STRICT;
