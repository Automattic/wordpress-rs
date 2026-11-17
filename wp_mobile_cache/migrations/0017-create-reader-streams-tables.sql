-- A Reader stream, such as the saved list, and where it's up to.
CREATE TABLE `reader_streams` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `account_id` INTEGER NOT NULL,
  `key` TEXT NOT NULL,                -- e.g. saved
  `next_page_handle` TEXT,
  `has_more` INTEGER,                 -- NULL until the first page loads
  `last_refreshed_at` TEXT,
  `state` INTEGER NOT NULL DEFAULT 0, -- ListState: 0 idle, 1 fetching first page, 2 fetching next page, 3 error
  `error_message` TEXT,
  `version` INTEGER NOT NULL DEFAULT 0, -- incremented by each refresh, so a load-more that started before it is discarded

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_reader_streams_key ON reader_streams(account_id, key);

-- A post in a stream, with what's particular to the stream.
CREATE TABLE `reader_stream_items` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `stream_id` INTEGER NOT NULL,
  `post_id` INTEGER NOT NULL,         -- reader_posts.rowid
  `sort_key` TEXT NOT NULL,           -- newest first, e.g. the date saved
  `gap_before` INTEGER NOT NULL DEFAULT 0,
  `railcar` TEXT,
  `score` REAL,
  `card_kind` INTEGER NOT NULL DEFAULT 0,

  FOREIGN KEY (stream_id) REFERENCES reader_streams(rowid) ON DELETE CASCADE,
  FOREIGN KEY (post_id) REFERENCES reader_posts(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_reader_stream_items_unique ON reader_stream_items(stream_id, post_id);
CREATE INDEX idx_reader_stream_items_order ON reader_stream_items(stream_id, sort_key);
CREATE INDEX idx_reader_stream_items_post ON reader_stream_items(post_id);
