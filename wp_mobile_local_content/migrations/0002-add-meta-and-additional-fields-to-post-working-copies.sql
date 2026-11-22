-- Post meta, as a JSON object with one value per meta key
ALTER TABLE `post_working_copy_fields` ADD COLUMN `meta` TEXT NOT NULL DEFAULT '{}'
  CHECK (json_valid(`meta`) AND json_type(`meta`) = 'object');

-- The post's fields that have no column of their own, such as fields plugins
-- add, as a JSON object with one value per key
ALTER TABLE `post_working_copy_fields` ADD COLUMN `additional_fields` TEXT NOT NULL DEFAULT '{}'
  CHECK (json_valid(`additional_fields`) AND json_type(`additional_fields`) = 'object');
