-- Something the user turned on or off that the server doesn't have yet, such as a post marked as
-- seen, a like, a follow or a block. The change shows in the cache straight away, and this row is
-- what's left to send. It goes once the server agrees, or refuses.
CREATE TABLE `reader_actions` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `account_id` INTEGER NOT NULL,
  `kind` INTEGER NOT NULL,            -- which action, as the Reader service numbers them
  `target` TEXT NOT NULL,             -- what it's about, e.g. a post's identity key, or site:123
  `wanted` INTEGER NOT NULL,          -- what the user wants: 1 on, 0 off
  `was` INTEGER,                      -- what the cache showed before the first tap still waiting, to go back to if the server refuses. NULL when it wasn't known
  `seq` INTEGER NOT NULL,             -- the order of the last tap: rows are sent in order, and a tap made while its row is being sent is noticed
  `payload` TEXT NOT NULL,            -- JSON: what the request needs, such as the post's IDs
  `attempts` INTEGER NOT NULL DEFAULT 0, -- how many times it's been sent without an answer
  `last_error` TEXT,

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_reader_actions_target ON reader_actions(account_id, kind, target);
CREATE INDEX idx_reader_actions_seq ON reader_actions(account_id, seq);

-- The sites the user blocked. The Reader's streams leave out their posts.
CREATE TABLE `reader_blocked_sites` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `account_id` INTEGER NOT NULL,
  `site_id` INTEGER NOT NULL,
  `name` TEXT,
  `url` TEXT,

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_reader_blocked_sites_site ON reader_blocked_sites(account_id, site_id);

-- The users the reader blocked. The server keeps no such list, so it lives on this device only.
-- The Reader's streams leave out posts they wrote.
CREATE TABLE `reader_blocked_users` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `account_id` INTEGER NOT NULL,
  `user_id` INTEGER NOT NULL,

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_reader_blocked_users_user ON reader_blocked_users(account_id, user_id);
