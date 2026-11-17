-- A WordPress.com account. Account-level data, such as the Reader, belongs to it rather than to a
-- site, and is deleted with it when the user signs out.
CREATE TABLE `wordpress_com_accounts` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` INTEGER NOT NULL UNIQUE
) STRICT;

-- One row per Reader post per account, whichever streams show it. Every column but the identity is
-- nullable, because some sources (such as the saved list) send only part of a post.
CREATE TABLE `reader_posts` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `account_id` INTEGER NOT NULL,
  `identity_key` TEXT NOT NULL,       -- the normalized saved key, e.g. post:123:456, feed-item:789, url:https://...
  `post_id` INTEGER NOT NULL,         -- ID
  `site_id` INTEGER,
  `feed_id` INTEGER,
  `feed_item_id` INTEGER,
  `is_external` INTEGER NOT NULL DEFAULT 0,
  `global_id` TEXT,
  `pseudo_id` TEXT,
  `url` TEXT,
  `short_url` TEXT,
  `title` TEXT,
  `excerpt` TEXT,
  `content` TEXT,                     -- raw HTML
  `use_excerpt` INTEGER,
  `date` TEXT,
  `modified` TEXT,
  `author_id` INTEGER,
  `author_name` TEXT,
  `author_first_name` TEXT,
  `author_nice_name` TEXT,
  `author_avatar_url` TEXT,
  `author_url` TEXT,
  `author_site_id` INTEGER,
  `site_name` TEXT,
  `site_url` TEXT,
  `feed_url` TEXT,
  `site_icon_url` TEXT,
  `site_locale` TEXT,
  `site_is_private` INTEGER,
  `site_is_atomic` INTEGER,
  `is_jetpack` INTEGER,
  `organization_id` INTEGER,
  `featured_image` TEXT,
  `display_image_url` TEXT,          -- the featured image, else the server's `featured_media` image, else the image an app passed with a save. Shown when there's no `editorial_image_url`
  `editorial_image_url` TEXT,        -- an editor's pick from Discover. Kept once stored, since the saved list never sends it, and shown ahead of `display_image_url`
  `tags` TEXT,                        -- JSON
  `editorial_highlight_topic` TEXT,
  `xpost_site_id` INTEGER,
  `xpost_post_id` INTEGER,
  `xpost_site_url` TEXT,
  `xpost_post_url` TEXT,
  `xpost_comment_url` TEXT,
  `like_count` INTEGER,
  `i_like` INTEGER,
  `likes_enabled` INTEGER,
  `is_following` INTEGER,
  `comment_count` INTEGER,
  `comments_open` INTEGER,
  `can_subscribe_comments` INTEGER,
  `is_subscribed_comments` INTEGER,
  `receives_comment_notifications` INTEGER,
  `is_seen` INTEGER,                  -- NULL when the post doesn't support seen status
  `word_count` INTEGER,
  `discover_metadata` TEXT,           -- JSON
  `attachments` TEXT,                 -- JSON
  `fetched_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  `additional_fields` TEXT,

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_reader_posts_identity ON reader_posts(account_id, identity_key);
CREATE INDEX idx_reader_posts_site_post ON reader_posts(account_id, site_id, post_id);
CREATE INDEX idx_reader_posts_feed_item ON reader_posts(account_id, feed_item_id);
CREATE INDEX idx_reader_posts_author ON reader_posts(account_id, author_id);

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

-- One row per save on the server, as far as this device knows, and what the user wants for it.
-- A row is pending while `saved` differs from `server`: the service sends it until they agree. A
-- tapped row is sent once more even if they agree, because the server may have changed since.
CREATE TABLE `reader_saved_posts` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `account_id` INTEGER NOT NULL,
  `key` TEXT NOT NULL,                -- normalized, e.g. post:123:456. The key the server stores, once an answer or a refresh says
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
  `imported` INTEGER NOT NULL DEFAULT 0, -- from an import, so it can be sent in a bulk request
  `waiting_for_room` INTEGER NOT NULL DEFAULT 0, -- refused for the account's save cap with no tap waiting on it, so kept saved and sent again once there's room

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE,
  FOREIGN KEY (post_id) REFERENCES reader_posts(rowid) ON DELETE SET NULL
) STRICT;

CREATE UNIQUE INDEX idx_reader_saved_posts_key ON reader_saved_posts(account_id, key);
CREATE INDEX idx_reader_saved_posts_feed_item_key ON reader_saved_posts(account_id, feed_item_key);
CREATE INDEX idx_reader_saved_posts_date ON reader_saved_posts(account_id, date_saved);

-- The URLs the server stores as a saved post's key: a post can have several, such as its own domain's
-- and its wordpress.com address, or a ?p= link.
CREATE TABLE `reader_saved_post_urls` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `saved_post_id` INTEGER NOT NULL,   -- reader_saved_posts.rowid
  `url_key` TEXT NOT NULL,            -- normalized, e.g. url:https://...

  FOREIGN KEY (saved_post_id) REFERENCES reader_saved_posts(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_reader_saved_post_urls_post ON reader_saved_post_urls(saved_post_id, url_key);
CREATE INDEX idx_reader_saved_post_urls_url ON reader_saved_post_urls(url_key);

-- What the saved posts service knows about an account besides its rows.
CREATE TABLE `reader_saved_state` (
  `account_id` INTEGER PRIMARY KEY,
  `keys_refreshed` INTEGER NOT NULL DEFAULT 0, -- 1 once a refresh of the saved keys has been applied
  `keys_complete` INTEGER NOT NULL DEFAULT 0,  -- 1 if the newest refresh returned every saved key: past 1,000 it doesn't
  `total` INTEGER,                    -- the newest total the server gave, from a refresh or a write's answer

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;
