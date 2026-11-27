-- A post's comments, and the comments the reader wrote on this device that the server hasn't taken
-- yet. A written comment is sent once: if that fails it stays here, marked as failed, until the
-- reader sends it again or discards it.
CREATE TABLE `reader_comments` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `account_id` INTEGER NOT NULL,
  `site_id` INTEGER NOT NULL,
  `post_id` INTEGER NOT NULL,
  `comment_id` INTEGER,               -- NULL for a comment written here that the server hasn't answered
  `local_id` TEXT,                    -- set for a comment written here: what it's called until it has the server's ID
  `parent_id` INTEGER,                -- the comment it replies to. NULL for a comment on the post itself
  `author_id` INTEGER,                -- 0 for a guest or a pingback
  `author_name` TEXT,
  `author_avatar_url` TEXT,
  `author_url` TEXT,
  `date` TEXT,
  `content` TEXT,                     -- HTML, as the site sent it. For a comment written here, what the reader typed
  `status` TEXT,                      -- approved, unapproved, spam, trash, or what the API sent
  `comment_type` TEXT,                -- comment, pingback, trackback, or what the API sent
  `like_count` INTEGER,
  `i_like` INTEGER,
  `send_state` INTEGER NOT NULL DEFAULT 0, -- 0 the server's, 1 written here and being sent, 2 written here and not taken
  `send_error` TEXT,                  -- why it wasn't taken

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_reader_comments_comment ON reader_comments(account_id, site_id, comment_id) WHERE comment_id IS NOT NULL;
CREATE UNIQUE INDEX idx_reader_comments_local ON reader_comments(account_id, local_id) WHERE local_id IS NOT NULL;
CREATE INDEX idx_reader_comments_post ON reader_comments(account_id, site_id, post_id, date);

-- Where a post's comments are up to: how many the server has, and whether a fetch is running.
CREATE TABLE `reader_comment_threads` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `account_id` INTEGER NOT NULL,
  `site_id` INTEGER NOT NULL,
  `post_id` INTEGER NOT NULL,
  `found` INTEGER,                    -- how many comments the server has, as of the newest page
  `last_refreshed_at` TEXT,
  `state` INTEGER NOT NULL DEFAULT 0, -- ListState: 0 idle, 1 fetching first page, 2 fetching next page, 3 error
  `error_message` TEXT,

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_reader_comment_threads_post ON reader_comment_threads(account_id, site_id, post_id);
