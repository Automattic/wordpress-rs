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
