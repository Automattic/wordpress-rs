use crate::{
    DbTable, RowId, SqliteDbError,
    context::{EditContext, IsContext},
    db_types::{
        db_site::DbSite,
        helpers::{
            deserialize_json_value, get_id, get_optional_id, parse_datetime, parse_enum,
            serialize_value_to_json,
        },
        media::{DbMediaWithEditContext, MediaEditContextColumn},
        row_ext::RowExt,
    },
    entity::{EntityId, FullEntity},
    repository::{QueryExecutor, TransactionManager},
};
use rusqlite::{OptionalExtension, Row};
use std::{collections::HashMap, marker::PhantomData, sync::Arc};
use wp_api::{
    media::{
        MediaCaptionWithEditContext, MediaDescriptionWithEditContext, MediaId, MediaWithEditContext,
    },
    posts::{PostGuidWithEditContext, PostTitleWithEditContext},
    prelude::WpGmtDateTime,
};

/// Entity-specific context trait for Media.
///
/// Mirrors `PostContext` but omits the term-relationship preload: media has no
/// categories or tags, so `from_row` takes only `&Row`.
pub trait MediaContext: IsContext {
    type Media;
    type DbMedia;

    fn table() -> DbTable;
    fn from_row(row: &Row) -> Result<Self::DbMedia, SqliteDbError>;
    fn rowid(db: &Self::DbMedia) -> RowId;
}

/// Repository for managing media in the database.
///
/// Generic over `MediaContext`. Phase 0 only implements `EditContext`.
pub struct MediaRepository<C: MediaContext> {
    _phantom: PhantomData<C>,
}

impl<C: MediaContext> Default for MediaRepository<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: MediaContext> MediaRepository<C> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }

    pub fn table_name() -> &'static str {
        C::table().table_name()
    }

    /// Select a media row by its EntityId.
    ///
    /// Returns an error if the EntityId's table doesn't match this repository's context.
    pub fn select_by_entity_id(
        &self,
        executor: &impl QueryExecutor,
        entity_id: &EntityId,
    ) -> Result<Option<FullEntity<C::DbMedia>>, SqliteDbError> {
        entity_id.validate_table(C::table())?;

        let sql = format!(
            "SELECT * FROM {} WHERE db_site_id = ? AND rowid = ?",
            Self::table_name()
        );
        let mut stmt = executor.prepare(&sql)?;
        let db_media = stmt
            .query_row([entity_id.db_site.row_id, entity_id.rowid], |row| {
                C::from_row(row).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
            })
            .optional()
            .map_err(SqliteDbError::from)?;

        Ok(db_media.map(|db_media| {
            let entity_id = Arc::new(*entity_id);
            FullEntity::new(entity_id, db_media)
        }))
    }

    /// Select all media rows for a given site.
    ///
    /// Unlike posts, media has no equivalent of status-filtered queries at the repository
    /// level (filtering happens at the API layer), so this is implemented directly without
    /// a `select_by_filter` indirection.
    pub fn select_all(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
    ) -> Result<Vec<FullEntity<C::DbMedia>>, SqliteDbError> {
        let sql = format!("SELECT * FROM {} WHERE db_site_id = ?", Self::table_name());
        let mut stmt = executor.prepare(&sql)?;
        let rows = stmt
            .query_map([site.row_id], |row| {
                C::from_row(row).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(SqliteDbError::from)?;

        Ok(rows
            .into_iter()
            .map(|db_media| {
                let rowid = C::rowid(&db_media);
                let entity_id = Arc::new(EntityId::new(*site, C::table(), rowid));
                FullEntity::new(entity_id, db_media)
            })
            .collect())
    }

    /// Select a media row by its WordPress media ID for a given site.
    pub fn select_by_media_id(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        media_id: MediaId,
    ) -> Result<Option<FullEntity<C::DbMedia>>, SqliteDbError> {
        let sql = format!(
            "SELECT * FROM {} WHERE db_site_id = ? AND id = ?",
            Self::table_name()
        );
        let mut stmt = executor.prepare(&sql)?;
        let db_media = stmt
            .query_row(rusqlite::params![site.row_id, media_id.0], |row| {
                C::from_row(row).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
            })
            .optional()
            .map_err(SqliteDbError::from)?;

        Ok(db_media.map(|db_media| {
            let rowid = C::rowid(&db_media);
            let entity_id = Arc::new(EntityId::new(*site, C::table(), rowid));
            FullEntity::new(entity_id, db_media)
        }))
    }

    /// Select `modified_gmt` timestamps for multiple media items by their WordPress media IDs.
    ///
    /// Lightweight query used for staleness detection; media not present in the cache are
    /// omitted from the result.
    pub fn select_modified_gmt_by_ids(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        media_ids: &[MediaId],
    ) -> Result<HashMap<MediaId, WpGmtDateTime>, SqliteDbError> {
        if media_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let ids_str = media_ids
            .iter()
            .map(|id| id.0.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            "SELECT id, modified_gmt FROM {} WHERE db_site_id = ? AND id IN ({})",
            Self::table_name(),
            ids_str
        );

        let mut stmt = executor.prepare(&sql)?;
        let rows = stmt.query_map([site.row_id], |row| {
            let id: i64 = row.get(0)?;
            let modified_gmt_str: String = row.get(1)?;
            Ok((id, modified_gmt_str))
        })?;

        Ok(rows
            .filter_map(|row_result| {
                row_result.ok().and_then(|(id, modified_gmt_str)| {
                    modified_gmt_str
                        .parse::<WpGmtDateTime>()
                        .ok()
                        .map(|modified_gmt| (MediaId(id), modified_gmt))
                })
            })
            .collect())
    }

    /// Delete a media row by its EntityId.
    ///
    /// Returns the number of rows deleted (0 or 1). Unlike posts, no term relationships
    /// are involved.
    pub fn delete_by_entity_id(
        &self,
        executor: &impl QueryExecutor,
        entity_id: &EntityId,
    ) -> Result<usize, SqliteDbError> {
        entity_id.validate_table(C::table())?;

        let sql = format!(
            "SELECT id FROM {} WHERE db_site_id = ? AND rowid = ?",
            Self::table_name()
        );
        let mut stmt = executor.prepare(&sql)?;
        let media_id = stmt
            .query_row([entity_id.db_site.row_id, entity_id.rowid], |row| {
                row.get::<_, i64>(0)
            })
            .optional()
            .map_err(SqliteDbError::from)?;

        match media_id {
            Some(id) => self.delete_by_media_id(executor, &entity_id.db_site, MediaId(id)),
            None => Ok(0),
        }
    }

    /// Delete a media row by its WordPress media ID for a given site.
    pub fn delete_by_media_id(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        media_id: MediaId,
    ) -> Result<usize, SqliteDbError> {
        let sql = format!(
            "DELETE FROM {} WHERE db_site_id = ? AND id = ?",
            Self::table_name()
        );
        executor.execute(&sql, rusqlite::params![site.row_id, media_id.0])
    }

    /// Get the total count of media rows for a given site.
    pub fn count(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
    ) -> Result<i64, SqliteDbError> {
        let sql = format!(
            "SELECT COUNT(*) FROM {} WHERE db_site_id = ?",
            Self::table_name()
        );
        let mut stmt = executor.prepare(&sql)?;
        stmt.query_row([site.row_id], |row| row.get(0))
            .map_err(SqliteDbError::from)
    }
}

impl MediaContext for EditContext {
    type Media = MediaWithEditContext;
    type DbMedia = DbMediaWithEditContext;

    fn table() -> DbTable {
        DbTable::MediaEditContext
    }

    fn from_row(row: &Row) -> Result<Self::DbMedia, SqliteDbError> {
        use MediaEditContextColumn::*;

        let row_id: RowId = row.get_column(Rowid)?;
        let db_site_id: RowId = row.get_column(MediaEditContextColumn::DbSiteId)?;

        // media_details is stored as raw JSON text and reconstructed into a `Box<RawValue>`.
        // Parsing here keeps the on-disk payload byte-for-byte identical to what was written,
        // while still validating that the stored value is syntactically valid JSON.
        let media_details_json: String = row.get_column(MediaDetails)?;
        let media_details_payload: Box<serde_json::value::RawValue> =
            serde_json::from_str(&media_details_json).map_err(|e| {
                SqliteDbError::SqliteError(format!("Failed to parse media_details JSON: {}", e))
            })?;
        let media_details = Arc::new(wp_api::media::MediaDetails {
            payload: media_details_payload,
        });

        // missing_image_sizes is a non-optional Vec<String>, persisted as a JSON array.
        let missing_image_sizes_json: String = row.get_column(MissingImageSizes)?;
        let missing_image_sizes: Vec<String> = serde_json::from_str(&missing_image_sizes_json)
            .map_err(|e| {
                SqliteDbError::SqliteError(format!(
                    "Failed to parse missing_image_sizes JSON: {}",
                    e
                ))
            })?;

        let media = MediaWithEditContext {
            id: get_id(row, Id)?,
            date: row.get_column(Date)?,
            date_gmt: parse_datetime(row, DateGmt)?,
            guid: PostGuidWithEditContext {
                raw: row.get_column(GuidRaw)?,
                rendered: row.get_column(GuidRendered)?,
            },
            link: row.get_column(Link)?,
            modified: row.get_column(Modified)?,
            modified_gmt: parse_datetime(row, ModifiedGmt)?,
            slug: row.get_column(Slug)?,
            status: parse_enum(row, Status)?,
            post_type: row.get_column(PostType)?,
            password: row.get_column(Password)?,
            permalink_template: row.get_column(PermalinkTemplate)?,
            generated_slug: row.get_column(GeneratedSlug)?,
            title: PostTitleWithEditContext {
                raw: row.get_column(TitleRaw)?,
                rendered: row.get_column(TitleRendered)?,
            },
            author: get_id(row, Author)?,
            comment_status: parse_enum(row, CommentStatus)?,
            ping_status: parse_enum(row, PingStatus)?,
            template: row.get_column(Template)?,
            alt_text: row.get_column(AltText)?,
            caption: MediaCaptionWithEditContext {
                raw: row.get_column(CaptionRaw)?,
                rendered: row.get_column(CaptionRendered)?,
            },
            description: MediaDescriptionWithEditContext {
                raw: row.get_column(DescriptionRaw)?,
                rendered: row.get_column(DescriptionRendered)?,
            },
            media_type: parse_enum(row, MediaType)?,
            mime_type: row.get_column(MimeType)?,
            media_details,
            post_id: get_optional_id(row, PostId)?,
            source_url: row.get_column(SourceUrl)?,
            missing_image_sizes,
        };

        // additional_fields is part of the schema for future use but is not currently
        // exposed on MediaWithEditContext. We still read+discard it (via the migration
        // column) so writes that include it round-trip cleanly via the upsert column list.
        let _additional_fields: Option<wp_api::WpAdditionalFields> =
            deserialize_json_value(row.get_column(AdditionalFields)?)?;

        Ok(DbMediaWithEditContext {
            row_id,
            db_site_id,
            media,
            last_fetched_at: row.get_column(LastFetchedAt)?,
        })
    }

    fn rowid(db: &Self::DbMedia) -> RowId {
        db.row_id
    }
}

impl MediaRepository<EditContext> {
    /// Upsert a media row with edit context (atomic transaction).
    ///
    /// Uses a transaction even though only one table is touched, matching the posts
    /// pattern and leaving room for future composability (e.g. associated metadata
    /// tables) without changing the public signature.
    pub fn upsert(
        &self,
        transaction_manager: &mut impl TransactionManager,
        site: &DbSite,
        media: &MediaWithEditContext,
    ) -> Result<EntityId, SqliteDbError> {
        let tx = transaction_manager.transaction()?;

        let missing_image_sizes_json =
            serde_json::to_string(&media.missing_image_sizes).map_err(|e| {
                SqliteDbError::SqliteError(format!(
                    "Failed to serialize missing_image_sizes: {}",
                    e
                ))
            })?;

        let upsert_sql = format!(
            r#"
            INSERT INTO {} (
                db_site_id, id, date, date_gmt, link, modified, modified_gmt, slug, status, post_type,
                password, permalink_template, generated_slug, author, comment_status, ping_status,
                template, alt_text, media_type, mime_type, source_url, post_id, missing_image_sizes,
                guid_raw, guid_rendered, title_raw, title_rendered,
                caption_raw, caption_rendered, description_raw, description_rendered,
                media_details, additional_fields
            ) VALUES (
                :db_site_id, :id, :date, :date_gmt, :link, :modified, :modified_gmt, :slug, :status, :post_type,
                :password, :permalink_template, :generated_slug, :author, :comment_status, :ping_status,
                :template, :alt_text, :media_type, :mime_type, :source_url, :post_id, :missing_image_sizes,
                :guid_raw, :guid_rendered, :title_raw, :title_rendered,
                :caption_raw, :caption_rendered, :description_raw, :description_rendered,
                :media_details, :additional_fields
            )
            ON CONFLICT(db_site_id, id) DO UPDATE SET
                date = excluded.date,
                date_gmt = excluded.date_gmt,
                link = excluded.link,
                modified = excluded.modified,
                modified_gmt = excluded.modified_gmt,
                slug = excluded.slug,
                status = excluded.status,
                post_type = excluded.post_type,
                password = excluded.password,
                permalink_template = excluded.permalink_template,
                generated_slug = excluded.generated_slug,
                author = excluded.author,
                comment_status = excluded.comment_status,
                ping_status = excluded.ping_status,
                template = excluded.template,
                alt_text = excluded.alt_text,
                media_type = excluded.media_type,
                mime_type = excluded.mime_type,
                source_url = excluded.source_url,
                post_id = excluded.post_id,
                missing_image_sizes = excluded.missing_image_sizes,
                guid_raw = excluded.guid_raw,
                guid_rendered = excluded.guid_rendered,
                title_raw = excluded.title_raw,
                title_rendered = excluded.title_rendered,
                caption_raw = excluded.caption_raw,
                caption_rendered = excluded.caption_rendered,
                description_raw = excluded.description_raw,
                description_rendered = excluded.description_rendered,
                media_details = excluded.media_details,
                additional_fields = excluded.additional_fields,
                last_fetched_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            RETURNING rowid
            "#,
            Self::table_name()
        );

        let no_additional_fields: Option<wp_api::WpAdditionalFields> = None;

        let media_rowid: i64 = tx
            .query_row(
                &upsert_sql,
                rusqlite::named_params! {
                    ":db_site_id": site.row_id,
                    ":id": media.id.0,
                    ":date": media.date,
                    ":date_gmt": media.date_gmt.to_string(),
                    ":link": media.link,
                    ":modified": media.modified,
                    ":modified_gmt": media.modified_gmt.to_string(),
                    ":slug": media.slug,
                    ":status": media.status.to_string(),
                    ":post_type": media.post_type,
                    ":password": media.password.clone(),
                    ":permalink_template": media.permalink_template,
                    ":generated_slug": media.generated_slug,
                    ":author": media.author.0,
                    ":comment_status": media.comment_status.to_string(),
                    ":ping_status": media.ping_status.to_string(),
                    ":template": media.template,
                    ":alt_text": media.alt_text,
                    ":media_type": media.media_type.to_string(),
                    ":mime_type": media.mime_type,
                    ":source_url": media.source_url,
                    ":post_id": media.post_id.map(|p| p.0),
                    ":missing_image_sizes": missing_image_sizes_json,
                    ":guid_raw": media.guid.raw,
                    ":guid_rendered": media.guid.rendered,
                    ":title_raw": media.title.raw,
                    ":title_rendered": media.title.rendered,
                    ":caption_raw": media.caption.raw,
                    ":caption_rendered": media.caption.rendered,
                    ":description_raw": media.description.raw,
                    ":description_rendered": media.description.rendered,
                    ":media_details": media.media_details.payload.get(),
                    ":additional_fields": serialize_value_to_json(&no_additional_fields)?,
                },
                |row| row.get(0),
            )
            .map_err(SqliteDbError::from)?;
        let media_rowid = RowId(media_rowid);

        tx.commit().map_err(SqliteDbError::from)?;
        Ok(EntityId::new(*site, EditContext::table(), media_rowid))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        MigrationManager, db_types::self_hosted_site::SelfHostedSite,
        repository::sites::SiteRepository, test_fixtures::media::MediaBuilder,
    };
    use rusqlite::Connection;
    use wp_api::{
        media::{MediaId, MediaStatus},
        posts::PostTitleWithEditContext,
    };

    fn setup_db() -> (Connection, DbSite) {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        let mut mgr = MigrationManager::new(&conn).expect("migration manager");
        mgr.perform_migrations().expect("migrations");
        let db_site = SiteRepository
            .upsert_self_hosted_site(
                &mut conn,
                &SelfHostedSite {
                    url: "https://test.local".into(),
                    api_root: "https://test.local/wp-json".into(),
                },
            )
            .expect("upsert site")
            .db_site;
        (conn, db_site)
    }

    #[test]
    fn select_by_media_id_returns_none_when_empty() {
        let (conn, site) = setup_db();
        let repo = MediaRepository::<EditContext>::new();
        let result = repo
            .select_by_media_id(&conn, &site, MediaId(42))
            .expect("select returns ok");
        assert!(result.is_none(), "expected None on empty table");
    }

    #[test]
    fn upsert_then_select_by_media_id_round_trips_fields() {
        let (mut conn, site) = setup_db();
        let repo = MediaRepository::<EditContext>::new();
        let media = MediaBuilder::minimal()
            .with_id(42)
            .with_slug("media-42")
            .build();

        repo.upsert(&mut conn, &site, &media).expect("upsert");
        let retrieved = repo
            .select_by_media_id(&conn, &site, MediaId(42))
            .expect("select")
            .expect("row should exist");

        assert_eq!(retrieved.data.media.id, MediaId(42));
        assert_eq!(retrieved.data.media.slug, "media-42");
        assert_eq!(retrieved.data.media.status, MediaStatus::Inherit);
    }

    #[test]
    fn upsert_twice_with_same_id_updates_in_place_no_duplicate_rows() {
        let (mut conn, site) = setup_db();
        let repo = MediaRepository::<EditContext>::new();

        let first = MediaBuilder::minimal().with_id(7).build();
        repo.upsert(&mut conn, &site, &first).expect("first upsert");

        let mut second = MediaBuilder::minimal().with_id(7).build();
        second.title = PostTitleWithEditContext {
            raw: Some("Updated raw".into()),
            rendered: "Updated rendered".into(),
        };
        repo.upsert(&mut conn, &site, &second)
            .expect("second upsert");

        assert_eq!(repo.count(&conn, &site).expect("count"), 1);
        let retrieved = repo
            .select_by_media_id(&conn, &site, MediaId(7))
            .expect("select")
            .expect("row should exist");
        assert_eq!(retrieved.data.media.title.rendered, "Updated rendered");
        assert_eq!(
            retrieved.data.media.title.raw,
            Some("Updated raw".to_string())
        );
    }

    #[test]
    fn count_returns_number_of_rows_for_site() {
        let (mut conn, site) = setup_db();
        let repo = MediaRepository::<EditContext>::new();
        for id in 1..=3 {
            let media = MediaBuilder::minimal().with_id(id).build();
            repo.upsert(&mut conn, &site, &media).expect("upsert");
        }
        assert_eq!(repo.count(&conn, &site).expect("count"), 3);
    }

    #[test]
    fn delete_by_media_id_returns_one_and_removes_row() {
        let (mut conn, site) = setup_db();
        let repo = MediaRepository::<EditContext>::new();
        let media = MediaBuilder::minimal().with_id(1).build();
        repo.upsert(&mut conn, &site, &media).expect("upsert");

        let deleted = repo
            .delete_by_media_id(&conn, &site, MediaId(1))
            .expect("delete");
        assert_eq!(deleted, 1);
        assert!(
            repo.select_by_media_id(&conn, &site, MediaId(1))
                .expect("select")
                .is_none()
        );
    }

    #[test]
    fn delete_by_media_id_non_existent_returns_zero() {
        let (conn, site) = setup_db();
        let repo = MediaRepository::<EditContext>::new();
        let deleted = repo
            .delete_by_media_id(&conn, &site, MediaId(999))
            .expect("delete");
        assert_eq!(deleted, 0);
    }

    #[test]
    fn select_by_entity_id_rejects_wrong_table_name() {
        let (conn, site) = setup_db();
        let repo = MediaRepository::<EditContext>::new();
        let bad_entity_id = EntityId::new(site, DbTable::PostsEditContext, RowId(1));
        let result = repo.select_by_entity_id(&conn, &bad_entity_id);
        match result {
            Err(SqliteDbError::TableNameMismatch { expected, actual }) => {
                assert_eq!(expected, DbTable::MediaEditContext);
                assert_eq!(actual, DbTable::PostsEditContext);
            }
            Err(other) => panic!("expected TableNameMismatch error, got error {:?}", other),
            Ok(_) => panic!("expected TableNameMismatch error, got Ok(_)"),
        }
    }
}
