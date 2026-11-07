-- The saved keys from the newest refresh applied, as `GET /wpcom/v2/read/saved/keys` returned them.
-- Replaced as a whole by each refresh. The refresh itself is described in reader_saved_state.
CREATE TABLE `reader_saved_keys` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `account_id` INTEGER NOT NULL,
  `key` TEXT NOT NULL,                -- normalized, e.g. post:123:456, feed-item:789, url:https://...
  `save_id` INTEGER NOT NULL,
  `date_saved` TEXT NOT NULL,
  `url` TEXT,                         -- the URL of a post saved by its IDs, when the server sent it

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_reader_saved_keys_key ON reader_saved_keys(account_id, key);
CREATE INDEX idx_reader_saved_keys_date ON reader_saved_keys(account_id, date_saved);

-- What the saved posts service knows about an account besides its keys.
CREATE TABLE `reader_saved_state` (
  `account_id` INTEGER PRIMARY KEY,
  `keys_refreshed_at_ms` INTEGER,     -- wall clock, when the refresh in reader_saved_keys started. NULL before the first
  `keys_returned` INTEGER,            -- how many keys it returned, before any normalized to the same key
  `keys_total` INTEGER,               -- the server's total, which goes past the 1,000 keys it returns
  `local` TEXT,                       -- versioned JSON owned by wp_mobile: this device's writes that may still override the server
  `imported_local_saves` INTEGER NOT NULL DEFAULT 0,

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;

-- Saves the server refused when this device imported them, which stay saved on this device.
CREATE TABLE `reader_saved_local_only` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `account_id` INTEGER NOT NULL,
  `key` TEXT NOT NULL,
  `post_id` INTEGER,                  -- reader_posts.rowid
  `date_saved` TEXT,

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE,
  FOREIGN KEY (post_id) REFERENCES reader_posts(rowid) ON DELETE SET NULL
) STRICT;

CREATE UNIQUE INDEX idx_reader_saved_local_only_key ON reader_saved_local_only(account_id, key);
