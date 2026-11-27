//! A working copy's versions: each save with a change adds one, made from the
//! newest, sharing the rows that didn't change.

use super::*;
use crate::repository::editor_sessions::EditorSessionRepository;

fn update(conn: &mut Connection, id: &WorkingCopyId, fields: &PostWorkingCopyFields) {
    update_in_session(conn, id, fields, None);
}

fn update_in_session(
    conn: &mut Connection,
    id: &WorkingCopyId,
    fields: &PostWorkingCopyFields,
    session: Option<&EditorSessionId>,
) {
    let tx = conn.transaction().expect("Failed to begin transaction");
    assert!(
        WorkingCopyRepository
            .update_post(&tx, id, fields, session)
            .expect("Update should succeed")
    );
    tx.commit().expect("Commit should succeed");
}

fn versions(conn: &Connection, id: &WorkingCopyId) -> Vec<WorkingCopyVersion> {
    WorkingCopyRepository
        .select_versions(conn, id)
        .expect("Select should succeed")
        .expect("The working copy should exist")
}

fn with_content(fields: &PostWorkingCopyFields, content: &str) -> PostWorkingCopyFields {
    let mut fields = fields.clone();
    fields.common.content = content.to_string();
    fields
}

#[test]
fn test_each_save_with_a_change_adds_a_version_made_from_the_newest() {
    let mut conn = migrated_connection();
    let first = simple_post("First");
    let id = insert_post(&mut conn, &first, &self_hosted_new_post_target());
    let second = with_content(&first, "Second");
    let third = simple_post("Third");

    update(&mut conn, &id, &second);
    // Saving what the newest version already holds adds nothing.
    update(&mut conn, &id, &second);
    update(&mut conn, &id, &third);

    let history = versions(&conn, &id);
    let parents: Vec<_> = history
        .iter()
        .map(|version| version.parent.clone())
        .collect();
    assert_eq!(
        parents,
        [
            None,
            Some(history[0].id.clone()),
            Some(history[1].id.clone())
        ]
    );
    let fields_of = |version: &WorkingCopyVersion| {
        WorkingCopyRepository
            .select_post_version(&conn, &version.id)
            .expect("Select should succeed")
            .expect("The version should exist")
    };
    assert_eq!(
        history.iter().map(fields_of).collect::<Vec<_>>(),
        [first, second, third.clone()]
    );
    let newest = WorkingCopyRepository
        .select_post(&conn, &id)
        .expect("Select should succeed")
        .expect("The working copy should exist");
    assert_eq!(newest.fields, third);
    assert_eq!(newest.updated_at, history[2].created_at);
    assert_eq!(history[0].created_at, newest.created_at);
}

/// The content and the other fields are stored apart, so a save that changes
/// only one of them doesn't copy the other.
#[test]
fn test_a_version_shares_the_rows_that_did_not_change() {
    let mut conn = migrated_connection();
    let first = detailed_post();
    let id = insert_post(&mut conn, &first, &self_hosted_new_post_target());

    let content_changed = with_content(&first, "New content");
    update(&mut conn, &id, &content_changed);
    let mut title_changed = content_changed.clone();
    title_changed.common.title = "New title".to_string();
    update(&mut conn, &id, &title_changed);

    assert_eq!(versions(&conn, &id).len(), 3);
    assert_eq!(count_rows(&conn, CONTENTS), 2);
    assert_eq!(count_rows(&conn, FIELD_SETS), 2);
    assert_eq!(count_rows(&conn, POST_FIELDS), 2);
}

#[test]
fn test_a_version_records_the_editor_session_it_was_made_in() {
    let mut conn = migrated_connection();
    let id = insert_post(
        &mut conn,
        &simple_post("First"),
        &self_hosted_new_post_target(),
    );
    let session = EditorSessionRepository
        .insert(&conn, &id, None)
        .expect("Insert should succeed");

    update_in_session(
        &mut conn,
        &id,
        &simple_post("In the editor"),
        Some(&session),
    );
    let missing = EditorSessionId("not-a-session".to_string());
    let tx = conn.transaction().expect("Failed to begin transaction");
    let in_missing_session =
        WorkingCopyRepository.update_post(&tx, &id, &simple_post("Lost"), Some(&missing));
    drop(tx);

    let sessions: Vec<_> = versions(&conn, &id)
        .into_iter()
        .map(|version| version.editor_session_id)
        .collect();
    assert_eq!(sessions, [None, Some(session)]);
    assert_eq!(
        in_missing_session,
        Err(LocalContentError::EditorSessionNotFound { id: missing })
    );
}

#[test]
fn test_versions_are_read_only_as_their_working_copy_kind() {
    let mut conn = migrated_connection();
    let page_id = insert_page(
        &mut conn,
        &PageWorkingCopyFields {
            common: common("A page"),
            parent: None,
        },
        &self_hosted_new_post_target(),
    );
    let page_version = versions(&conn, &page_id).remove(0);

    assert_eq!(
        WorkingCopyRepository
            .select_post_version(&conn, &page_version.id)
            .expect("Select should succeed"),
        None
    );
    assert!(
        WorkingCopyRepository
            .select_page_version(&conn, &page_version.id)
            .expect("Select should succeed")
            .is_some()
    );
    assert_eq!(
        WorkingCopyRepository
            .select_versions(&conn, &WorkingCopyId("missing".to_string()))
            .expect("Select should succeed"),
        None
    );
}
