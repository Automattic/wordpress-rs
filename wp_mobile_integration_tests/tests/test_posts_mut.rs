use wp_api::posts::{
    PostCreateParams, PostId, PostStatus, PostUpdateParams, WpApiParamPostsOrderBy,
};
use wp_api::prelude::*;
use wp_api::request::endpoint::posts_endpoint::PostEndpointType;
use wp_mobile::collection::PostItemState;
use wp_mobile::filters::PostListFilter;
use wp_mobile_integration_tests::*;

#[tokio::test]
#[serial]
async fn test_load_next_page_with_duplicate_items_all_fresh() {
    // This test reproduces an issue where the loaded items have a "Missing"
    // state when the second page contains a post that was already fetched in
    // the first page.
    //
    // To simulate the scenario, we use a `PostMetadataCollection` instance to
    // load future posts, ordered by date desc (furthest date first). The page
    // size is set to 5.
    //
    // The local test site already has one future post, so we create 5 more
    // future posts with dates that will ensure we have two pages of results.
    // Here is the detailed setup:
    //
    // 1. Create 5 future posts with dates 2050-01-01 to 2050-01-05.
    // 2. Use `PostMetadataCollection.refresh` to load the first page. All 5
    //    posts that we just created should be loaded with "Fresh" state.
    // 3. Create another future post with date 2050-01-04, which is in the
    //    middle of the 5 posts created above. This will push one of the 5
    //    posts to the second page when we load the next page. That means the
    //    second page will contain one duplicate post that was already loaded
    //    in the first page.
    // 4. Call `load_next_page` to load the second page, and verify that all
    //    loaded items are still in "Fresh" state.
    //
    // Expected result: All loaded items should be in "Fresh" state, including
    // the duplicate post, since all of them have been fetched.

    let ctx = create_test_context();

    // Step 1: Create 5 future posts
    let base_date = "2050-01-";
    for i in 1..=5 {
        let params = PostCreateParams {
            title: Some(format!("Test Post {}", i)),
            date: Some(format!("{}{:02}T12:00:00", base_date, i)),
            status: Some(PostStatus::Future),
            ..Default::default()
        };
        ctx.api
            .posts()
            .create(&PostEndpointType::Posts, &params)
            .await
            .expect("Failed to create post");
    }

    // Step 2: Create collection and refresh to load first page
    let filter = PostListFilter {
        order: Some(WpApiParamOrder::Desc),
        orderby: Some(WpApiParamPostsOrderBy::Date),
        status: vec![PostStatus::Future],
        ..Default::default()
    };
    let collection = ctx
        .service
        .posts()
        .create_post_metadata_collection_with_edit_context(PostEndpointType::Posts, filter, 5);

    let refresh_result = collection.refresh().await;
    assert!(
        refresh_result.is_ok(),
        "refresh should succeed: {:?}",
        refresh_result.err()
    );
    assert_eq!(collection.current_page(), Some(1));

    // Step 3: Create another future post
    let new_post_date = format!("{}04T06:00:00", base_date);
    let new_post_params = PostCreateParams {
        title: Some("New Post That Pushes One to Page 2".to_string()),
        date: Some(new_post_date),
        status: Some(PostStatus::Future),
        ..Default::default()
    };
    ctx.api
        .posts()
        .create(&PostEndpointType::Posts, &new_post_params)
        .await
        .expect("Failed to create new post");

    // Step 4: Load next page
    collection
        .load_next_page()
        .await
        .expect("load_next_page should succeed");
    assert_eq!(collection.current_page(), Some(2));

    let items = collection
        .load_items()
        .await
        .expect("load_items should succeed");
    assert!(!items.is_empty());

    // Expectation: All items should be Fresh
    for item in &items {
        assert!(
            matches!(item.state, PostItemState::Fresh { .. }),
            "item {} should be Fresh after load_next_page, got {:?}",
            item.id,
            item.state
        );
    }

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn test_publish_draft_in_first_page_updates_collection() {
    // The test site has 4 draft posts. We use per_page=2 so page 1 has 2 drafts.
    //
    // 1. Create a draft collection with per_page=2 and call `refresh()` to
    //    load page 1.
    // 2. Record the first item's ID and the current `total_items`.
    // 3. Publish that draft via the REST API.
    //
    // Expected result: The published post no longer appears in `load_items()`.

    let ctx = create_test_context();

    let filter = PostListFilter {
        order: Some(WpApiParamOrder::Desc),
        orderby: Some(WpApiParamPostsOrderBy::Date),
        status: vec![PostStatus::Draft],
        ..Default::default()
    };
    let collection = ctx
        .service
        .posts()
        .create_post_metadata_collection_with_edit_context(PostEndpointType::Posts, filter, 2);

    collection.refresh().await.expect("refresh should succeed");
    assert_eq!(collection.current_page(), Some(1));

    let items = collection
        .load_items()
        .await
        .expect("load_items should succeed");
    assert!(!items.is_empty());

    let published_id = items[0].id;

    // Publish the first draft post
    ctx.service
        .posts()
        .update_post(
            &PostEndpointType::Posts,
            &PostId(published_id),
            &PostUpdateParams {
                status: Some(PostStatus::Publish),
                ..Default::default()
            },
        )
        .await
        .expect("refresh_post should succeed");

    let updated_items = collection
        .load_items()
        .await
        .expect("load_items should succeed");
    assert!(
        !updated_items.iter().any(|item| item.id == published_id),
        "published post {} should not appear in draft collection",
        published_id
    );

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn test_publish_draft_in_second_page_updates_collection() {
    // The test site has 4 draft posts. We use per_page=2 so page 1 has
    // 2 drafts and page 2 has 2 drafts.
    //
    // 1. Create a draft collection with per_page=2 and call `refresh()` to
    //    load page 1, then `load_next_page()` to load page 2.
    // 2. Record the last item's ID (from page 2) and the current `total_items`.
    // 3. Publish that draft via the REST API.
    //
    // Expected result: the published post no longer appears in `load_items()`.

    let ctx = create_test_context();

    let filter = PostListFilter {
        order: Some(WpApiParamOrder::Desc),
        orderby: Some(WpApiParamPostsOrderBy::Date),
        status: vec![PostStatus::Draft],
        ..Default::default()
    };
    let collection = ctx
        .service
        .posts()
        .create_post_metadata_collection_with_edit_context(PostEndpointType::Posts, filter, 2);

    collection.refresh().await.expect("refresh should succeed");
    assert_eq!(collection.current_page(), Some(1));

    collection
        .load_next_page()
        .await
        .expect("load_next_page should succeed");
    assert_eq!(collection.current_page(), Some(2));

    let items = collection
        .load_items()
        .await
        .expect("load_items should succeed");
    assert!(items.len() > 2, "should have items from both pages");

    // Pick the last item (from page 2)
    let published_id = items.last().expect("items should not be empty").id;

    // Publish the draft post from page 2
    ctx.service
        .posts()
        .update_post(
            &PostEndpointType::Posts,
            &PostId(published_id),
            &PostUpdateParams {
                status: Some(PostStatus::Publish),
                ..Default::default()
            },
        )
        .await
        .expect("update should succeed");

    let updated_items = collection
        .load_items()
        .await
        .expect("load_items should succeed");
    assert!(
        !updated_items.iter().any(|item| item.id == published_id),
        "published post {} should not appear in draft collection",
        published_id
    );

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn test_create_draft_inserts_into_draft_collection() {
    // The test site has existing draft posts. We use per_page=10 so
    // page 1 has all of them.
    //
    // 1. Create a draft collection and call `refresh()` to load page 1.
    // 2. Record the current item count.
    // 3. Create a new draft via `PostService::create_post`, which caches
    //    the post and notifies collections.
    //
    // Expected result: the new draft appears in `load_items()`.

    let ctx = create_test_context();

    let filter = PostListFilter {
        order: Some(WpApiParamOrder::Desc),
        orderby: Some(WpApiParamPostsOrderBy::Date),
        status: vec![PostStatus::Draft],
        ..Default::default()
    };
    let collection = ctx
        .service
        .posts()
        .create_post_metadata_collection_with_edit_context(PostEndpointType::Posts, filter, 10);

    collection.refresh().await.expect("refresh should succeed");

    let items_before = collection
        .load_items()
        .await
        .expect("load_items should succeed");

    let created = ctx
        .service
        .posts()
        .create_post(
            &PostEndpointType::Posts,
            &PostCreateParams {
                title: Some("Integration Test Draft".to_string()),
                status: Some(PostStatus::Draft),
                ..Default::default()
            },
        )
        .await
        .expect("create_post should succeed");

    let items_after = collection
        .load_items()
        .await
        .expect("load_items should succeed");
    assert_eq!(
        items_after.len(),
        items_before.len() + 1,
        "creating a draft should add one item to the collection"
    );
    assert!(
        items_after.iter().any(|item| item.id == created.id.0),
        "new draft {} should appear in load_items",
        created.id.0
    );

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn test_trash_draft_removes_from_draft_collection() {
    // 1. Create a draft collection with per_page=10 and call `refresh()`
    //    to load all drafts.
    // 2. Record one draft's ID and the current item count.
    // 3. Trash that draft via `PostService::trash_post`.
    //
    // Expected result: the trashed post no longer appears in `load_items()`
    // because its status changed from Draft to Trash.

    let ctx = create_test_context();

    let filter = PostListFilter {
        order: Some(WpApiParamOrder::Desc),
        orderby: Some(WpApiParamPostsOrderBy::Date),
        status: vec![PostStatus::Draft],
        ..Default::default()
    };
    let collection = ctx
        .service
        .posts()
        .create_post_metadata_collection_with_edit_context(PostEndpointType::Posts, filter, 10);

    collection.refresh().await.expect("refresh should succeed");

    let items_before = collection
        .load_items()
        .await
        .expect("load_items should succeed");
    assert!(!items_before.is_empty(), "should have at least one draft");

    let trashed_id = items_before[0].id;

    ctx.service
        .posts()
        .trash_post(&PostEndpointType::Posts, &PostId(trashed_id))
        .await
        .expect("trash_post should succeed");

    let items_after = collection
        .load_items()
        .await
        .expect("load_items should succeed");
    assert!(
        !items_after.iter().any(|item| item.id == trashed_id),
        "trashed post {} should not appear in draft collection",
        trashed_id
    );
    assert_eq!(
        items_after.len(),
        items_before.len() - 1,
        "item count should decrease by one after trashing"
    );

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn test_delete_permanently_removes_from_collection() {
    // 1. Create a draft, then trash it so it can be permanently deleted.
    // 2. Create a trash collection with per_page=10 and call `refresh()`.
    // 3. Permanently delete the trashed post via
    //    `PostService::delete_post_permanently`.
    //
    // Expected result: the deleted post no longer appears in `load_items()`.

    let ctx = create_test_context();

    // Create a draft and immediately trash it so we have a trashed post
    let created = ctx
        .service
        .posts()
        .create_post(
            &PostEndpointType::Posts,
            &PostCreateParams {
                title: Some("Post To Delete Permanently".to_string()),
                status: Some(PostStatus::Draft),
                ..Default::default()
            },
        )
        .await
        .expect("create_post should succeed");

    let trashed = ctx
        .service
        .posts()
        .trash_post(&PostEndpointType::Posts, &created.id)
        .await
        .expect("trash_post should succeed");
    assert_eq!(trashed.status, PostStatus::Trash);

    // Build a trash collection
    let filter = PostListFilter {
        order: Some(WpApiParamOrder::Desc),
        orderby: Some(WpApiParamPostsOrderBy::Date),
        status: vec![PostStatus::Trash],
        ..Default::default()
    };
    let collection = ctx
        .service
        .posts()
        .create_post_metadata_collection_with_edit_context(PostEndpointType::Posts, filter, 10);

    collection.refresh().await.expect("refresh should succeed");

    let items_before = collection
        .load_items()
        .await
        .expect("load_items should succeed");
    assert!(
        items_before.iter().any(|item| item.id == created.id.0),
        "trashed post {} should appear in trash collection before delete",
        created.id.0
    );

    ctx.service
        .posts()
        .delete_post_permanently(&PostEndpointType::Posts, &created.id)
        .await
        .expect("delete_post_permanently should succeed");

    let items_after = collection
        .load_items()
        .await
        .expect("load_items should succeed");
    assert!(
        !items_after.iter().any(|item| item.id == created.id.0),
        "permanently deleted post {} should not appear in trash collection",
        created.id.0
    );
    assert_eq!(
        items_after.len(),
        items_before.len() - 1,
        "item count should decrease by one after permanent delete"
    );

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn test_load_posts_by_ids_includes_trashed_post() {
    // 1. Create a draft post, then trash it.
    // 2. Call `load_posts_by_ids` with the trashed post's ID.
    //
    // Expected result: the trashed post is returned successfully and
    // its status in the cache is `PostStatus::Trash`.

    let ctx = create_test_context();

    let created = ctx
        .service
        .posts()
        .create_post(
            &PostEndpointType::Posts,
            &PostCreateParams {
                title: Some("Post To Trash Then Load By ID".to_string()),
                status: Some(PostStatus::Draft),
                ..Default::default()
            },
        )
        .await
        .expect("create_post should succeed");

    let trashed = ctx
        .service
        .posts()
        .trash_post(&PostEndpointType::Posts, &created.id)
        .await
        .expect("trash_post should succeed");
    assert_eq!(trashed.status, PostStatus::Trash);

    let result = ctx
        .service
        .posts()
        .load_posts_by_ids(&PostEndpointType::Posts, vec![created.id])
        .await
        .expect("load_posts_by_ids should succeed");

    assert_eq!(result.entity_ids.len(), 1, "should load 1 post");
    assert_eq!(result.failed_count, 0, "no posts should fail to load");

    let cached_posts = ctx
        .service
        .posts()
        .read_posts_by_ids_from_db(&[created.id.0])
        .expect("read_posts_by_ids_from_db should succeed");
    assert_eq!(cached_posts.len(), 1, "should have 1 cached post");
    assert_eq!(
        cached_posts[0].data.status,
        PostStatus::Trash,
        "cached post should have Trash status"
    );

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn test_load_posts_by_ids_includes_mixed_status_posts() {
    // 1. Create a draft post.
    // 2. Create another post and trash it.
    // 3. Call `load_posts_by_ids` with both IDs.
    //
    // Expected result: both posts are returned—one with Draft status and
    // one with Trash status.

    let ctx = create_test_context();

    let draft = ctx
        .service
        .posts()
        .create_post(
            &PostEndpointType::Posts,
            &PostCreateParams {
                title: Some("Draft Post For Mixed Status Test".to_string()),
                status: Some(PostStatus::Draft),
                ..Default::default()
            },
        )
        .await
        .expect("create draft should succeed");

    let to_trash = ctx
        .service
        .posts()
        .create_post(
            &PostEndpointType::Posts,
            &PostCreateParams {
                title: Some("Post To Trash For Mixed Status Test".to_string()),
                status: Some(PostStatus::Draft),
                ..Default::default()
            },
        )
        .await
        .expect("create post to trash should succeed");

    ctx.service
        .posts()
        .trash_post(&PostEndpointType::Posts, &to_trash.id)
        .await
        .expect("trash_post should succeed");

    let result = ctx
        .service
        .posts()
        .load_posts_by_ids(&PostEndpointType::Posts, vec![draft.id, to_trash.id])
        .await
        .expect("load_posts_by_ids should succeed");

    assert_eq!(result.entity_ids.len(), 2, "should load 2 posts");
    assert_eq!(result.failed_count, 0, "no posts should fail to load");

    let cached_posts = ctx
        .service
        .posts()
        .read_posts_by_ids_from_db(&[draft.id.0, to_trash.id.0])
        .expect("read_posts_by_ids_from_db should succeed");
    assert_eq!(cached_posts.len(), 2, "should have 2 cached posts");

    let draft_post = cached_posts
        .iter()
        .find(|p| p.data.id == draft.id)
        .expect("draft post should be in cache");
    let trashed_post = cached_posts
        .iter()
        .find(|p| p.data.id == to_trash.id)
        .expect("trashed post should be in cache");

    assert_eq!(
        draft_post.data.status,
        PostStatus::Draft,
        "draft post should have Draft status"
    );
    assert_eq!(
        trashed_post.data.status,
        PostStatus::Trash,
        "trashed post should have Trash status"
    );

    RestoreServer::db().await;
}
