//! Transaction handling tests for PostRepository.
//!
//! These tests verify both successful transactions and failure cases,
//! ensuring proper rollback on errors without leaving partial writes or corrupted data.

use crate::{
    RowId,
    db_types::db_site::{DbSite, DbSiteType},
    test_fixtures::{TestContext, posts::PostBuilder, test_ctx},
};
use rstest::*;
use wp_api::posts::PostId;

#[rstest]
fn test_upsert_batch_handles_duplicate_ids_by_updating(mut test_ctx: TestContext) {
    // Pre-insert a post with ID 200
    let existing_post = PostBuilder::minimal()
        .with_id(200)
        .with_title("Original")
        .build();
    test_ctx
        .post_repo
        .upsert(&mut test_ctx.conn, &test_ctx.site, &existing_post)
        .unwrap();

    // Create batch where 2nd post has duplicate ID (200) with different title
    let post1 = PostBuilder::minimal().with_id(100).build();
    let post2 = PostBuilder::minimal()
        .with_id(200)
        .with_title("Updated")
        .build();
    let post3 = PostBuilder::minimal().with_id(300).build();

    let posts = vec![post1, post2, post3];

    // Batch upsert should succeed - duplicate is updated
    let entity_ids = test_ctx
        .post_repo
        .upsert_batch(&mut test_ctx.conn, &test_ctx.site, &posts)
        .unwrap();
    assert_eq!(entity_ids.len(), 3);

    // Verify all 3 posts exist (100, 200 updated, 300)
    let count = test_ctx
        .post_repo
        .count(&test_ctx.conn, &test_ctx.site)
        .unwrap();
    assert_eq!(count, 3, "Should have 3 posts total");

    // Verify post 100 was inserted
    assert!(
        test_ctx
            .post_repo
            .select_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(100))
            .expect("Failed to select post by post_id")
            .is_some()
    );

    // Verify post 200 was updated
    let post200 = test_ctx
        .post_repo
        .select_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(200))
        .expect("Failed to select post by post_id")
        .expect("Post should exist");
    assert_eq!(post200.data.post.title.rendered, "Updated");

    // Verify post 300 was inserted
    assert!(
        test_ctx
            .post_repo
            .select_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(300))
            .expect("Failed to select post by post_id")
            .is_some()
    );
}

#[rstest]
fn test_upsert_batch_fails_on_foreign_key_violation(mut test_ctx: TestContext) {
    // Site doesn't exist in database - intentionally invalid for testing error handling
    let invalid_site = DbSite {
        row_id: RowId(999),
        site_type: DbSiteType::SelfHosted,
        mapped_site_id: RowId(999),
    };

    let post1 = PostBuilder::minimal().build();
    let post2 = PostBuilder::minimal().build();

    let posts = vec![post1, post2];

    // Batch upsert to invalid site should fail on first post
    let result = test_ctx
        .post_repo
        .upsert_batch(&mut test_ctx.conn, &invalid_site, &posts);

    assert!(
        result.is_err(),
        "upsert_batch should fail with foreign key constraint"
    );

    // Verify no posts were inserted (fails fast on first error)
    let count = test_ctx
        .post_repo
        .count(&test_ctx.conn, &test_ctx.site)
        .unwrap();
    assert_eq!(count, 0, "No posts should exist after failure");
}

#[rstest]
fn test_upsert_maintains_consistency_on_success(mut test_ctx: TestContext) {
    let post_id_500 = PostId(500);

    // Create post with terms
    let post = PostBuilder::minimal()
        .with_post_id(post_id_500)
        .with_categories(vec![wp_api::terms::TermId(1), wp_api::terms::TermId(2)])
        .with_tags(vec![wp_api::terms::TermId(10)])
        .build();

    // Upsert should succeed
    let entity_id = test_ctx
        .post_repo
        .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
        .unwrap();

    // Verify post exists
    let retrieved = test_ctx
        .post_repo
        .select_by_post_id(&test_ctx.conn, &test_ctx.site, post_id_500)
        .expect("Failed to select post by post_id")
        .expect("Post should exist");
    assert_eq!(retrieved.data.post.id, post_id_500);
    assert_eq!(retrieved.data.row_id, entity_id.rowid);

    // Verify terms were synced correctly
    assert_eq!(
        retrieved.data.post.categories,
        Some(vec![wp_api::terms::TermId(1), wp_api::terms::TermId(2)])
    );
    assert_eq!(
        retrieved.data.post.tags,
        Some(vec![wp_api::terms::TermId(10)])
    );

    // Update the post with different terms
    let updated_post = PostBuilder::minimal()
        .with_post_id(post_id_500)
        .with_categories(vec![wp_api::terms::TermId(3)]) // Changed
        .with_tags(vec![]) // Cleared
        .build();

    // Upsert again
    test_ctx
        .post_repo
        .upsert(&mut test_ctx.conn, &test_ctx.site, &updated_post)
        .unwrap();

    // Verify terms were updated correctly
    let retrieved = test_ctx
        .post_repo
        .select_by_post_id(&test_ctx.conn, &test_ctx.site, post_id_500)
        .expect("Failed to select post by post_id")
        .expect("Post should exist");
    assert_eq!(
        retrieved.data.post.categories,
        Some(vec![wp_api::terms::TermId(3)])
    );
    assert_eq!(retrieved.data.post.tags, None);

    // Verify old terms are gone (no orphaned relationships)
    // The term_relationships table should only have the new category term
    let all_terms = test_ctx
        .term_repo
        .get_all_terms_for_object(&test_ctx.conn, &test_ctx.site, post_id_500.0)
        .unwrap();

    // Should only have one entry (Category with term 3)
    assert_eq!(
        all_terms.len(),
        1,
        "Should only have category taxonomy after update"
    );
    let categories = all_terms
        .get(&wp_api::taxonomies::TaxonomyType::Category)
        .unwrap();
    assert_eq!(categories.len(), 1);
    assert_eq!(categories[0], wp_api::terms::TermId(3));
}

#[rstest]
fn test_insert_batch_succeeds_with_valid_posts(mut test_ctx: TestContext) {
    // Create valid batch
    let post1 = PostBuilder::minimal().with_id(100).build();
    let post2 = PostBuilder::minimal().with_id(200).build();
    let post3 = PostBuilder::minimal().with_id(300).build();

    let posts = vec![post1, post2, post3];

    // Should succeed
    let rowids = test_ctx
        .post_repo
        .upsert_batch(&mut test_ctx.conn, &test_ctx.site, &posts)
        .unwrap();

    assert_eq!(rowids.len(), 3, "All 3 posts should be inserted");

    // Verify all posts exist
    let count = test_ctx
        .post_repo
        .count(&test_ctx.conn, &test_ctx.site)
        .unwrap();
    assert_eq!(count, 3);

    // Verify each post can be retrieved
    test_ctx
        .post_repo
        .select_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(100))
        .expect("Should not error")
        .expect("Post 100 should exist");
    test_ctx
        .post_repo
        .select_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(200))
        .expect("Should not error")
        .expect("Post 200 should exist");
    test_ctx
        .post_repo
        .select_by_post_id(&test_ctx.conn, &test_ctx.site, PostId(300))
        .expect("Should not error")
        .expect("Post 300 should exist");
}
