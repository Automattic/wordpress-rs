//! Multi-site isolation tests for PostRepository.

use crate::test_fixtures::{TestContext, create_random_test_site, posts::PostBuilder, test_ctx};
use rstest::*;
use wp_api::posts::PostId;

#[rstest]
fn test_posts_in_site_1_invisible_to_site_2(mut test_ctx: TestContext) {
    let site2 = create_random_test_site(&test_ctx.conn);

    // Insert post in site 1
    let post = PostBuilder::minimal().with_id(100).build();
    test_ctx
        .post_repo
        .upsert(&mut test_ctx.conn, &test_ctx.site, &post)
        .unwrap();

    // Site 2 should not see site 1's post
    let result = test_ctx
        .post_repo
        .select_by_post_id(&test_ctx.conn, &site2, PostId(100))
        .unwrap();
    assert!(
        result.is_none(),
        "Site 2 should not be able to access Site 1's posts"
    );
}

#[rstest]
fn test_same_post_id_can_exist_in_different_sites(mut test_ctx: TestContext) {
    let site2 = create_random_test_site(&test_ctx.conn);

    // Create post with same ID in both sites
    let post_id = PostId(42);
    let post1 = PostBuilder::minimal()
        .with_id(42)
        .with_title("Site 1 Post")
        .build();
    let post2 = PostBuilder::minimal()
        .with_id(42)
        .with_title("Site 2 Post")
        .build();

    // Both inserts should succeed
    test_ctx
        .post_repo
        .upsert(&mut test_ctx.conn, &test_ctx.site, &post1)
        .expect("Site 1 insert should succeed");
    test_ctx
        .post_repo
        .upsert(&mut test_ctx.conn, &site2, &post2)
        .expect("Site 2 insert should succeed - same post ID in different site");

    // Verify each site sees its own post
    let retrieved1 = test_ctx
        .post_repo
        .select_by_post_id(&test_ctx.conn, &test_ctx.site, post_id)
        .expect("Failed to select post by post_id")
        .expect("Post should exist in site 1");
    let retrieved2 = test_ctx
        .post_repo
        .select_by_post_id(&test_ctx.conn, &site2, post_id)
        .expect("Failed to select post by post_id")
        .expect("Post should exist in site 2");

    assert_eq!(retrieved1.post.title.rendered, "Site 1 Post");
    assert_eq!(retrieved2.post.title.rendered, "Site 2 Post");
}

#[rstest]
fn test_select_all_only_returns_posts_for_requested_site(mut test_ctx: TestContext) {
    let site2 = create_random_test_site(&test_ctx.conn);

    // Insert posts in site 1
    test_ctx
        .post_repo
        .upsert(
            &mut test_ctx.conn,
            &test_ctx.site,
            &PostBuilder::minimal().build(),
        )
        .unwrap();
    test_ctx
        .post_repo
        .upsert(
            &mut test_ctx.conn,
            &test_ctx.site,
            &PostBuilder::minimal().build(),
        )
        .unwrap();

    // Insert posts in site 2
    test_ctx
        .post_repo
        .upsert(&mut test_ctx.conn, &site2, &PostBuilder::minimal().build())
        .unwrap();
    test_ctx
        .post_repo
        .upsert(&mut test_ctx.conn, &site2, &PostBuilder::minimal().build())
        .unwrap();
    test_ctx
        .post_repo
        .upsert(&mut test_ctx.conn, &site2, &PostBuilder::minimal().build())
        .unwrap();

    // Verify counts
    let site1_posts = test_ctx
        .post_repo
        .select_all(&test_ctx.conn, &test_ctx.site)
        .unwrap();
    let site2_posts = test_ctx
        .post_repo
        .select_all(&test_ctx.conn, &site2)
        .unwrap();

    assert_eq!(site1_posts.len(), 2, "Site 1 should have 2 posts");
    assert_eq!(site2_posts.len(), 3, "Site 2 should have 3 posts");
}

#[rstest]
fn test_count_only_counts_posts_for_requested_site(mut test_ctx: TestContext) {
    let site2 = create_random_test_site(&test_ctx.conn);

    // Insert posts in both sites
    test_ctx
        .post_repo
        .upsert(
            &mut test_ctx.conn,
            &test_ctx.site,
            &PostBuilder::minimal().build(),
        )
        .unwrap();
    test_ctx
        .post_repo
        .upsert(
            &mut test_ctx.conn,
            &test_ctx.site,
            &PostBuilder::minimal().build(),
        )
        .unwrap();

    test_ctx
        .post_repo
        .upsert(&mut test_ctx.conn, &site2, &PostBuilder::minimal().build())
        .unwrap();

    assert_eq!(
        test_ctx
            .post_repo
            .count(&test_ctx.conn, &test_ctx.site)
            .unwrap(),
        2
    );
    assert_eq!(test_ctx.post_repo.count(&test_ctx.conn, &site2).unwrap(), 1);
}

#[rstest]
fn test_delete_by_post_id_only_deletes_from_specified_site(mut test_ctx: TestContext) {
    let site2 = create_random_test_site(&test_ctx.conn);

    let post_id = PostId(999);

    // Create post with same ID in both sites
    test_ctx
        .post_repo
        .upsert(
            &mut test_ctx.conn,
            &test_ctx.site,
            &PostBuilder::minimal().with_id(999).build(),
        )
        .unwrap();
    test_ctx
        .post_repo
        .upsert(
            &mut test_ctx.conn,
            &site2,
            &PostBuilder::minimal().with_id(999).build(),
        )
        .unwrap();

    // Delete from site 1
    let deleted = test_ctx
        .post_repo
        .delete_by_post_id(&test_ctx.conn, &test_ctx.site, post_id)
        .unwrap();
    assert_eq!(deleted, 1);

    // Site 1 should no longer have the post
    assert!(
        test_ctx
            .post_repo
            .select_by_post_id(&test_ctx.conn, &test_ctx.site, post_id)
            .expect("Failed to select post by post_id")
            .is_none(),
        "Post should not exist in site 1 after deletion"
    );

    // Site 2 should still have its post
    assert!(
        test_ctx
            .post_repo
            .select_by_post_id(&test_ctx.conn, &site2, post_id)
            .expect("Failed to select post by post_id")
            .is_some(),
        "Post should still exist in site 2"
    );
}
