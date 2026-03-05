-- Migration 0011 added the additional_fields column to post tables but
-- didn't invalidate existing cached posts. This UPDATE should have been
-- part of 0011, but due to oversight, we need this separate migration.
--
-- Existing cached posts have NULL for additional_fields, which doesn't
-- reflect the server state. Mark all Fresh post entity states as Stale
-- so they get re-fetched on the next sync.
UPDATE entity_state SET state = 3 WHERE entity_type = 0 AND state = 2;
