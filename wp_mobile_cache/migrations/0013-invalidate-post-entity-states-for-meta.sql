-- PostMeta was refactored from a typed struct (footnotes-only) to a
-- type-erased JSON wrapper. The old WpDeserialize helper only retained
-- the `footnotes` key when writing to the cache, so existing rows hold
-- a lossy meta payload — every other plugin-registered meta key was
-- silently dropped on write.
--
-- Mark all Fresh post entity states as Stale so they get re-fetched
-- with the full meta object on the next sync.
UPDATE entity_state SET state = 3 WHERE entity_type = 0 AND state = 2;
