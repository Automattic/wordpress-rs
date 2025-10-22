//! Transaction rollback tests for PostRepository.
//!
//! These tests verify that transaction failures properly rollback database state
//! without leaving partial writes or corrupted data.

use super::{Repository, posts::PostRepository};
use crate::{
    DbSite,
    test_fixtures::posts::PostBuilder,
    test_helpers::{test_db, test_site},
};
use rstest::*;
use rusqlite::Connection;
use wp_api::posts::PostId;

#[rstest]
fn test_insert_batch_rolls_back_on_constraint_violation(
    mut test_db: Connection,
    test_site: DbSite,
) {
    let repo = PostRepository;

    // Pre-insert a post with ID 200 to create conflict
    let existing_post = PostBuilder::new().with_id(PostId(200)).build();
    repo.insert(&test_db, &existing_post, &test_site).unwrap();

    // Create batch where 2nd post has duplicate ID (200)
    let post1 = PostBuilder::new().with_id(PostId(100)).build();
    let post2 = PostBuilder::new().with_id(PostId(200)).build(); // Duplicate!
    let post3 = PostBuilder::new().with_id(PostId(300)).build();

    let posts = vec![post1, post2, post3];

    // Batch insert should fail due to duplicate ID
    let result = repo.insert_batch(&mut test_db, &posts, &test_site);

    assert!(
        result.is_err(),
        "insert_batch should fail when constraint violated"
    );

    // Verify rollback: Only the pre-existing post should remain (count = 1)
    // Posts 100 and 300 should NOT have been inserted
    let count = repo.count(&test_db, &test_site).unwrap();
    assert_eq!(
        count, 1,
        "Transaction should have rolled back - only pre-existing post should remain"
    );

    // Verify specifically that post 100 was NOT inserted (rollback worked)
    let result = repo.select_by_post_id(&test_db, &test_site, PostId(100));
    assert!(
        result.is_err(),
        "Post 100 should not exist - transaction rolled back"
    );

    // Verify post 300 was NOT inserted either
    let result = repo.select_by_post_id(&test_db, &test_site, PostId(300));
    assert!(
        result.is_err(),
        "Post 300 should not exist - transaction rolled back"
    );

    // Verify only original post exists
    let result = repo.select_by_post_id(&test_db, &test_site, PostId(200));
    assert!(result.is_ok(), "Original post 200 should still exist");
}

#[rstest]
fn test_insert_batch_rolls_back_on_foreign_key_violation(
    mut test_db: Connection,
    test_site: DbSite,
) {
    let repo = PostRepository;
    let invalid_site = DbSite {
        row_id: crate::RowId(999),
    }; // Non-existent site

    // Create batch where 2nd post references invalid site
    // Note: We can't mix sites in a single batch call, but we can verify
    // that a batch to an invalid site doesn't leave partial state

    let post1 = PostBuilder::new().with_id(PostId(100)).build();
    let post2 = PostBuilder::new().with_id(PostId(200)).build();

    let posts = vec![post1, post2];

    // Batch insert to invalid site should fail
    let result = repo.insert_batch(&mut test_db, &posts, &invalid_site);

    assert!(
        result.is_err(),
        "insert_batch should fail with foreign key constraint"
    );

    // Verify rollback: No posts should have been inserted in the valid site either
    let count = repo.count(&test_db, &test_site).unwrap();
    assert_eq!(count, 0, "No posts should exist after rollback");

    // Verify specifically that neither post was inserted
    let result = repo.select_by_post_id(&test_db, &test_site, PostId(100));
    assert!(result.is_err(), "Post 100 should not exist");

    let result = repo.select_by_post_id(&test_db, &test_site, PostId(200));
    assert!(result.is_err(), "Post 200 should not exist");
}

#[rstest]
fn test_upsert_with_terms_maintains_consistency_on_success(
    mut test_db: Connection,
    test_site: DbSite,
) {
    let repo = PostRepository;

    // Create post with terms
    let post = PostBuilder::new()
        .with_id(PostId(500))
        .with_categories(vec![wp_api::terms::TermId(1), wp_api::terms::TermId(2)])
        .with_tags(vec![wp_api::terms::TermId(10)])
        .build();

    // Upsert should succeed
    let rowid = repo
        .upsert_with_terms(&mut test_db, &test_site, &post)
        .unwrap();

    // Verify post exists
    let retrieved = repo
        .select_by_post_id(&test_db, &test_site, PostId(500))
        .unwrap();
    assert_eq!(retrieved.post.id, PostId(500));
    assert_eq!(retrieved.row_id, rowid);

    // Verify terms were synced correctly
    assert_eq!(
        retrieved.post.categories,
        Some(vec![wp_api::terms::TermId(1), wp_api::terms::TermId(2)])
    );
    assert_eq!(retrieved.post.tags, Some(vec![wp_api::terms::TermId(10)]));

    // Update the post with different terms
    let updated_post = PostBuilder::new()
        .with_id(PostId(500))
        .with_categories(vec![wp_api::terms::TermId(3)]) // Changed
        .with_tags(vec![]) // Cleared
        .build();

    // Upsert again
    repo.upsert_with_terms(&mut test_db, &test_site, &updated_post)
        .unwrap();

    // Verify terms were updated correctly
    let retrieved = repo
        .select_by_post_id(&test_db, &test_site, PostId(500))
        .unwrap();
    assert_eq!(
        retrieved.post.categories,
        Some(vec![wp_api::terms::TermId(3)])
    );
    assert_eq!(retrieved.post.tags, Some(vec![]));

    // Verify old terms are gone (no orphaned relationships)
    // The term_relationships table should only have the new category term
    let term_repo = super::term_relationships::TermRelationshipRepository;
    let all_terms = term_repo
        .get_all_terms_for_object(&test_db, &test_site, rowid)
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
fn test_insert_batch_succeeds_with_valid_posts(mut test_db: Connection, test_site: DbSite) {
    let repo = PostRepository;

    // Create valid batch
    let post1 = PostBuilder::new().with_id(PostId(100)).build();
    let post2 = PostBuilder::new().with_id(PostId(200)).build();
    let post3 = PostBuilder::new().with_id(PostId(300)).build();

    let posts = vec![post1, post2, post3];

    // Should succeed
    let rowids = repo.insert_batch(&mut test_db, &posts, &test_site).unwrap();

    assert_eq!(rowids.len(), 3, "All 3 posts should be inserted");

    // Verify all posts exist
    let count = repo.count(&test_db, &test_site).unwrap();
    assert_eq!(count, 3);

    // Verify each post can be retrieved
    repo.select_by_post_id(&test_db, &test_site, PostId(100))
        .expect("Post 100 should exist");
    repo.select_by_post_id(&test_db, &test_site, PostId(200))
        .expect("Post 200 should exist");
    repo.select_by_post_id(&test_db, &test_site, PostId(300))
        .expect("Post 300 should exist");
}
