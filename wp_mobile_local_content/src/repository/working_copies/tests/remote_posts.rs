//! One working copy per remote post, and recording pushes.

use super::*;

#[test]
fn test_a_remote_post_has_at_most_one_working_copy() {
    let mut conn = migrated_connection();
    let self_hosted =
        |url: &str, remote_post: Option<RemotePostReference>| WorkingCopyRemoteTarget {
            site: site(url, None),
            remote_post,
        };
    let editing_post_5 = Some(RemotePostReference {
        post_id: PostId(5),
        base_modified_gmt: date(1_700_000_000),
    });
    insert_post(
        &mut conn,
        &simple_post("First"),
        &self_hosted("https://example.com", editing_post_5),
    );

    let insert = |conn: &mut Connection, remote_target: &WorkingCopyRemoteTarget| {
        let tx = conn.transaction().expect("Failed to begin transaction");
        let result = WorkingCopyRepository.insert_post(&tx, &simple_post("Another"), remote_target);
        // A failed insert has already written the working copy's row, so
        // it's rolled back by dropping the transaction rather than committed.
        if result.is_ok() {
            tx.commit().expect("Commit should succeed");
        }
        result
    };
    let same_post = insert(
        &mut conn,
        &self_hosted("https://example.com", editing_post_5),
    );
    let same_post_differently_written_url = insert(
        &mut conn,
        &self_hosted("HTTPS://EXAMPLE.COM/", editing_post_5),
    );
    let new_posts = [
        insert(&mut conn, &self_hosted("https://example.com", None)),
        insert(&mut conn, &self_hosted("https://example.com", None)),
    ];
    let same_post_id_on_another_site = insert(
        &mut conn,
        &self_hosted("https://other.example.com", editing_post_5),
    );

    let duplicate = Err(LocalContentError::RemotePostAlreadyHasWorkingCopy { post_id: PostId(5) });
    assert_eq!(same_post, duplicate);
    assert_eq!(same_post_differently_written_url, duplicate);
    for result in new_posts {
        assert!(result.is_ok(), "New posts must not collide: {result:?}");
    }
    assert!(
        same_post_id_on_another_site.is_ok(),
        "Another site's post with the same ID is a different post: {same_post_id_on_another_site:?}"
    );
    assert_eq!(count_rows(&conn, SITES), 2);
}

#[test]
fn test_recording_a_push_points_the_working_copy_at_the_pushed_post() {
    let mut conn = migrated_connection();
    let new_post = insert_post(
        &mut conn,
        &simple_post("New"),
        &self_hosted_new_post_target(),
    );
    let editing_99 = insert_post(
        &mut conn,
        &simple_post("Editing 99"),
        &wordpress_com_target_editing(99),
    );
    let before = WorkingCopyRepository
        .select_post(&conn, &new_post)
        .expect("Select should succeed")
        .expect("The new post should exist");
    let pushed_as_7 = RemotePostReference {
        post_id: PostId(7),
        base_modified_gmt: date(1_800_000_000),
    };
    let pushed_99_again = RemotePostReference {
        post_id: PostId(99),
        base_modified_gmt: date(1_800_000_000),
    };

    let tx = conn.transaction().expect("Failed to begin transaction");
    let recorded_new_post = WorkingCopyRepository
        .record_post_push(&tx, &new_post, pushed_as_7)
        .expect("Recording should succeed");
    let recorded_99 = WorkingCopyRepository
        .record_post_push(&tx, &editing_99, pushed_99_again)
        .expect("Recording should succeed");
    tx.commit().expect("Commit should succeed");
    let after = WorkingCopyRepository
        .select_post(&conn, &new_post)
        .expect("Select should succeed")
        .expect("The new post should exist");
    let editing_99_target = WorkingCopyRepository
        .select_post(&conn, &editing_99)
        .expect("Select should succeed")
        .expect("The post should exist")
        .remote_target;

    assert!(recorded_new_post && recorded_99);
    assert_eq!(after.remote_target.remote_post, Some(pushed_as_7));
    assert_eq!(after.remote_target.site, before.remote_target.site);
    assert_eq!(after.fields, before.fields);
    assert_eq!(after.updated_at, before.updated_at);
    assert_eq!(editing_99_target.remote_post, Some(pushed_99_again));
}

#[test]
fn test_recording_a_push_refuses_what_would_break_the_target() {
    let mut conn = migrated_connection();
    let editing_99 = insert_post(
        &mut conn,
        &simple_post("Editing 99"),
        &wordpress_com_target_editing(99),
    );
    let new_post = insert_post(
        &mut conn,
        &simple_post("New"),
        &new_post_on(&wordpress_com_target_editing(99).site),
    );
    let page = insert_page(
        &mut conn,
        &PageWorkingCopyFields {
            common: common("A page"),
            parent: None,
        },
        &self_hosted_new_post_target(),
    );
    let pushed_as = |post_id: i64| RemotePostReference {
        post_id: PostId(post_id),
        base_modified_gmt: date(1_800_000_000),
    };

    let tx = conn.transaction().expect("Failed to begin transaction");
    let to_another_post = WorkingCopyRepository.record_post_push(&tx, &editing_99, pushed_as(5));
    let to_a_post_with_a_working_copy =
        WorkingCopyRepository.record_post_push(&tx, &new_post, pushed_as(99));
    let page_as_a_post = WorkingCopyRepository.record_post_push(&tx, &page, pushed_as(5));
    tx.commit().expect("Commit should succeed");
    let new_post_target = WorkingCopyRepository
        .select_post(&conn, &new_post)
        .expect("Select should succeed")
        .expect("The new post should exist")
        .remote_target;

    assert_eq!(
        to_another_post,
        Err(LocalContentError::PushedToAnotherRemotePost {
            remote_post_id: PostId(99),
            pushed_post_id: PostId(5),
        })
    );
    assert_eq!(
        to_a_post_with_a_working_copy,
        Err(LocalContentError::RemotePostAlreadyHasWorkingCopy {
            post_id: PostId(99)
        })
    );
    assert_eq!(new_post_target.remote_post, None);
    assert_eq!(page_as_a_post, Ok(false));
}
