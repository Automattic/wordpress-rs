use rusqlite::types::{FromSqlError, Type};
use rusqlite::{OptionalExtension, Row};
use wp_api::{
    posts::{PostCommentStatus, PostFormat, PostId, PostPingStatus},
    terms::TermId,
    users::UserId,
};
use wp_mobile_sqlite::{ColumnIndex, RowExt};

use crate::{
    LocalContentError,
    db_types::{
        working_copy::{
            DbPageWorkingCopy, DbPostWorkingCopy, FeaturedImage, PageFieldsColumn, PageWorkingCopy,
            PostFieldsColumn, PostWorkingCopy, PublishDate, Tag, WorkingCopyColumn,
            WorkingCopyCommon, WorkingCopyId, WorkingCopyKind, date_from_timestamp,
        },
        working_copy_target::{
            RemotePostReference, SiteIdentity, WorkingCopyTarget, WorkingCopyTargetColumn,
        },
    },
    repository::{InTransaction, QueryExecutor},
};

const WORKING_COPIES: &str = "working_copies";
const POST_FIELDS: &str = "post_working_copy_fields";
const PAGE_FIELDS: &str = "page_working_copy_fields";
const TARGETS: &str = "working_copy_targets";

/// Repository for working copies and their targets.
///
/// A working copy spans its row in `working_copies`, the fields table for its
/// kind, and its row in `working_copy_targets`. Writes that touch more than one
/// of them require a transaction, so a working copy is never stored without
/// its fields or its target.
pub struct WorkingCopyRepository;

impl WorkingCopyRepository {
    /// Insert a post working copy and its target under a newly generated ID.
    pub fn insert_post(
        &self,
        transaction: &impl InTransaction,
        working_copy: &PostWorkingCopy,
        target: &WorkingCopyTarget,
    ) -> Result<WorkingCopyId, LocalContentError> {
        let (id, rowid) = insert_common(transaction, WorkingCopyKind::Post, &working_copy.common)?;
        transaction.execute(
            &format!(
                "INSERT INTO {POST_FIELDS} (working_copy_rowid, category_ids, tags, format, sticky)
                 VALUES (?, ?, ?, ?, ?)"
            ),
            rusqlite::params![
                rowid,
                categories_to_json(&working_copy.categories)?,
                tags_to_json(&working_copy.tags)?,
                working_copy.format.as_str(),
                working_copy.sticky,
            ],
        )?;
        insert_target(transaction, rowid, target)?;
        Ok(id)
    }

    /// Insert a page working copy and its target under a newly generated ID.
    pub fn insert_page(
        &self,
        transaction: &impl InTransaction,
        working_copy: &PageWorkingCopy,
        target: &WorkingCopyTarget,
    ) -> Result<WorkingCopyId, LocalContentError> {
        let (id, rowid) = insert_common(transaction, WorkingCopyKind::Page, &working_copy.common)?;
        transaction.execute(
            &format!("INSERT INTO {PAGE_FIELDS} (working_copy_rowid, parent_id) VALUES (?, ?)"),
            rusqlite::params![rowid, working_copy.parent.map(|parent| parent.0)],
        )?;
        insert_target(transaction, rowid, target)?;
        Ok(id)
    }

    /// Replace the fields of a post working copy and refresh its `updated_at`.
    /// The target is left unchanged.
    ///
    /// Returns `false` if no post working copy has the given ID.
    pub fn update_post(
        &self,
        transaction: &impl InTransaction,
        id: &WorkingCopyId,
        working_copy: &PostWorkingCopy,
    ) -> Result<bool, LocalContentError> {
        let Some(rowid) =
            update_common(transaction, WorkingCopyKind::Post, id, &working_copy.common)?
        else {
            return Ok(false);
        };
        transaction.execute(
            &format!(
                "UPDATE {POST_FIELDS} SET category_ids = ?, tags = ?, format = ?, sticky = ?
                 WHERE working_copy_rowid = ?"
            ),
            rusqlite::params![
                categories_to_json(&working_copy.categories)?,
                tags_to_json(&working_copy.tags)?,
                working_copy.format.as_str(),
                working_copy.sticky,
                rowid,
            ],
        )?;
        Ok(true)
    }

    /// Replace the fields of a page working copy and refresh its `updated_at`.
    /// The target is left unchanged.
    ///
    /// Returns `false` if no page working copy has the given ID.
    pub fn update_page(
        &self,
        transaction: &impl InTransaction,
        id: &WorkingCopyId,
        working_copy: &PageWorkingCopy,
    ) -> Result<bool, LocalContentError> {
        let Some(rowid) =
            update_common(transaction, WorkingCopyKind::Page, id, &working_copy.common)?
        else {
            return Ok(false);
        };
        transaction.execute(
            &format!("UPDATE {PAGE_FIELDS} SET parent_id = ? WHERE working_copy_rowid = ?"),
            rusqlite::params![working_copy.parent.map(|parent| parent.0), rowid],
        )?;
        Ok(true)
    }

    /// Select a post working copy and its target by ID.
    ///
    /// Returns `Ok(None)` if no post working copy has the given ID, including
    /// when the ID belongs to a page.
    pub fn select_post(
        &self,
        executor: &impl QueryExecutor,
        id: &WorkingCopyId,
    ) -> Result<Option<DbPostWorkingCopy>, LocalContentError> {
        let Some(common_row) = select_common(executor, WorkingCopyKind::Post, id)? else {
            return Ok(None);
        };
        let mut stmt = executor.prepare(&format!(
            "SELECT * FROM {POST_FIELDS} WHERE working_copy_rowid = ?"
        ))?;
        let (categories, tags, format, sticky) = stmt.query_row([common_row.rowid], |row| {
            use PostFieldsColumn::*;
            let categories_json: String = row.get_column(CategoryIds)?;
            let tags_json: String = row.get_column(Tags)?;
            let format: String = row.get_column(Format)?;
            Ok((
                categories_from_json(&categories_json)
                    .map_err(|e| conversion_error(CategoryIds, e))?,
                tags_from_json(&tags_json).map_err(|e| conversion_error(Tags, e))?,
                format,
                row.get_column::<bool, _>(Sticky)?,
            ))
        })?;
        let target = select_target(executor, common_row.rowid)?;

        Ok(Some(DbPostWorkingCopy {
            id: common_row.id,
            working_copy: PostWorkingCopy {
                common: common_row.common,
                categories,
                tags,
                format: format
                    .parse::<PostFormat>()
                    .unwrap_or(PostFormat::Custom(format)),
                sticky,
            },
            target,
            created_at: common_row.created_at,
            updated_at: common_row.updated_at,
        }))
    }

    /// Select a page working copy and its target by ID.
    ///
    /// Returns `Ok(None)` if no page working copy has the given ID, including
    /// when the ID belongs to a post.
    pub fn select_page(
        &self,
        executor: &impl QueryExecutor,
        id: &WorkingCopyId,
    ) -> Result<Option<DbPageWorkingCopy>, LocalContentError> {
        let Some(common_row) = select_common(executor, WorkingCopyKind::Page, id)? else {
            return Ok(None);
        };
        let mut stmt = executor.prepare(&format!(
            "SELECT * FROM {PAGE_FIELDS} WHERE working_copy_rowid = ?"
        ))?;
        let parent: Option<i64> = stmt.query_row([common_row.rowid], |row| {
            row.get_column(PageFieldsColumn::ParentId)
        })?;
        let target = select_target(executor, common_row.rowid)?;

        Ok(Some(DbPageWorkingCopy {
            id: common_row.id,
            working_copy: PageWorkingCopy {
                common: common_row.common,
                parent: parent.map(PostId),
            },
            target,
            created_at: common_row.created_at,
            updated_at: common_row.updated_at,
        }))
    }
}

// Internal helpers

/// A `working_copies` row, read before the fields table for its kind.
struct CommonRow {
    rowid: i64,
    id: WorkingCopyId,
    common: WorkingCopyCommon,
    created_at: String,
    updated_at: String,
}

fn insert_common(
    transaction: &impl InTransaction,
    kind: WorkingCopyKind,
    common: &WorkingCopyCommon,
) -> Result<(WorkingCopyId, i64), LocalContentError> {
    let id = WorkingCopyId::generate();
    let mut stmt = transaction.prepare(&format!(
        "INSERT INTO {WORKING_COPIES} (
             uuid, kind, title, content, excerpt, slug, status, password, publish_date,
             author_id, featured_media_id, comment_status, ping_status
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING rowid"
    ))?;
    let rowid = stmt.query_row(
        rusqlite::params![
            id,
            kind.as_str(),
            common.title,
            common.content,
            common.excerpt,
            common.slug,
            common.status,
            common.password,
            common.publish_date.to_column(),
            common.author.map(|author| author.0),
            common.featured_image.to_column(),
            common.comment_status.as_ref().map(comment_status_str),
            common.ping_status.as_ref().map(ping_status_str),
        ],
        |row| row.get(0),
    )?;
    Ok((id, rowid))
}

/// Returns the updated row's `rowid`, or `None` if no working copy of `kind` has the ID.
fn update_common(
    transaction: &impl InTransaction,
    kind: WorkingCopyKind,
    id: &WorkingCopyId,
    common: &WorkingCopyCommon,
) -> Result<Option<i64>, LocalContentError> {
    let mut stmt = transaction.prepare(&format!(
        "UPDATE {WORKING_COPIES} SET
             title = ?, content = ?, excerpt = ?, slug = ?, status = ?, password = ?,
             publish_date = ?, author_id = ?, featured_media_id = ?, comment_status = ?,
             ping_status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE uuid = ? AND kind = ?
         RETURNING rowid"
    ))?;
    let rowid = stmt
        .query_row(
            rusqlite::params![
                common.title,
                common.content,
                common.excerpt,
                common.slug,
                common.status,
                common.password,
                common.publish_date.to_column(),
                common.author.map(|author| author.0),
                common.featured_image.to_column(),
                common.comment_status.as_ref().map(comment_status_str),
                common.ping_status.as_ref().map(ping_status_str),
                id,
                kind.as_str(),
            ],
            |row| row.get(0),
        )
        .optional()?;
    Ok(rowid)
}

fn select_common(
    executor: &impl QueryExecutor,
    kind: WorkingCopyKind,
    id: &WorkingCopyId,
) -> Result<Option<CommonRow>, LocalContentError> {
    let mut stmt = executor.prepare(&format!(
        "SELECT * FROM {WORKING_COPIES} WHERE uuid = ? AND kind = ?"
    ))?;
    let row = stmt
        .query_row(rusqlite::params![id, kind.as_str()], common_from_row)
        .optional()?;
    Ok(row)
}

fn common_from_row(row: &Row) -> rusqlite::Result<CommonRow> {
    use WorkingCopyColumn as C;

    let publish_date: Option<i64> = row.get_column(C::PublishDate)?;
    let author: Option<i64> = row.get_column(C::AuthorId)?;
    let featured_media_id: Option<i64> = row.get_column(C::FeaturedMediaId)?;
    let comment_status: Option<String> = row.get_column(C::CommentStatus)?;
    let ping_status: Option<String> = row.get_column(C::PingStatus)?;

    Ok(CommonRow {
        rowid: row.get_column(C::Rowid)?,
        id: row.get_column(C::Uuid)?,
        common: WorkingCopyCommon {
            title: row.get_column(C::Title)?,
            content: row.get_column(C::Content)?,
            excerpt: row.get_column(C::Excerpt)?,
            slug: row.get_column(C::Slug)?,
            status: row.get_column(C::Status)?,
            password: row.get_column(C::Password)?,
            publish_date: PublishDate::from_column(publish_date)
                .map_err(|e| conversion_error(C::PublishDate, e))?,
            author: author.map(UserId),
            featured_image: FeaturedImage::from_column(featured_media_id),
            comment_status: comment_status.as_deref().map(comment_status_from_str),
            ping_status: ping_status.as_deref().map(ping_status_from_str),
        },
        created_at: row.get_column(C::CreatedAt)?,
        updated_at: row.get_column(C::UpdatedAt)?,
    })
}

fn insert_target(
    transaction: &impl InTransaction,
    rowid: i64,
    target: &WorkingCopyTarget,
) -> Result<(), LocalContentError> {
    let (site_kind, wordpress_com_site_id, self_hosted_url) = target.site.to_columns();
    transaction.execute(
        &format!(
            "INSERT INTO {TARGETS} (
                 working_copy_rowid, site_kind, wordpress_com_site_id, self_hosted_url,
                 remote_post_id, base_modified_gmt
             ) VALUES (?, ?, ?, ?, ?, ?)"
        ),
        rusqlite::params![
            rowid,
            site_kind,
            wordpress_com_site_id,
            self_hosted_url,
            target.remote_post.map(|remote| remote.post_id.0),
            target
                .remote_post
                .map(|remote| remote.base_modified_gmt.0.timestamp()),
        ],
    )?;
    Ok(())
}

fn select_target(
    executor: &impl QueryExecutor,
    rowid: i64,
) -> Result<WorkingCopyTarget, LocalContentError> {
    let mut stmt = executor.prepare(&format!(
        "SELECT * FROM {TARGETS} WHERE working_copy_rowid = ?"
    ))?;
    let target = stmt.query_row([rowid], |row| {
        use WorkingCopyTargetColumn::*;

        let site_kind: String = row.get_column(SiteKind)?;
        let site = SiteIdentity::from_columns(
            &site_kind,
            row.get_column(WordPressComSiteId)?,
            row.get_column(SelfHostedUrl)?,
        )
        .map_err(|e| conversion_error(SiteKind, e))?;

        let remote_post_id: Option<i64> = row.get_column(RemotePostId)?;
        let base_modified_gmt: Option<i64> = row.get_column(BaseModifiedGmt)?;
        let remote_post = match (remote_post_id, base_modified_gmt) {
            (Some(post_id), Some(seconds)) => Some(RemotePostReference {
                post_id: PostId(post_id),
                base_modified_gmt: date_from_timestamp(seconds)
                    .map_err(|e| conversion_error(BaseModifiedGmt, e))?,
            }),
            (None, None) => None,
            _ => {
                return Err(conversion_error(
                    RemotePostId,
                    FromSqlError::Other(
                        "remote_post_id and base_modified_gmt must both be set or both be NULL"
                            .into(),
                    ),
                ));
            }
        };

        Ok(WorkingCopyTarget { site, remote_post })
    })?;
    Ok(target)
}

fn categories_to_json(categories: &[TermId]) -> Result<String, LocalContentError> {
    let ids: Vec<i64> = categories.iter().map(|category| category.0).collect();
    serde_json::to_string(&ids).map_err(json_error)
}

fn categories_from_json(json: &str) -> Result<Vec<TermId>, FromSqlError> {
    let ids: Vec<i64> = serde_json::from_str(json).map_err(|e| FromSqlError::Other(e.into()))?;
    Ok(ids.into_iter().map(TermId).collect())
}

fn tags_to_json(tags: &[Tag]) -> Result<String, LocalContentError> {
    serde_json::to_string(tags).map_err(json_error)
}

fn tags_from_json(json: &str) -> Result<Vec<Tag>, FromSqlError> {
    serde_json::from_str(json).map_err(|e| FromSqlError::Other(e.into()))
}

fn comment_status_str(status: &PostCommentStatus) -> &str {
    match status {
        PostCommentStatus::Open => "open",
        PostCommentStatus::Closed => "closed",
        PostCommentStatus::Custom(status) => status,
    }
}

fn comment_status_from_str(status: &str) -> PostCommentStatus {
    match status {
        "open" => PostCommentStatus::Open,
        "closed" => PostCommentStatus::Closed,
        status => PostCommentStatus::Custom(status.to_string()),
    }
}

fn ping_status_str(status: &PostPingStatus) -> &str {
    match status {
        PostPingStatus::Open => "open",
        PostPingStatus::Closed => "closed",
        PostPingStatus::Custom(status) => status,
    }
}

fn ping_status_from_str(status: &str) -> PostPingStatus {
    match status {
        "open" => PostPingStatus::Open,
        "closed" => PostPingStatus::Closed,
        status => PostPingStatus::Custom(status.to_string()),
    }
}

fn json_error(err: serde_json::Error) -> LocalContentError {
    LocalContentError::Sqlite {
        err_message: format!("Failed to serialize to JSON: {err}"),
    }
}

fn conversion_error(column: impl ColumnIndex, err: FromSqlError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(column.as_index(), Type::Text, Box::new(err))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        MIGRATIONS,
        db_types::working_copy::{StatusSetting, WorkingCopyId},
    };
    use rusqlite::Connection;
    use wp_api::{
        date::WpGmtDateTime,
        media::MediaId,
        posts::{PostCommentStatus, PostPingStatus},
        wp_com::WpComSiteId,
    };
    use wp_mobile_sqlite::MigrationManager;

    fn migrated_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory database");
        MigrationManager::new(&conn, &MIGRATIONS)
            .expect("Failed to create MigrationManager")
            .perform_migrations()
            .expect("Migrations should succeed");
        conn
    }

    fn date(seconds: i64) -> WpGmtDateTime {
        date_from_timestamp(seconds).expect("Timestamp should be in range")
    }

    fn common(title: &str) -> WorkingCopyCommon {
        WorkingCopyCommon {
            title: title.to_string(),
            content: format!("<!-- wp:paragraph --><p>{title}</p><!-- /wp:paragraph -->"),
            excerpt: String::new(),
            slug: None,
            status: StatusSetting::Draft,
            password: None,
            publish_date: PublishDate::Immediately,
            author: None,
            featured_image: FeaturedImage::None,
            comment_status: None,
            ping_status: None,
        }
    }

    /// A post with every optional or enum field set away from its default.
    fn detailed_post() -> PostWorkingCopy {
        PostWorkingCopy {
            common: WorkingCopyCommon {
                excerpt: "An excerpt".to_string(),
                slug: Some("a-slug".to_string()),
                status: StatusSetting::Custom {
                    status: "in-review".to_string(),
                },
                password: Some("secret".to_string()),
                publish_date: PublishDate::At {
                    date: date(1_900_000_000),
                },
                author: Some(UserId(7)),
                featured_image: FeaturedImage::Remote {
                    media_id: MediaId(42),
                },
                comment_status: Some(PostCommentStatus::Closed),
                ping_status: Some(PostPingStatus::Custom("moderated".to_string())),
                ..common("Detailed post")
            },
            categories: vec![TermId(1), TermId(5)],
            tags: vec![
                Tag::Existing { id: TermId(3) },
                Tag::New {
                    name: "not on the site yet".to_string(),
                },
            ],
            format: PostFormat::Custom("theme-specific".to_string()),
            sticky: true,
        }
    }

    fn simple_post(title: &str) -> PostWorkingCopy {
        PostWorkingCopy {
            common: common(title),
            categories: vec![],
            tags: vec![],
            format: PostFormat::Standard,
            sticky: false,
        }
    }

    fn wordpress_com_target_editing(post_id: i64) -> WorkingCopyTarget {
        WorkingCopyTarget {
            site: SiteIdentity::WordPressCom {
                site_id: WpComSiteId(123_456_789),
            },
            remote_post: Some(RemotePostReference {
                post_id: PostId(post_id),
                base_modified_gmt: date(1_700_000_000),
            }),
        }
    }

    fn self_hosted_new_post_target() -> WorkingCopyTarget {
        WorkingCopyTarget {
            site: SiteIdentity::SelfHosted {
                url: "https://example.com".to_string(),
            },
            remote_post: None,
        }
    }

    fn insert_post(
        conn: &mut Connection,
        working_copy: &PostWorkingCopy,
        target: &WorkingCopyTarget,
    ) -> WorkingCopyId {
        let tx = conn.transaction().expect("Failed to begin transaction");
        let id = WorkingCopyRepository
            .insert_post(&tx, working_copy, target)
            .expect("Insert should succeed");
        tx.commit().expect("Commit should succeed");
        id
    }

    fn insert_page(
        conn: &mut Connection,
        working_copy: &PageWorkingCopy,
        target: &WorkingCopyTarget,
    ) -> WorkingCopyId {
        let tx = conn.transaction().expect("Failed to begin transaction");
        let id = WorkingCopyRepository
            .insert_page(&tx, working_copy, target)
            .expect("Insert should succeed");
        tx.commit().expect("Commit should succeed");
        id
    }

    fn count_rows(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("Count should succeed")
    }

    fn column_names(conn: &Connection, table: &str) -> Vec<String> {
        conn.prepare(&format!("PRAGMA table_info({table})"))
            .expect("Failed to prepare PRAGMA query")
            .query_map([], |row| row.get::<_, String>(1))
            .expect("Failed to execute PRAGMA query")
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to collect column names")
    }

    /// The repository reads columns by index, so every column enum has to match
    /// the schema's column order.
    #[test]
    fn test_column_enums_match_schema() {
        let conn = migrated_connection();

        let columns = column_names(&conn, WORKING_COPIES);
        for (column, name) in [
            (WorkingCopyColumn::Rowid, "rowid"),
            (WorkingCopyColumn::Uuid, "uuid"),
            (WorkingCopyColumn::Title, "title"),
            (WorkingCopyColumn::Content, "content"),
            (WorkingCopyColumn::Excerpt, "excerpt"),
            (WorkingCopyColumn::Slug, "slug"),
            (WorkingCopyColumn::Status, "status"),
            (WorkingCopyColumn::Password, "password"),
            (WorkingCopyColumn::PublishDate, "publish_date"),
            (WorkingCopyColumn::AuthorId, "author_id"),
            (WorkingCopyColumn::FeaturedMediaId, "featured_media_id"),
            (WorkingCopyColumn::CommentStatus, "comment_status"),
            (WorkingCopyColumn::PingStatus, "ping_status"),
            (WorkingCopyColumn::CreatedAt, "created_at"),
            (WorkingCopyColumn::UpdatedAt, "updated_at"),
        ] {
            assert_eq!(columns[column.as_index()], name);
        }
        assert_eq!(columns[2], "kind");
        assert_eq!(columns.len(), 16);

        let columns = column_names(&conn, POST_FIELDS);
        for (column, name) in [
            (PostFieldsColumn::CategoryIds, "category_ids"),
            (PostFieldsColumn::Tags, "tags"),
            (PostFieldsColumn::Format, "format"),
            (PostFieldsColumn::Sticky, "sticky"),
        ] {
            assert_eq!(columns[column.as_index()], name);
        }
        assert_eq!(columns.len(), 5);

        let columns = column_names(&conn, PAGE_FIELDS);
        assert_eq!(columns[PageFieldsColumn::ParentId.as_index()], "parent_id");
        assert_eq!(columns.len(), 2);

        let columns = column_names(&conn, TARGETS);
        for (column, name) in [
            (WorkingCopyTargetColumn::SiteKind, "site_kind"),
            (
                WorkingCopyTargetColumn::WordPressComSiteId,
                "wordpress_com_site_id",
            ),
            (WorkingCopyTargetColumn::SelfHostedUrl, "self_hosted_url"),
            (WorkingCopyTargetColumn::RemotePostId, "remote_post_id"),
            (
                WorkingCopyTargetColumn::BaseModifiedGmt,
                "base_modified_gmt",
            ),
        ] {
            assert_eq!(columns[column.as_index()], name);
        }
        assert_eq!(columns.len(), 6);
    }

    #[test]
    fn test_post_round_trips_every_field() {
        let mut conn = migrated_connection();
        let post = detailed_post();
        let target = wordpress_com_target_editing(99);

        let id = insert_post(&mut conn, &post, &target);
        let stored = WorkingCopyRepository
            .select_post(&conn, &id)
            .expect("Select should succeed")
            .expect("Inserted post should exist");

        assert_eq!(stored.id, id);
        assert_eq!(stored.working_copy, post);
        assert_eq!(stored.target, target);
    }

    #[test]
    fn test_page_round_trips_every_field() {
        let mut conn = migrated_connection();
        let page = PageWorkingCopy {
            common: common("A page"),
            parent: Some(PostId(3)),
        };
        let target = self_hosted_new_post_target();

        let id = insert_page(&mut conn, &page, &target);
        let stored = WorkingCopyRepository
            .select_page(&conn, &id)
            .expect("Select should succeed")
            .expect("Inserted page should exist");

        assert_eq!(stored.working_copy, page);
        assert_eq!(stored.target, target);
    }

    #[test]
    fn test_post_and_page_ids_are_not_interchangeable() {
        let mut conn = migrated_connection();
        let page_id = insert_page(
            &mut conn,
            &PageWorkingCopy {
                common: common("A page"),
                parent: None,
            },
            &self_hosted_new_post_target(),
        );

        let as_post = WorkingCopyRepository
            .select_post(&conn, &page_id)
            .expect("Select should succeed");
        let tx = conn.transaction().expect("Failed to begin transaction");
        let updated_as_post = WorkingCopyRepository
            .update_post(&tx, &page_id, &simple_post("Not a page"))
            .expect("Update should succeed");

        assert_eq!(as_post, None);
        assert!(!updated_as_post, "A page must not be updated as a post");
    }

    #[test]
    fn test_update_replaces_fields_and_keeps_the_target() {
        let mut conn = migrated_connection();
        let target = wordpress_com_target_editing(99);
        let id = insert_post(&mut conn, &detailed_post(), &target);
        let before = WorkingCopyRepository
            .select_post(&conn, &id)
            .expect("Select should succeed")
            .expect("Inserted post should exist");

        // Removing the password must clear the stored one.
        let edited = simple_post("Edited");
        let tx = conn.transaction().expect("Failed to begin transaction");
        let updated = WorkingCopyRepository
            .update_post(&tx, &id, &edited)
            .expect("Update should succeed");
        tx.commit().expect("Commit should succeed");
        let after = WorkingCopyRepository
            .select_post(&conn, &id)
            .expect("Select should succeed")
            .expect("Updated post should exist");
        let stored_password: Option<String> = conn
            .query_row(
                &format!("SELECT password FROM {WORKING_COPIES} WHERE uuid = ?"),
                [&id],
                |row| row.get(0),
            )
            .expect("Query should succeed");

        assert!(updated);
        assert_eq!(after.working_copy, edited);
        assert_eq!(after.target, target);
        assert_eq!(after.created_at, before.created_at);
        assert_eq!(stored_password, None);
    }

    #[test]
    fn test_update_returns_false_for_unknown_id() {
        let mut conn = migrated_connection();
        let tx = conn.transaction().expect("Failed to begin transaction");
        let updated = WorkingCopyRepository
            .update_post(
                &tx,
                &WorkingCopyId("00000000-0000-4000-8000-000000000000".to_string()),
                &simple_post("Title"),
            )
            .expect("Update should succeed");
        assert!(!updated);
    }

    /// The repository only writes consistent rows; the schema rejects the
    /// inconsistent ones in case anything else ever writes to these tables.
    #[test]
    fn test_schema_rejects_impossible_states() {
        let mut conn = migrated_connection();
        let id = insert_post(
            &mut conn,
            &simple_post("Title"),
            &self_hosted_new_post_target(),
        );

        let empty_status = conn.execute(
            &format!("UPDATE {WORKING_COPIES} SET status = '' WHERE uuid = ?"),
            [&id],
        );
        let tags_not_an_array = conn.execute(
            &format!("UPDATE {POST_FIELDS} SET tags = '{{\"new\": {{\"name\": \"a\"}}}}'"),
            [],
        );
        let remote_post_without_base =
            conn.execute(&format!("UPDATE {TARGETS} SET remote_post_id = 5"), []);
        let site_kind_without_its_id = conn.execute(
            &format!("UPDATE {TARGETS} SET site_kind = 'wordpress_com'"),
            [],
        );

        for (case, result) in [
            ("an empty status", empty_status),
            ("tags that aren't an array", tags_not_an_array),
            ("remote post without base", remote_post_without_base),
            ("site kind without its ID", site_kind_without_its_id),
        ] {
            assert!(result.is_err(), "Schema should reject {case}");
        }
    }

    /// Relationships depend on their working copy, never the other way around.
    #[test]
    fn test_relationships_are_deleted_with_their_working_copy_and_not_without_one() {
        let mut conn = migrated_connection();
        let id = insert_post(
            &mut conn,
            &detailed_post(),
            &wordpress_com_target_editing(99),
        );

        let orphan_target = conn.execute(
            &format!(
                "INSERT INTO {TARGETS} (working_copy_rowid, site_kind, self_hosted_url)
                 VALUES (12345, 'self_hosted', 'https://example.com')"
            ),
            [],
        );
        conn.execute(
            &format!("DELETE FROM {WORKING_COPIES} WHERE uuid = ?"),
            [&id],
        )
        .expect("Delete should succeed");

        assert!(
            orphan_target.is_err(),
            "A target must not exist without a working copy"
        );
        assert_eq!(count_rows(&conn, POST_FIELDS), 0);
        assert_eq!(count_rows(&conn, TARGETS), 0);
    }
}
