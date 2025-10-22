//! Constraint violation and error handling tests for PostRepository.
//!
//! These tests verify that database constraints are enforced correctly
//! and that error cases are handled appropriately.

use super::posts::PostRepository;
use crate::{
    DbSite,
    test_fixtures::posts::PostBuilder,
    test_helpers::{test_db, test_site},
};
use rstest::*;
use rusqlite::Connection;
use wp_api::posts::PostId;

#[rstest]
fn test_duplicate_post_id_in_same_site_updates_on_upsert(
    mut test_db: Connection,
    test_site: DbSite,
) {
    let repo = PostRepository;
    let post_id = PostId(42);

    // Insert first post
    let post1 = PostBuilder::new()
        .with_id(post_id)
        .with_title("Original Title")
        .build();
    let rowid1 = repo.upsert(&mut test_db, &test_site, &post1).unwrap();

    // Upsert second post with same ID - should update existing post
    let post2 = PostBuilder::new()
        .with_id(post_id)
        .with_title("Updated Title")
        .build();
    let rowid2 = repo.upsert(&mut test_db, &test_site, &post2).unwrap();

    // Should return same rowid (updated existing row)
    assert_eq!(rowid1, rowid2, "Upsert should update existing post");

    // Verify only one post exists
    assert_eq!(repo.count(&test_db, &test_site).unwrap(), 1);

    // Verify the title was updated
    let retrieved = repo
        .select_by_post_id(&test_db, &test_site, post_id)
        .unwrap();
    assert_eq!(retrieved.post.title.rendered, "Updated Title");
}

#[rstest]
fn test_invalid_site_id_fails_foreign_key_constraint(mut test_db: Connection) {
    let repo = PostRepository;
    let non_existent_site = DbSite {
        row_id: crate::RowId(999),
    }; // Site doesn't exist

    let post = PostBuilder::new().build();
    let result = repo.upsert(&mut test_db, &non_existent_site, &post);

    assert!(
        result.is_err(),
        "Should fail with foreign key constraint violation"
    );

    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("FOREIGN KEY constraint failed")
            || err.to_string().contains("foreign key"),
        "Error should mention foreign key violation, got: {}",
        err
    );
}

#[rstest]
fn test_select_by_post_id_returns_error_for_non_existent_post(
    test_db: Connection,
    test_site: DbSite,
) {
    let repo = PostRepository;

    let result = repo.select_by_post_id(&test_db, &test_site, PostId(99999));

    assert!(
        result.is_err(),
        "Should return error when post doesn't exist"
    );
}

#[rstest]
fn test_select_by_rowid_returns_error_for_non_existent_rowid(
    test_db: Connection,
    test_site: DbSite,
) {
    let repo = PostRepository;

    let result = repo.select_by_rowid(&test_db, &test_site, crate::RowId(99999));

    assert!(
        result.is_err(),
        "Should return error when rowid doesn't exist"
    );
}

#[rstest]
fn test_delete_non_existent_post_returns_zero(test_db: Connection, test_site: DbSite) {
    let repo = PostRepository;

    let deleted = repo
        .delete_by_post_id(&test_db, &test_site, PostId(99999))
        .unwrap();

    assert_eq!(
        deleted, 0,
        "Should return 0 when deleting non-existent post"
    );
}

#[rstest]
fn test_count_returns_zero_for_empty_site(test_db: Connection, test_site: DbSite) {
    let repo = PostRepository;

    let count = repo.count(&test_db, &test_site).unwrap();

    assert_eq!(count, 0, "Empty site should have count of 0");
}

#[rstest]
fn test_select_all_returns_empty_for_empty_site(test_db: Connection, test_site: DbSite) {
    let repo = PostRepository;

    let posts = repo.select_all(&test_db, &test_site).unwrap();

    assert_eq!(posts.len(), 0, "Empty site should return empty vector");
}
