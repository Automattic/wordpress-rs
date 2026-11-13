-- One row per save on the server, as far as this device knows, and what the user wants for it.
-- A row is pending while `saved` differs from `server`: the service sends it until they agree. A
-- tapped row is sent once more even if they agree, because the server may have changed since.
CREATE TABLE `reader_saved_posts` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `account_id` INTEGER NOT NULL,
  `key` TEXT NOT NULL,                -- normalized, e.g. post:123:456. The key the server stores, once an answer or a refresh says
  `url_key` TEXT,                     -- a URL key the server stores as this key, e.g. url:https://...
  `feed_item_key` TEXT,               -- the feed item key of a WordPress.com post saved under its IDs
  `saved` INTEGER NOT NULL,           -- what the user wants
  `server` INTEGER,                   -- what the server last said: 1 saved, 0 not saved, NULL not known
  `in_doubt` INTEGER NOT NULL DEFAULT 0, -- sent without an answer, so not sent again until a refresh says what the server has
  `tapped` INTEGER NOT NULL DEFAULT 0, -- a tap's request isn't answered yet, so it's sent even if the row matches what the server last said
  `seq` INTEGER NOT NULL DEFAULT 0,   -- the order of the last tap on this device, 0 for none. The newer tap wins a merge
  `date_saved` TEXT,
  `date_known` INTEGER NOT NULL DEFAULT 0, -- 1 if `date_saved` came from the server: the keys or a list page. A tap's date isn't the server's: a save of a post already saved keeps its old date
  `post_id` INTEGER,                  -- reader_posts.rowid
  `listed` INTEGER NOT NULL DEFAULT 0, -- shown in the saved list: the list sent it, or this device saved it
  `imported` INTEGER NOT NULL DEFAULT 0, -- from the import of the device's old local saves
  `local_only` INTEGER NOT NULL DEFAULT 0, -- refused on import, so kept on this device and never sent

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE,
  FOREIGN KEY (post_id) REFERENCES reader_posts(rowid) ON DELETE SET NULL
) STRICT;

CREATE UNIQUE INDEX idx_reader_saved_posts_key ON reader_saved_posts(account_id, key);
CREATE INDEX idx_reader_saved_posts_url_key ON reader_saved_posts(account_id, url_key);
CREATE INDEX idx_reader_saved_posts_feed_item_key ON reader_saved_posts(account_id, feed_item_key);
CREATE INDEX idx_reader_saved_posts_date ON reader_saved_posts(account_id, date_saved);

-- What the saved posts service knows about an account besides its rows.
CREATE TABLE `reader_saved_state` (
  `account_id` INTEGER PRIMARY KEY,
  `keys_refreshed` INTEGER NOT NULL DEFAULT 0, -- 1 once a refresh of the saved keys has been applied
  `keys_complete` INTEGER NOT NULL DEFAULT 0,  -- 1 if the newest refresh returned every saved key: past 1,000 it doesn't
  `total` INTEGER,                    -- the newest total the server gave, from a refresh or a write's answer
  `imported_local_saves` INTEGER NOT NULL DEFAULT 0,

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;
