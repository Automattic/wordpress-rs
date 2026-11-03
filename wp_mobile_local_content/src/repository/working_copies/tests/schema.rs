//! What the schema guarantees on its own: column order, rejected states and
//! cascades.

use super::*;
use crate::db_types::working_copy_remote_target::{SiteColumn, WorkingCopyRemoteTargetColumn};

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

    let columns = column_names(&conn, REMOTE_TARGETS);
    for (column, name) in [
        (WorkingCopyRemoteTargetColumn::SiteRowid, "site_rowid"),
        (
            WorkingCopyRemoteTargetColumn::RemotePostId,
            "remote_post_id",
        ),
        (
            WorkingCopyRemoteTargetColumn::BaseModifiedGmt,
            "base_modified_gmt",
        ),
    ] {
        assert_eq!(columns[column.as_index()], name);
    }
    assert_eq!(columns.len(), 4);

    let columns = column_names(&conn, SITES);
    for (column, name) in [
        (SiteColumn::WordPressComSiteId, "wordpress_com_site_id"),
        (SiteColumn::Url, "url"),
    ] {
        assert_eq!(columns[column.as_index()], name);
    }
    assert_eq!(columns.len(), 3);
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
    let private_with_password = conn.execute(
        &format!(
            "UPDATE {WORKING_COPIES} SET status = 'private', password = 'secret' WHERE uuid = ?"
        ),
        [&id],
    );
    let tags_not_an_array = conn.execute(
        &format!("UPDATE {POST_FIELDS} SET tags = '{{\"new\": {{\"name\": \"a\"}}}}'"),
        [],
    );
    let remote_post_without_base = conn.execute(
        &format!("UPDATE {REMOTE_TARGETS} SET remote_post_id = 5"),
        [],
    );
    let two_sites_without_a_site_id_at_one_url = conn.execute(
        &format!("INSERT INTO {SITES} (url) SELECT url FROM {SITES}"),
        [],
    );

    for (case, result) in [
        ("an empty status", empty_status),
        ("a private post with a password", private_with_password),
        ("tags that aren't an array", tags_not_an_array),
        ("remote post without base", remote_post_without_base),
        (
            "two sites without a site ID at one URL",
            two_sites_without_a_site_id_at_one_url,
        ),
    ] {
        assert!(result.is_err(), "Schema should reject {case}");
    }
}

/// A working copy's fields and remote target are deleted with it, and a
/// remote target can't exist without one. A site can't be deleted while a
/// working copy belongs to it.
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
            "INSERT INTO {REMOTE_TARGETS} (working_copy_rowid, site_rowid)
             SELECT 12345, rowid FROM {SITES}"
        ),
        [],
    );
    let delete_targeted_site = conn.execute(&format!("DELETE FROM {SITES}"), []);
    conn.execute(
        &format!("DELETE FROM {WORKING_COPIES} WHERE uuid = ?"),
        [&id],
    )
    .expect("Delete should succeed");

    assert!(
        orphan_target.is_err(),
        "A remote target must not exist without a working copy"
    );
    assert!(
        delete_targeted_site.is_err(),
        "A site must not be deleted while a working copy targets it"
    );
    assert_eq!(count_rows(&conn, POST_FIELDS), 0);
    assert_eq!(count_rows(&conn, REMOTE_TARGETS), 0);
}
