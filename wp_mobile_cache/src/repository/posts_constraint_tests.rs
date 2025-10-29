//! Constraint violation and error handling tests for PostRepository.
//!
//! These tests verify that database constraints are enforced correctly
//! and that error cases are handled appropriately.

use crate::{
    DbSite, RowId,
    test_fixtures::{TestContext, posts::PostBuilder, test_ctx},
};
use rstest::*;
use wp_api::posts::PostId;

#[rstest]
fn test_duplicate_post_id_in_same_site_updates_on_upsert(mut test_ctx: TestContext) {
    let post_id = PostId(42);

    // Insert first post
    let post1 = PostBuilder::minimal()
        .with_id(42)
        .with_title("Original Title")
        .build();
    let rowid1 = test_ctx
        .post_repo
        .upsert(&mut test_ctx.conn, &test_ctx.site, &post1)
        .unwrap();

    // Upsert second post with same ID - should update existing post
    let post2 = PostBuilder::minimal()
        .with_id(42)
        .with_title("Updated Title")
        .build();
    let rowid2 = test_ctx
        .post_repo
        .upsert(&mut test_ctx.conn, &test_ctx.site, &post2)
        .unwrap();

    // Should return same rowid (updated existing row)
    assert_eq!(rowid1, rowid2, "Upsert should update existing post");

    // Verify only one post exists
    assert_eq!(
        test_ctx
            .post_repo
            .count(&test_ctx.conn, &test_ctx.site)
            .unwrap(),
        1
    );

    // Verify the title was updated
    let retrieved = test_ctx
        .post_repo
        .select_by_post_id(&test_ctx.conn, &test_ctx.site, post_id)
        .unwrap()
        .expect("Post should exist");
    assert_eq!(retrieved.post.title.rendered, "Updated Title");
}

#[rstest]
fn test_invalid_site_id_fails_foreign_key_constraint(mut test_ctx: TestContext) {
    let non_existent_site = DbSite { row_id: RowId(999) }; // Site doesn't exist

    let post = PostBuilder::minimal().build();
    let result = test_ctx
        .post_repo
        .upsert(&mut test_ctx.conn, &non_existent_site, &post);

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
fn test_select_by_post_id_returns_none_for_non_existent_post(test_ctx: TestContext) {
    let result = test_ctx
        .post_repo
        .select_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(99999))
        .unwrap();

    assert!(
        result.is_none(),
        "Should return None when post doesn't exist"
    );
}

#[rstest]
fn test_select_by_rowid_returns_none_for_non_existent_rowid(test_ctx: TestContext) {
    let result = test_ctx
        .post_repo
        .select_by_rowid(&test_ctx.conn, &test_ctx.site, RowId(99999))
        .unwrap();

    assert!(
        result.is_none(),
        "Should return None when rowid doesn't exist"
    );
}

#[rstest]
fn test_delete_non_existent_post_returns_zero(test_ctx: TestContext) {
    let deleted = test_ctx
        .post_repo
        .delete_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(99999))
        .unwrap();

    assert_eq!(
        deleted, 0,
        "Should return 0 when deleting non-existent post"
    );
}

#[rstest]
fn test_count_returns_zero_for_empty_site(test_ctx: TestContext) {
    let count = test_ctx
        .post_repo
        .count(&test_ctx.conn, &test_ctx.site)
        .unwrap();

    assert_eq!(count, 0, "Empty site should have count of 0");
}

#[rstest]
fn test_select_all_returns_empty_for_empty_site(test_ctx: TestContext) {
    let posts = test_ctx
        .post_repo
        .select_all(&test_ctx.conn, &test_ctx.site)
        .unwrap();

    assert_eq!(posts.len(), 0, "Empty site should return empty vector");
}
