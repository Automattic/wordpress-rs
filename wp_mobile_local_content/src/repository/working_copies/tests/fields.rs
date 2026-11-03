//! Storing, updating and listing a working copy's fields.

use super::*;

#[test]
fn test_post_round_trips_every_field() {
    let mut conn = migrated_connection();
    let post = detailed_post();
    let remote_target = wordpress_com_target_editing(99);

    let id = insert_post(&mut conn, &post, &remote_target);
    let stored = WorkingCopyRepository
        .select_post(&conn, &id)
        .expect("Select should succeed")
        .expect("Inserted post should exist");

    assert_eq!(stored.id, id);
    assert_eq!(stored.fields, post);
    assert_eq!(stored.remote_target, remote_target);
}

#[test]
fn test_page_round_trips_every_field() {
    let mut conn = migrated_connection();
    let page = PageWorkingCopyFields {
        common: common("A page"),
        parent: Some(PostId(3)),
    };
    let remote_target = self_hosted_new_post_target();

    let id = insert_page(&mut conn, &page, &remote_target);
    let stored = WorkingCopyRepository
        .select_page(&conn, &id)
        .expect("Select should succeed")
        .expect("Inserted page should exist");

    assert_eq!(stored.fields, page);
    assert_eq!(stored.remote_target, remote_target);
}

#[test]
fn test_post_and_page_ids_are_not_interchangeable() {
    let mut conn = migrated_connection();
    let page_id = insert_page(
        &mut conn,
        &PageWorkingCopyFields {
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
    let remote_target = wordpress_com_target_editing(99);
    let id = insert_post(&mut conn, &detailed_post(), &remote_target);
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
    assert_eq!(after.fields, edited);
    assert_eq!(after.remote_target, remote_target);
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

/// A `Custom` status that another variant covers is saved as that variant,
/// on insert and on update, so a status always reads back the same way. Any
/// other status stays `Custom`.
#[test]
fn test_custom_status_another_variant_covers_is_saved_as_that_variant() {
    let mut conn = migrated_connection();
    let remote_target = self_hosted_new_post_target();
    let saved = insert_post(&mut conn, &simple_post("Saved"), &remote_target);
    let secret = || Some("secret".to_string());

    for (status, expected) in [
        ("draft", StatusSetting::Draft { password: secret() }),
        (
            "pending",
            StatusSetting::PendingReview { password: secret() },
        ),
        ("publish", StatusSetting::Publish { password: secret() }),
        // A scheduled post is `Publish` with a future publish date.
        ("future", StatusSetting::Publish { password: secret() }),
        // WordPress clears the password when saving a private post.
        ("private", StatusSetting::Private),
        ("", StatusSetting::Draft { password: secret() }),
        (
            "in-review",
            StatusSetting::Custom {
                status: "in-review".to_string(),
                password: secret(),
            },
        ),
    ] {
        let post = PostWorkingCopyFields {
            common: WorkingCopyCommonFields {
                status: StatusSetting::Custom {
                    status: status.to_string(),
                    password: secret(),
                },
                ..common("Custom")
            },
            ..simple_post("Custom")
        };
        let inserted = insert_post(&mut conn, &post, &remote_target);
        let tx = conn.transaction().expect("Failed to begin transaction");
        WorkingCopyRepository
            .update_post(&tx, &saved, &post)
            .expect("Update should succeed");
        tx.commit().expect("Commit should succeed");
        let status_of = |id: &WorkingCopyId| {
            WorkingCopyRepository
                .select_post(&conn, id)
                .expect("Select should succeed")
                .expect("The post should exist")
                .fields
                .common
                .status
        };

        assert_eq!(status_of(&inserted), expected, "inserting {status:?}");
        assert_eq!(status_of(&saved), expected, "updating {status:?}");
    }
}

#[test]
fn test_working_copies_are_listed_most_recently_updated_first() {
    let mut conn = migrated_connection();
    let site = self_hosted_new_post_target();
    // Created first and updated last, so creation order and update order disagree.
    let edited_last = insert_post(&mut conn, &simple_post("Edited last"), &site);
    let oldest = insert_post(&mut conn, &simple_post("Oldest"), &site);
    let newer = insert_post(&mut conn, &simple_post("Newer"), &site);
    let another_sites = insert_post(
        &mut conn,
        &simple_post("Another site's"),
        &wordpress_com_target_editing(99),
    );
    insert_page(
        &mut conn,
        &PageWorkingCopyFields {
            common: common("A page"),
            parent: None,
        },
        &site,
    );
    for (id, updated_at) in [
        (&oldest, "2026-01-01T00:00:00.000Z"),
        (&newer, "2026-02-01T00:00:00.000Z"),
        (&another_sites, "2026-02-15T00:00:00.000Z"),
        (&edited_last, "2026-03-01T00:00:00.000Z"),
    ] {
        conn.execute(
            &format!("UPDATE {WORKING_COPIES} SET updated_at = ? WHERE uuid = ?"),
            rusqlite::params![updated_at, id],
        )
        .expect("Update should succeed");
    }

    let ids = |posts: Vec<PostWorkingCopy>| -> Vec<WorkingCopyId> {
        posts.into_iter().map(|post| post.id).collect()
    };
    let by_site = WorkingCopyRepository
        .select_posts_by_site(&conn, &site.site)
        .expect("Select should succeed");
    let all = WorkingCopyRepository
        .select_all_posts(&conn)
        .expect("Select should succeed");

    assert_eq!(
        ids(by_site),
        vec![edited_last.clone(), newer.clone(), oldest.clone()]
    );
    assert_eq!(ids(all), vec![edited_last, another_sites, newer, oldest]);
}
