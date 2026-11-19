-- What this device has of a saved post for reading with no connection, besides the post itself.
-- A row goes when its save does.
CREATE TABLE `reader_saved_offline` (
  `saved_post_id` INTEGER PRIMARY KEY, -- reader_saved_posts.rowid
  `fetched` INTEGER NOT NULL DEFAULT 0, -- 1 once the post was stored in full, from the saved list or fetched on its own: the device has all the content there is, even if that's none
  `downloading` INTEGER NOT NULL DEFAULT 0, -- 1 while its content or images are being fetched. Reset when the cache opens
  `error` TEXT,                       -- why the last try to get its content failed. NULL when it didn't
  `attempts` INTEGER NOT NULL DEFAULT 0, -- tries the server answered with a failure
  `attempted_at` TEXT,                -- when the last of those was
  `images` INTEGER NOT NULL DEFAULT 0, -- 0 not stored, 1 stored: each image is in reader_offline_files, 2 removed to make room for newer saves

  FOREIGN KEY (saved_post_id) REFERENCES reader_saved_posts(rowid) ON DELETE CASCADE
) STRICT;

-- An image of a saved post, kept as a file in the directory the app gave for them. A file is
-- deleted once no row names it.
CREATE TABLE `reader_offline_files` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `post_id` INTEGER NOT NULL,         -- reader_posts.rowid
  `url` TEXT NOT NULL,                -- the image's address, as the post has it
  `file_name` TEXT,                   -- the file's name in the directory. NULL when the server said the image can't be had
  `bytes` INTEGER NOT NULL DEFAULT 0,

  FOREIGN KEY (post_id) REFERENCES reader_posts(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_reader_offline_files_post_url ON reader_offline_files(post_id, url);
CREATE INDEX idx_reader_offline_files_name ON reader_offline_files(file_name);
