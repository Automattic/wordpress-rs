-- A card in a Reader stream that isn't a post, such as Discover's recommended sites. It has a place
-- among the stream's posts, in the same order they're kept in.
CREATE TABLE `reader_stream_cards` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `stream_id` INTEGER NOT NULL,
  `sort_key` TEXT NOT NULL,           -- as reader_stream_items.sort_key, which places it among the stream's posts
  `card` TEXT NOT NULL,               -- the card, as JSON

  FOREIGN KEY (stream_id) REFERENCES reader_streams(rowid) ON DELETE CASCADE
) STRICT;

CREATE INDEX idx_reader_stream_cards_order ON reader_stream_cards(stream_id, sort_key);
