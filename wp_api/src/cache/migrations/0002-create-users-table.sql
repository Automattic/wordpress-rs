CREATE TABLE `users` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` INTEGER NOT NULL,
  `context` TEXT COLLATE NOCASE NOT NULL,
  `user_login` TEXT COLLATE NOCASE NOT NULL,
  `user_nicename` TEXT COLLATE NOCASE NOT NULL,
  `user_email` TEXT COLLATE NOCASE NOT NULL,
  `user_url` TEXT COLLATE NOCASE NOT NULL,
  `user_registered` TEXT COLLATE NOCASE NOT NULL,
  `user_status` INTEGER NOT NULL,
  `display_name` TEXT COLLATE NOCASE NOT NULL
) STRICT;

CREATE UNIQUE INDEX idx_users_have_unique_user_id_and_context ON users(user_id, context);
