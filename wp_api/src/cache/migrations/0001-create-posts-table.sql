CREATE TABLE `posts` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `post_id` INTEGER NOT NULL,
  `context` TEXT COLLATE NOCASE NOT NULL,
  `post_author` INTEGER NOT NULL,
  `post_date` TEXT COLLATE NOCASE NOT NULL,
  `post_content` TEXT COLLATE NOCASE NOT NULL,
  `post_title` TEXT COLLATE NOCASE NOT NULL,
  `post_excerpt` TEXT COLLATE NOCASE NOT NULL,
  `post_status` TEXT COLLATE NOCASE NOT NULL,
  `comment_status` TEXT COLLATE NOCASE NOT NULL,
  `ping_status` TEXT COLLATE NOCASE NOT NULL,
  `post_password` TEXT COLLATE NOCASE DEFAULT NULL,
  `post_modified` TEXT COLLATE NOCASE NOT NULL,
  `post_parent` INTEGER,
  `guid` TEXT COLLATE NOCASE NOT NULL,
  `menu_order` INTEGER NOT NULL DEFAULT '0',
  `post_type` TEXT COLLATE NOCASE NOT NULL,
  `comment_count` INTEGER NOT NULL
) STRICT;

CREATE UNIQUE INDEX idx_posts_have_unique_post_id_and_context ON posts(post_id, context);
