CREATE TABLE `self_hosted_sites` (
  -- Internal DB field (auto-incrementing)
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,

  -- Site URL (unique constraint for upsert logic)
  `url` TEXT NOT NULL UNIQUE,

  -- WordPress REST API root URL
  `api_root` TEXT NOT NULL
) STRICT;

CREATE INDEX idx_self_hosted_sites_url ON self_hosted_sites(url);
