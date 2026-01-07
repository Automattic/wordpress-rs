-- Make title_rendered and password nullable in posts tables
-- This migration preserves existing data by backing up before dropping columns

-- 1. posts_edit_context: Make title_rendered and password nullable

-- Backup existing data
CREATE TEMPORARY TABLE posts_edit_context_backup AS
SELECT rowid, password, title_rendered FROM posts_edit_context;

-- Drop and recreate columns as nullable
ALTER TABLE posts_edit_context DROP COLUMN title_rendered;
ALTER TABLE posts_edit_context ADD COLUMN title_rendered TEXT;

ALTER TABLE posts_edit_context DROP COLUMN password;
ALTER TABLE posts_edit_context ADD COLUMN password TEXT;

-- Restore data
UPDATE posts_edit_context
SET title_rendered = (SELECT title_rendered FROM posts_edit_context_backup WHERE posts_edit_context_backup.rowid = posts_edit_context.rowid),
    password = (SELECT password FROM posts_edit_context_backup WHERE posts_edit_context_backup.rowid = posts_edit_context.rowid);

DROP TABLE posts_edit_context_backup;

-- 2. posts_view_context: Make title_rendered nullable

-- Backup existing data
CREATE TEMPORARY TABLE posts_view_context_backup AS
SELECT rowid, title_rendered FROM posts_view_context;

-- Drop and recreate column as nullable
ALTER TABLE posts_view_context DROP COLUMN title_rendered;
ALTER TABLE posts_view_context ADD COLUMN title_rendered TEXT;

-- Restore data
UPDATE posts_view_context
SET title_rendered = (SELECT title_rendered FROM posts_view_context_backup WHERE posts_view_context_backup.rowid = posts_view_context.rowid);

DROP TABLE posts_view_context_backup;

-- 3. posts_embed_context: Make title_rendered nullable

-- Backup existing data
CREATE TEMPORARY TABLE posts_embed_context_backup AS
SELECT rowid, title_rendered FROM posts_embed_context;

-- Drop and recreate column as nullable
ALTER TABLE posts_embed_context DROP COLUMN title_rendered;
ALTER TABLE posts_embed_context ADD COLUMN title_rendered TEXT;

-- Restore data
UPDATE posts_embed_context
SET title_rendered = (SELECT title_rendered FROM posts_embed_context_backup WHERE posts_embed_context_backup.rowid = posts_embed_context.rowid);

DROP TABLE posts_embed_context_backup
