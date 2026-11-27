-- A working copy: the user's version of a post or page. Its fields live in
-- its versions. Each row has one row in `working_copy_remote_targets`, and at
-- least one version. The repository writes them in one transaction.
CREATE TABLE `working_copies` (
  -- Internal DB field (auto-incrementing)
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,

  -- Public identifier, a v4 UUID generated on insert. Unlike `rowid`, it
  -- stays unique across databases.
  `uuid` TEXT NOT NULL UNIQUE,

  `kind` TEXT NOT NULL CHECK (`kind` IN ('post', 'page')),

  `created_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

-- The content of a working copy's versions. A version whose content is the
-- same as its parent's refers to the parent's row instead of adding one.
CREATE TABLE `working_copy_contents` (
  -- Internal DB field (auto-incrementing)
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,

  `working_copy_rowid` INTEGER NOT NULL REFERENCES `working_copies`(`rowid`) ON DELETE CASCADE,

  `content` TEXT NOT NULL
) STRICT;

CREATE INDEX `idx_working_copy_contents_working_copy` ON `working_copy_contents`(`working_copy_rowid`);

-- Every field of a working copy's versions other than the content, shared by
-- posts and pages. Each row has one row in the field sets table for its
-- working copy's kind. A version whose fields other than the content are the
-- same as its parent's refers to the parent's row instead of adding one.
CREATE TABLE `working_copy_field_sets` (
  -- Internal DB field (auto-incrementing)
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,

  `working_copy_rowid` INTEGER NOT NULL REFERENCES `working_copies`(`rowid`) ON DELETE CASCADE,

  `title` TEXT NOT NULL,
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

  -- NULL: the default template
  `template` TEXT,
  -- NULL: the post type has no custom order
  `menu_order` INTEGER
) STRICT;

CREATE INDEX `idx_working_copy_field_sets_working_copy` ON `working_copy_field_sets`(`working_copy_rowid`);

CREATE TABLE `post_field_sets` (
  `field_set_rowid` INTEGER PRIMARY KEY REFERENCES `working_copy_field_sets`(`rowid`) ON DELETE CASCADE,

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

CREATE TABLE `page_field_sets` (
  `field_set_rowid` INTEGER PRIMARY KEY REFERENCES `working_copy_field_sets`(`rowid`) ON DELETE CASCADE,

  -- NULL: a top-level page
  `parent_id` INTEGER
) STRICT;

-- The history of a working copy: one row each time its fields were saved with
-- a change. The newest version, the one with the highest `rowid`, holds the
-- working copy's fields.
CREATE TABLE `working_copy_versions` (
  -- Internal DB field (auto-incrementing)
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,

  -- Public identifier, a v4 UUID generated on insert
  `uuid` TEXT NOT NULL UNIQUE,

  `working_copy_rowid` INTEGER NOT NULL REFERENCES `working_copies`(`rowid`) ON DELETE CASCADE,

  -- The version this one was made from. NULL: the working copy's first
  -- version. No ON DELETE action: removing a version from the history means
  -- giving its children another parent first.
  `parent_rowid` INTEGER REFERENCES `working_copy_versions`(`rowid`),

  `content_rowid` INTEGER NOT NULL REFERENCES `working_copy_contents`(`rowid`),
  `field_set_rowid` INTEGER NOT NULL REFERENCES `working_copy_field_sets`(`rowid`),

  -- The editing session the version was saved in. A version goes with its
  -- session.
  `session_rowid` INTEGER NOT NULL REFERENCES `editing_sessions`(`rowid`) ON DELETE CASCADE,

  `created_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

CREATE INDEX `idx_working_copy_versions_working_copy` ON `working_copy_versions`(`working_copy_rowid`);
CREATE INDEX `idx_working_copy_versions_parent` ON `working_copy_versions`(`parent_rowid`);

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

-- One stretch of work on a working copy, such as editing it in an editor, or
-- one save from a screen of the app. A session that ended has `ended_at` and
-- `ending`. One without them is still open, until a later run of its client
-- records it as abandoned.
CREATE TABLE `editing_sessions` (
  -- Internal DB field (auto-incrementing)
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,

  -- Public identifier, a v4 UUID generated on insert
  `uuid` TEXT NOT NULL UNIQUE,

  `working_copy_rowid` INTEGER NOT NULL REFERENCES `working_copies`(`rowid`) ON DELETE CASCADE,

  -- Where the work happens: 'editor' for an editor, 'app' for the app itself
  `source` TEXT NOT NULL CHECK (`source` IN ('editor', 'app')),
  -- Which editor or part of the app, such as GutenbergKit or post-settings.
  -- NULL: not given
  `source_name` TEXT,

  -- The client that opened the session, such as the app or one of its
  -- extensions, as the client names itself
  `client` TEXT NOT NULL,
  -- The run of the process that opened the session, a v4 UUID per process
  `run` TEXT NOT NULL,

  -- Who the session makes its requests as, in a form that isn't a credential.
  -- Recorded as given. NULL: not given
  `identity` TEXT,

  -- Unix timestamps in seconds
  `started_at` INTEGER NOT NULL,
  -- NULL: no change has been recorded in the session
  `last_recorded_at` INTEGER,
  -- For an abandoned session, when a later run found it not ended
  `ended_at` INTEGER,
  -- 'normal', or 'abandoned' when a later run of its client found it not
  -- ended. Set exactly when `ended_at` is
  `ending` TEXT CHECK (`ending` IN ('normal', 'abandoned')),

  CHECK ((`ended_at` IS NULL) = (`ending` IS NULL))
) STRICT;

-- The requests sent during a session, by the editor or by wordpress-rs, with
-- what came back. A row is added when the request is sent, and completed with
-- its response or failure. A row that's never completed is a request whose
-- outcome nobody recorded. Credential headers are left out.
CREATE TABLE `editing_session_requests` (
  -- Internal DB field (auto-incrementing). The order the reports arrived in.
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,

  `session_rowid` INTEGER NOT NULL REFERENCES `editing_sessions`(`rowid`) ON DELETE CASCADE,

  -- The request's ID, unique within the session
  `request_id` TEXT NOT NULL,

  -- Who sent it: 'editor' or 'app', as for a session's `source`
  `source` TEXT NOT NULL CHECK (`source` IN ('editor', 'app')),
  -- NULL: not given
  `source_name` TEXT,

  `method` TEXT NOT NULL,
  -- The REST API path with its query, as the editor sent it
  `path` TEXT NOT NULL,
  -- A JSON object, one value per lowercase header name
  `request_headers` TEXT NOT NULL CHECK (json_valid(`request_headers`) AND json_type(`request_headers`) = 'object'),
  -- NULL: the request had no body
  `request_body` BLOB,
  -- Unix timestamp in seconds, when wordpress-rs recorded the report
  `sent_at` INTEGER NOT NULL,

  -- Set once the response arrives
  `status_code` INTEGER,
  -- A JSON object, one value per lowercase header name. Set with `status_code`
  `response_headers` TEXT CHECK (`response_headers` IS NULL OR (json_valid(`response_headers`) AND json_type(`response_headers`) = 'object')),
  `response_body` BLOB,
  -- Set when the request failed without a response
  `failure` TEXT,
  -- Unix timestamp in seconds, when wordpress-rs recorded the response or the failure
  `completed_at` INTEGER,

  CHECK (`status_code` IS NULL OR `failure` IS NULL),
  CHECK ((`status_code` IS NULL) = (`response_headers` IS NULL)),
  CHECK ((`status_code` IS NULL AND `failure` IS NULL) = (`completed_at` IS NULL)),
  UNIQUE (`session_rowid`, `request_id`)
) STRICT;
