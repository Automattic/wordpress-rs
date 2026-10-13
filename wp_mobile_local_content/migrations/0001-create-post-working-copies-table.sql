CREATE TABLE `post_working_copies` (
  -- Internal DB field (auto-incrementing)
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,

  -- Public identifier, a v4 UUID generated on insert. Unlike `rowid`, it
  -- stays unique across databases.
  `uuid` TEXT NOT NULL UNIQUE,

  `title` TEXT NOT NULL,
  `content` TEXT NOT NULL,

  `created_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  `updated_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;
