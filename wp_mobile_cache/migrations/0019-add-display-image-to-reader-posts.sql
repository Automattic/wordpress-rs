-- The image to show for a Reader post: an editor's pick, else the featured image, else the image the
-- server picked as `featured_media`. See `reader_post_display_image_url` in wp_api.
ALTER TABLE `reader_posts` ADD COLUMN `display_image_url` TEXT;

-- Posts stored before this column had only their featured image, which is the best they have.
UPDATE `reader_posts` SET `display_image_url` = `featured_image` WHERE `featured_image` IS NOT NULL;
