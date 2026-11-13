-- A WordPress.com account. Account-level data, such as the Reader, belongs to it rather than to a
-- site, and is deleted with it when the user signs out.
CREATE TABLE `wordpress_com_accounts` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` INTEGER NOT NULL UNIQUE
) STRICT;
