-- One row per WordPress.com notification (a note) per account, as the API sent it whole. The note
-- is read back from `json`, so a later version of the library can read fields this one doesn't
-- store. The other columns are what a list sorts and filters by.
CREATE TABLE `notifications` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `account_id` INTEGER NOT NULL,
  `note_id` INTEGER NOT NULL,
  `note_type` TEXT,                   -- as the API sent it, e.g. comment
  `is_read` INTEGER,                  -- 1 read, 0 unread, NULL when the note didn't say. Set to what the server took when the user marks the note, as `json`'s `read` is
  `timestamp` INTEGER,                -- seconds since 1970. A note with no date has one in the first day of 1970
  `note_hash` TEXT,                   -- the hash the whole note came with. Only compares with the hashes of requests made the same way
  `json` TEXT NOT NULL,               -- the note's JSON object, as the API sent it, apart from `read` (see `is_read`)
  `fetched_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_notifications_note ON notifications(account_id, note_id);
CREATE INDEX idx_notifications_timestamp ON notifications(account_id, timestamp);

-- A note a list of IDs had that didn't come whole when asked for, or came and couldn't be read. It
-- isn't asked for again until the list gives it a different hash.
CREATE TABLE `notification_listed_only` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `account_id` INTEGER NOT NULL,
  `note_id` INTEGER NOT NULL,
  `note_hash` TEXT,                   -- the hash the list gave it when it was asked for
  `error` TEXT,                       -- why it couldn't be read, when it came. NULL when it didn't come

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_notification_listed_only_note ON notification_listed_only(account_id, note_id);

-- When an account's notifications were last synced and looked at, and the changes to all of them
-- the server doesn't have yet.
CREATE TABLE `notification_sync_state` (
  `account_id` INTEGER PRIMARY KEY,
  `last_synced_at` INTEGER,           -- when the newest notes were last synced, in seconds since 1970
  `last_seen_time` INTEGER,           -- when the user last looked at their notifications, as the API last said, in seconds since 1970. Only moves forward
  `seen_time_to_send` INTEGER,        -- a newer time the user saw notes up to, not sent yet, in seconds since 1970
  `read_all_up_to` INTEGER,           -- the user marked every note at or before this time read, and it's not sent yet, in seconds since 1970

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;

-- Where each tab of an account's notifications is up to in loading older notes. A tab's notes are
-- the rows of `notifications` it shows: a note is stored once, however many tabs show it.
CREATE TABLE `notification_tab_state` (
  `account_id` INTEGER NOT NULL,
  `tab` TEXT NOT NULL,                -- all, unread, comments, subscribers or likes
  `older_before` INTEGER,             -- the `before` of the tab's next older page, in seconds since 1970. NULL for the newest notes
  `older_number` INTEGER,             -- the `number` of the tab's next older page. NULL until a sync sets it
  `reached_end` INTEGER NOT NULL DEFAULT 0, -- 1 once the tab's oldest note has been reached
  `newest_time` INTEGER,              -- the time of the newest dated note the tab's last sync listed, in seconds since 1970

  PRIMARY KEY (account_id, tab),
  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;

-- A note the user marked read or unread that the server doesn't have yet. The cached lists show
-- `wanted` instead of the note's own read state, so a sync that stores the server's copy doesn't
-- undo it. The row goes once the server takes the change, or refuses it.
CREATE TABLE `notification_read_changes` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `account_id` INTEGER NOT NULL,
  `note_id` INTEGER NOT NULL,
  `wanted` INTEGER NOT NULL,          -- 1 read, 0 unread
  `seq` INTEGER NOT NULL,             -- the order of the last change: rows are sent in order, and a change made while its row is being sent is noticed
  `attempts` INTEGER NOT NULL DEFAULT 0, -- how many times it's been sent without an answer
  `last_error` TEXT,

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_notification_read_changes_note ON notification_read_changes(account_id, note_id);
