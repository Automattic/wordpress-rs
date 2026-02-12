CREATE TABLE `wordpress_com_sites` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `site_id` INTEGER NOT NULL UNIQUE
) STRICT;

CREATE INDEX idx_wordpress_com_sites_site_id ON wordpress_com_sites(site_id);
