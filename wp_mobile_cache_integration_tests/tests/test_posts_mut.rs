use wp_api::posts::{PostCreateParams, PostStatus, WpApiParamPostsOrderBy};
use wp_api::prelude::*;
use wp_api::request::endpoint::posts_endpoint::PostEndpointType;
use wp_mobile::collection::PostItemState;
use wp_mobile::filters::PostListFilter;
use wp_mobile_cache_integration_tests::*;

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
