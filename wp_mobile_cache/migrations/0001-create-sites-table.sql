CREATE TABLE `sites` (
  -- Internal DB field (auto-incrementing)
  `id` INTEGER PRIMARY KEY AUTOINCREMENT,

  -- Type of site (0 = SelfHosted, 1 = WordPressCom)
  `site_type` INTEGER NOT NULL,

  -- Reference to type-specific table (self_hosted_sites.id or wordpress_com_sites.id)
  -- Note: Not a foreign key constraint since it can point to different tables
  `mapped_site_id` INTEGER NOT NULL
) STRICT;

-- Unique constraint to prevent duplicate site mappings
CREATE UNIQUE INDEX idx_sites_unique_site_type_and_mapped_site_id ON sites(site_type, mapped_site_id);
