-- What's worked out about a post when it's stored, so both apps show the same: the summary on its
-- card, how long it takes to read, and which layout it gets in a stream. NULL for a post stored
-- before these were added, or from a source that sent too little of it to tell.
ALTER TABLE `reader_posts` ADD COLUMN `summary` TEXT;
ALTER TABLE `reader_posts` ADD COLUMN `reading_time_minutes` INTEGER;
ALTER TABLE `reader_posts` ADD COLUMN `card_type` INTEGER;
