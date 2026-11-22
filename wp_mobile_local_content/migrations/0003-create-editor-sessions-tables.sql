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
