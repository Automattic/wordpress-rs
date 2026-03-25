-- Existing cached posts have PostMeta that was serialized with only the
-- footnotes field. Other meta keys (e.g., jetpack_publicize_message) were
-- silently dropped. Now that PostMeta preserves all keys, mark cached
-- posts as Stale so they are re-fetched with complete meta data.
UPDATE entity_state SET state = 3 WHERE entity_type = 0 AND state = 2;
