-- The sites the account is a member of. A view of a public site's post by one of its members isn't
-- counted as a view in the Reader.
CREATE TABLE `reader_member_sites` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `account_id` INTEGER NOT NULL,
  `site_id` INTEGER NOT NULL,

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_reader_member_sites_site ON reader_member_sites(account_id, site_id);

-- When each of a reader's lists was last fetched whole, such as the sites they're a member of.
CREATE TABLE `reader_sync_times` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `account_id` INTEGER NOT NULL,
  `name` TEXT NOT NULL,               -- which list, e.g. member_sites
  `synced_at` TEXT NOT NULL,

  FOREIGN KEY (account_id) REFERENCES wordpress_com_accounts(rowid) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX idx_reader_sync_times_name ON reader_sync_times(account_id, name);
