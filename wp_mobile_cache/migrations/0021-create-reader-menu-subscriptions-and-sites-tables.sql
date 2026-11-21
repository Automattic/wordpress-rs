-- The Reader menu: the streams, tags, lists and organizations a reader chooses from. The followed
-- tags are the entries of the `subscribed` section. A signed-out reader's followed tags are kept
-- here only, since no server has them.
CREATE TABLE `reader_menu_items` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `account_id` INTEGER NOT NULL,
  `section` INTEGER NOT NULL,         -- 0 default, 1 subscribed, 2 recommended
  `key` TEXT NOT NULL,                -- the entry's key in its section, e.g. following, or a tag's slug
  `kind` TEXT NOT NULL,               -- stream, tag, list, organization, or another kind as the API named it
  `position` INTEGER NOT NULL,        -- the order the server sent, or the order tags were followed in
  `title` TEXT,
  `slug` TEXT,
  `display_name` TEXT,
  `url` TEXT,
  `tag_id` INTEGER,
  `organization_id` INTEGER,

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_reader_menu_items_key ON reader_menu_items(account_id, section, key);

-- The sites and feeds the account follows.
CREATE TABLE `reader_subscriptions` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `account_id` INTEGER NOT NULL,
  `subscription_id` INTEGER,          -- NULL for a follow made on this device that the server hasn't answered yet
  `site_id` INTEGER,                  -- NULL for a feed that belongs to no WordPress.com or Jetpack site
  `feed_id` INTEGER,
  `url` TEXT,                         -- the address that was followed
  `feed_url` TEXT,
  `name` TEXT,
  `icon_url` TEXT,
  `date_subscribed` TEXT,
  `unseen_count` INTEGER,
  `organization_id` INTEGER,
  `is_owner` INTEGER,
  `is_rss` INTEGER,
  `email_posts` INTEGER,              -- whether new posts are emailed
  `email_comments` INTEGER,           -- whether new comments are emailed
  `email_frequency` TEXT,             -- instantly, daily, weekly, or what the API sent
  `notify_posts` INTEGER,             -- whether new posts send a notification
  `position` INTEGER NOT NULL DEFAULT 0, -- the order the server lists them in

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;

CREATE INDEX idx_reader_subscriptions_site ON reader_subscriptions(account_id, site_id);
CREATE INDEX idx_reader_subscriptions_feed ON reader_subscriptions(account_id, feed_id);
CREATE INDEX idx_reader_subscriptions_position ON reader_subscriptions(account_id, position);

-- What the Reader shows of a site or a feed besides its posts, such as at the top of its stream.
CREATE TABLE `reader_sites` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `account_id` INTEGER NOT NULL,
  `key` TEXT NOT NULL,                -- site:123 for a WordPress.com or Jetpack site, feed:456 for a feed
  `site_id` INTEGER,
  `feed_id` INTEGER,
  `name` TEXT,
  `description` TEXT,
  `url` TEXT,
  `feed_url` TEXT,
  `icon_url` TEXT,
  `subscribers_count` INTEGER,
  `post_count` INTEGER,
  `is_private` INTEGER,
  `is_jetpack` INTEGER,
  `organization_id` INTEGER,
  `is_following` INTEGER,
  `fetched_at` TEXT NOT NULL,

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_reader_sites_key ON reader_sites(account_id, key);
CREATE INDEX idx_reader_sites_site ON reader_sites(account_id, site_id);
CREATE INDEX idx_reader_sites_feed ON reader_sites(account_id, feed_id);
