CREATE TABLE `term_relationships` (
  -- Internal DB field (auto-incrementing)
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,

  -- Site identifier (foreign key to sites table)
  `db_site_id` INTEGER NOT NULL,

  -- Object identifier (rowid of post/page/nav_menu_item/etc)
  -- Note: No FK constraint since this can reference different tables
  `object_id` INTEGER NOT NULL,

  -- WordPress term ID
  `term_id` INTEGER NOT NULL,

  -- Taxonomy type ('category', 'post_tag', or custom taxonomy)
  `taxonomy_type` TEXT NOT NULL,

  FOREIGN KEY (db_site_id) REFERENCES sites(id) ON DELETE CASCADE
) STRICT;

-- Prevent duplicate associations (same object can't have same term twice in same taxonomy)
CREATE UNIQUE INDEX idx_term_relationships_unique
  ON term_relationships(db_site_id, object_id, term_id, taxonomy_type);

-- Query: "Find all objects with taxonomy X and term Y"
CREATE INDEX idx_term_relationships_by_term
  ON term_relationships(db_site_id, taxonomy_type, term_id);

-- Query: "Find all terms for object X" (used in joins when reading posts)
CREATE INDEX idx_term_relationships_by_object
  ON term_relationships(db_site_id, object_id);
