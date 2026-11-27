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

    assert_eq!(
        column_names(&conn, WORKING_COPIES),
        ["rowid", "uuid", "kind", "created_at"]
    );

    let columns = column_names(&conn, FIELD_SETS);
    for (column, name) in [
        (FieldSetColumn::Title, "title"),
        (FieldSetColumn::Excerpt, "excerpt"),
        (FieldSetColumn::Slug, "slug"),
        (FieldSetColumn::Status, "status"),
        (FieldSetColumn::Password, "password"),
        (FieldSetColumn::PublishDate, "publish_date"),
        (FieldSetColumn::AuthorId, "author_id"),
        (FieldSetColumn::FeaturedMediaId, "featured_media_id"),
        (FieldSetColumn::CommentStatus, "comment_status"),
        (FieldSetColumn::PingStatus, "ping_status"),
        (FieldSetColumn::Template, "template"),
        (FieldSetColumn::MenuOrder, "menu_order"),
    ] {
        assert_eq!(columns[column.as_index()], name);
    }
    assert_eq!(columns.len(), 14);

    let columns = column_names(&conn, POST_FIELDS);
    for (column, name) in [
        (PostFieldsColumn::CategoryIds, "category_ids"),
        (PostFieldsColumn::TagIds, "tag_ids"),
        (PostFieldsColumn::Format, "format"),
        (PostFieldsColumn::Sticky, "sticky"),
        (PostFieldsColumn::Meta, "meta"),
        (PostFieldsColumn::AdditionalFields, "additional_fields"),
    ] {
        assert_eq!(columns[column.as_index()], name);
    }
    assert_eq!(columns.len(), 7);

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
    insert_post(
        &mut conn,
        &simple_post("Title"),
        &self_hosted_new_post_target(),
    );

    let empty_status = conn.execute(&format!("UPDATE {FIELD_SETS} SET status = ''"), []);
    let private_with_password = conn.execute(
        &format!("UPDATE {FIELD_SETS} SET status = 'private', password = 'secret'"),
        [],
    );
    let tags_not_an_array = conn.execute(
        &format!("UPDATE {POST_FIELDS} SET tag_ids = '{{\"new\": {{\"name\": \"a\"}}}}'"),
        [],
    );
    let version_without_content =
        conn.execute(&format!("UPDATE {VERSIONS} SET content_rowid = 12345"), []);
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
        ("a version without its content", version_without_content),
        ("remote post without base", remote_post_without_base),
        (
            "two sites without a site ID at one URL",
            two_sites_without_a_site_id_at_one_url,
        ),
    ] {
        assert!(result.is_err(), "Schema should reject {case}");
    }
}

/// A working copy's versions, their rows and its remote target are deleted
/// with it, and a remote target can't exist without one. A site can't be
/// deleted while a working copy belongs to it, and a version can't be deleted
/// while another version has it as its parent.
#[test]
fn test_relationships_are_deleted_with_their_working_copy_and_not_without_one() {
    let mut conn = migrated_connection();
    let id = insert_post(
        &mut conn,
        &detailed_post(),
        &wordpress_com_target_editing(99),
    );
    let tx = conn.transaction().expect("Failed to begin transaction");
    WorkingCopyRepository
        .update_post(&tx, &id, &simple_post("Edited"), None)
        .expect("Update should succeed");
    tx.commit().expect("Commit should succeed");

    let delete_parent = conn.execute(
        &format!("DELETE FROM {VERSIONS} WHERE parent_rowid IS NULL"),
        [],
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
        delete_parent.is_err(),
        "A version must not be deleted while it's another version's parent"
    );
    assert!(
        orphan_target.is_err(),
        "A remote target must not exist without a working copy"
    );
    assert!(
        delete_targeted_site.is_err(),
        "A site must not be deleted while a working copy targets it"
    );
    for table in [VERSIONS, CONTENTS, FIELD_SETS, POST_FIELDS, REMOTE_TARGETS] {
        assert_eq!(count_rows(&conn, table), 0, "{table} should be empty");
    }
}
