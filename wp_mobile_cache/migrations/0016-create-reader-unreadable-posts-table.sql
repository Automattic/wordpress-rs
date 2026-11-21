-- A post a Reader stream's page had that couldn't be read, such as one whose date has an impossible
-- UTC offset. The page keeps its other posts, and this row is what's known of the one it couldn't:
-- enough to ask for the post on its own later, once the server sends it in a form that reads.
CREATE TABLE `reader_unreadable_posts` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `account_id` INTEGER NOT NULL,
  `stream_id` INTEGER NOT NULL,       -- the stream whose page had it
  `identity_key` TEXT NOT NULL,       -- as reader_posts.identity_key
  `post_id` INTEGER NOT NULL,
  `site_id` INTEGER,
  `feed_id` INTEGER,
  `feed_item_id` INTEGER,
  `is_external` INTEGER NOT NULL DEFAULT 0,
  `url` TEXT,
  `error` TEXT NOT NULL,              -- why it couldn't be read, the last time a page had it
  `attempts` INTEGER NOT NULL DEFAULT 0, -- how many times it's been asked for on its own since
  `first_seen_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  `last_attempt_at` TEXT,

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE,
  FOREIGN KEY (stream_id) REFERENCES reader_streams(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_reader_unreadable_posts_key ON reader_unreadable_posts(stream_id, identity_key);
CREATE INDEX idx_reader_unreadable_posts_account ON reader_unreadable_posts(account_id);
