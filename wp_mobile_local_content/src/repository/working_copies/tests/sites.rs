//! Matching a site, and deleting a site's working copies.

use super::*;

/// A site is matched by its WordPress.com site ID once it has one, so its
/// working copies stay with it when a self-hosted site gets a site ID
/// (connecting it through Jetpack) and when its URL changes.
#[test]
fn test_a_site_keeps_its_working_copies_when_it_gains_a_site_id_or_changes_url() {
    let mut conn = migrated_connection();
    let url_only = site("https://example.com", None);
    let connected = site("https://example.com", Some(5));
    let moved = site("https://new-domain.example", Some(5));
    let ids_for = |conn: &Connection, site: &SiteIdentity| -> Vec<WorkingCopyId> {
        WorkingCopyRepository
            .select_posts_by_site(conn, site)
            .expect("Select should succeed")
            .into_iter()
            .map(|post| post.id)
            .collect()
    };

    let draft = insert_post(&mut conn, &simple_post("Draft"), &new_post_on(&url_only));
    let found_once_connected = ids_for(&conn, &connected);
    insert_post(
        &mut conn,
        &simple_post("After connecting"),
        &new_post_on(&connected),
    );
    insert_post(
        &mut conn,
        &simple_post("After moving"),
        &new_post_on(&moved),
    );
    let stored_site = WorkingCopyRepository
        .select_post(&conn, &draft)
        .expect("Select should succeed")
        .expect("The draft should exist")
        .remote_target
        .site;

    assert_eq!(found_once_connected, vec![draft.clone()]);
    assert_eq!(ids_for(&conn, &moved).len(), 3);
    assert_eq!(stored_site, moved, "The site should have its latest URL");
    assert_eq!(count_rows(&conn, SITES), 1);
    // Once the site has a site ID, its old URL without the ID doesn't find
    // it, and a different site ID at that URL is a different site.
    assert_eq!(ids_for(&conn, &url_only), vec![]);
    assert_eq!(
        ids_for(&conn, &site("https://example.com", Some(7))),
        vec![]
    );
}

#[test]
fn test_deleting_a_site_deletes_its_working_copies_and_nothing_else() {
    let mut conn = migrated_connection();
    let removed = site("https://removed.example", Some(5));
    let kept = site("https://kept.example", None);
    insert_post(
        &mut conn,
        &simple_post("Removed post"),
        &new_post_on(&removed),
    );
    insert_page(
        &mut conn,
        &PageWorkingCopyFields {
            common: common("Removed page"),
            parent: None,
        },
        &new_post_on(&removed),
    );
    let kept_post = insert_post(&mut conn, &simple_post("Kept"), &new_post_on(&kept));

    let tx = conn.transaction().expect("Failed to begin transaction");
    let deleted = WorkingCopyRepository
        .delete_by_site(&tx, &removed)
        .expect("Delete should succeed");
    let deleted_again = WorkingCopyRepository
        .delete_by_site(&tx, &removed)
        .expect("Delete should succeed");
    tx.commit().expect("Commit should succeed");
    let remaining = WorkingCopyRepository
        .select_all_posts(&conn)
        .expect("Select should succeed");

    assert_eq!((deleted, deleted_again), (2, 0));
    assert_eq!(
        remaining
            .into_iter()
            .map(|post| post.id)
            .collect::<Vec<_>>(),
        vec![kept_post]
    );
    assert_eq!(count_rows(&conn, PAGE_FIELDS), 0);
    assert_eq!(count_rows(&conn, SITES), 1);
}
