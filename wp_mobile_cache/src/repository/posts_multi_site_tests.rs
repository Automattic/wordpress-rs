//! Multi-site isolation tests for PostRepository.

use super::posts::PostRepository;
use crate::{
    DbSite,
    test_fixtures::posts::PostBuilder,
    test_helpers::{create_test_site, test_db, test_site},
};
use rstest::*;
use rusqlite::Connection;
use wp_api::posts::PostId;

#[rstest]
fn test_posts_in_site_1_invisible_to_site_2(mut test_db: Connection, test_site: DbSite) {
    let repo = PostRepository;
    let site2 = create_test_site(&test_db, 2);

    // Insert post in site 1
    let post = PostBuilder::new().with_id(PostId(100)).build();
    repo.upsert(&mut test_db, &test_site, &post).unwrap();

    // Site 2 should not see site 1's post
    let result = repo.select_by_post_id(&test_db, &site2, PostId(100));
    assert!(
        result.is_err(),
        "Site 2 should not be able to access Site 1's posts"
    );
}

#[rstest]
fn test_same_post_id_can_exist_in_different_sites(mut test_db: Connection, test_site: DbSite) {
    let repo = PostRepository;
    let site2 = create_test_site(&test_db, 2);

    // Create post with same ID in both sites
    let post_id = PostId(42);
    let post1 = PostBuilder::new()
        .with_id(post_id)
        .with_title("Site 1 Post")
        .build();
    let post2 = PostBuilder::new()
        .with_id(post_id)
        .with_title("Site 2 Post")
        .build();

    // Both inserts should succeed
    repo.upsert(&mut test_db, &test_site, &post1)
        .expect("Site 1 insert should succeed");
    repo.upsert(&mut test_db, &site2, &post2)
        .expect("Site 2 insert should succeed - same post ID in different site");

    // Verify each site sees its own post
    let retrieved1 = repo
        .select_by_post_id(&test_db, &test_site, post_id)
        .unwrap();
    let retrieved2 = repo.select_by_post_id(&test_db, &site2, post_id).unwrap();

    assert_eq!(retrieved1.post.title.rendered, "Site 1 Post");
    assert_eq!(retrieved2.post.title.rendered, "Site 2 Post");
}

#[rstest]
fn test_select_all_only_returns_posts_for_requested_site(
    mut test_db: Connection,
    test_site: DbSite,
) {
    let repo = PostRepository;
    let site2 = create_test_site(&test_db, 2);

    // Insert posts in site 1
    repo.upsert(&mut test_db, &test_site, &PostBuilder::new().build())
        .unwrap();
    repo.upsert(&mut test_db, &test_site, &PostBuilder::new().build())
        .unwrap();

    // Insert posts in site 2
    repo.upsert(&mut test_db, &site2, &PostBuilder::new().build())
        .unwrap();
    repo.upsert(&mut test_db, &site2, &PostBuilder::new().build())
        .unwrap();
    repo.upsert(&mut test_db, &site2, &PostBuilder::new().build())
        .unwrap();

    // Verify counts
    let site1_posts = repo.select_all(&test_db, &test_site).unwrap();
    let site2_posts = repo.select_all(&test_db, &site2).unwrap();

    assert_eq!(site1_posts.len(), 2, "Site 1 should have 2 posts");
    assert_eq!(site2_posts.len(), 3, "Site 2 should have 3 posts");
}

#[rstest]
fn test_count_only_counts_posts_for_requested_site(mut test_db: Connection, test_site: DbSite) {
    let repo = PostRepository;
    let site2 = create_test_site(&test_db, 2);

    // Insert posts in both sites
    repo.upsert(&mut test_db, &test_site, &PostBuilder::new().build())
        .unwrap();
    repo.upsert(&mut test_db, &test_site, &PostBuilder::new().build())
        .unwrap();

    repo.upsert(&mut test_db, &site2, &PostBuilder::new().build())
        .unwrap();

    assert_eq!(repo.count(&test_db, &test_site).unwrap(), 2);
    assert_eq!(repo.count(&test_db, &site2).unwrap(), 1);
}

#[rstest]
fn test_delete_by_post_id_only_deletes_from_specified_site(
    mut test_db: Connection,
    test_site: DbSite,
) {
    let repo = PostRepository;
    let site2 = create_test_site(&test_db, 2);

    let post_id = PostId(999);

    // Create post with same ID in both sites
    repo.upsert(
        &mut test_db,
        &test_site,
        &PostBuilder::new().with_id(post_id).build(),
    )
    .unwrap();
    repo.upsert(
        &mut test_db,
        &site2,
        &PostBuilder::new().with_id(post_id).build(),
    )
    .unwrap();

    // Delete from site 1
    let deleted = repo
        .delete_by_post_id(&test_db, &test_site, post_id)
        .unwrap();
    assert_eq!(deleted, 1);

    // Site 1 should no longer have the post
    assert!(
        repo.select_by_post_id(&test_db, &test_site, post_id)
            .is_err()
    );

    // Site 2 should still have its post
    assert!(repo.select_by_post_id(&test_db, &site2, post_id).is_ok());
}
