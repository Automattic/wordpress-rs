-- Fields shared by every kind of working copy. Each row has exactly one
-- matching row in the fields table for its `kind`, and one in
-- `working_copy_targets`. The repository writes all of them in one transaction.
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

  -- Unix timestamp in seconds. NULL: publish immediately
  `publish_date` INTEGER,

  -- NULL: the user hasn't chosen an author
  `author_id` INTEGER,

  -- NULL: no featured image
  `featured_media_id` INTEGER,

  -- NULL: the user hasn't chosen whether comments and pings are open
  `comment_status` TEXT,
  `ping_status` TEXT,

  `created_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  `updated_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

CREATE TABLE `post_working_copy_fields` (
  `working_copy_rowid` INTEGER PRIMARY KEY REFERENCES `working_copies`(`rowid`) ON DELETE CASCADE,

  -- JSON array of term IDs
  `category_ids` TEXT NOT NULL CHECK (json_valid(`category_ids`) AND json_type(`category_ids`) = 'array'),
  -- JSON array of tags, each {"existing": {"id": <term ID>}} or {"new": {"name": <name>}}
  `tags` TEXT NOT NULL CHECK (json_valid(`tags`) AND json_type(`tags`) = 'array'),
  `format` TEXT NOT NULL,
  `sticky` INTEGER NOT NULL CHECK (`sticky` IN (0, 1))
) STRICT;

CREATE TABLE `page_working_copy_fields` (
  `working_copy_rowid` INTEGER PRIMARY KEY REFERENCES `working_copies`(`rowid`) ON DELETE CASCADE,

  -- NULL: a top-level page
  `parent_id` INTEGER
) STRICT;

-- Sites working copies are pushed to, one row per site, identified by
-- something that survives the cache being cleared. A site with a WordPress.com
-- site ID is identified by it, whatever its URL. A site without one is
-- identified by its URL.
CREATE TABLE `sites` (
  -- Internal DB field (auto-incrementing)
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,

  -- NULL: no WordPress.com site ID is known for the site
  `wordpress_com_site_id` INTEGER UNIQUE,
  -- The URL last given for the site, as `ParsedUrl` serializes it
  `url` TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX `idx_sites_url_without_wordpress_com_site_id`
  ON `sites`(`url`) WHERE `wordpress_com_site_id` IS NULL;

-- Where a working copy is pushed: the site, and the remote post it edits.
-- A row depends on its working copy and is deleted with it, never the other
-- way around.
CREATE TABLE `working_copy_targets` (
  `working_copy_rowid` INTEGER PRIMARY KEY REFERENCES `working_copies`(`rowid`) ON DELETE CASCADE,

  -- No ON DELETE action: a site can't be deleted while a working copy targets it
  `site_rowid` INTEGER NOT NULL REFERENCES `sites`(`rowid`),

  -- NULL: the working copy is a new post
  `remote_post_id` INTEGER,
  -- The remote post's `modified_gmt` when the working copy was made from it,
  -- as a Unix timestamp in seconds. Set exactly when `remote_post_id` is.
  `base_modified_gmt` INTEGER,

  CHECK ((`remote_post_id` IS NULL) = (`base_modified_gmt` IS NULL)),

  -- At most one working copy per remote post. New posts have a NULL
  -- `remote_post_id`, and SQLite treats NULLs as distinct, so a site can have
  -- any number of them.
  UNIQUE (`site_rowid`, `remote_post_id`)
) STRICT;
