-- What the saved posts service knows, so it survives a relaunch. The two blobs are versioned JSON
-- owned by wp_mobile: the keys from the last refresh, and this device's writes since.
CREATE TABLE `reader_saved_state` (
  `account_id` INTEGER PRIMARY KEY,
  `snapshot` TEXT,
  `local` TEXT,
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
