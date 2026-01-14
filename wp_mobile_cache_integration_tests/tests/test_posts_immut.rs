use wp_api::posts::PostStatus;
use wp_api::request::endpoint::posts_endpoint::PostEndpointType;
use wp_mobile::collection::PostItemState;
use wp_mobile::filters::PostListFilter;
use wp_mobile_cache_integration_tests::*;

#[tokio::test]
#[parallel]
async fn test_refresh() {
    let ctx = create_test_context();

    let collection = ctx
        .service
        .posts()
        .create_post_metadata_collection_with_edit_context(
            PostEndpointType::Posts,
            PostListFilter::default(),
            10,
        );

    let result = collection.refresh().await;
    assert!(result.is_ok(), "refresh should succeed: {:?}", result.err());

    let sync_result = result.unwrap();
    assert!(
        sync_result.total_items > 0,
        "should have fetched some posts"
    );

    let items = collection
        .load_items()
        .await
        .expect("load_items should succeed");
    assert!(!items.is_empty(), "should have loaded some items");

    for item in &items {
        assert!(
            matches!(item.state, PostItemState::Fresh { .. }),
            "all items should be Fresh after refresh"
        );
    }
}

#[tokio::test]
#[parallel]
async fn test_fetch_all_posts_with_any_status() {
    let ctx = create_test_context();

    let filter = PostListFilter {
        status: vec![PostStatus::Custom("any".to_string())],
        ..Default::default()
    };

    let collection = ctx
        .service
        .posts()
        .create_post_metadata_collection_with_edit_context(PostEndpointType::Posts, filter, 40);

    let first_page_result = collection.refresh().await;
    assert!(
        first_page_result.is_ok(),
        "refresh should succeed: {:?}",
        first_page_result.err()
    );

    let second_page_result = collection.load_next_page().await;
    assert!(
        second_page_result.is_ok(),
        "load_next_page should succeed: {:?}",
        second_page_result.err()
    );

    assert_eq!(
        collection.has_more_pages(),
        Some(false),
        "should have no more pages after loading two pages of 40 items"
    );

    let items = collection
        .load_items()
        .await
        .expect("load_items should succeed");
    assert!(!items.is_empty(), "should have loaded some items");

    for item in &items {
        assert!(
            matches!(item.state, PostItemState::Fresh { .. }),
            "all items should be Fresh after refresh"
        );
    }
}

#[tokio::test]
#[parallel]
async fn test_fetch_published_then_all_posts() {
    let ctx = create_test_context();
    let post_service = ctx.service.posts();

    let published_collection = post_service.create_post_metadata_collection_with_edit_context(
        PostEndpointType::Posts,
        PostListFilter::default(),
        40,
    );

    let published_result = published_collection.refresh().await;
    assert!(
        published_result.is_ok(),
        "refresh published posts should succeed: {:?}",
        published_result.err()
    );

    let filter = PostListFilter {
        status: vec![PostStatus::Custom("any".to_string())],
        ..Default::default()
    };

    let all_posts_collection = post_service.create_post_metadata_collection_with_edit_context(
        PostEndpointType::Posts,
        filter,
        40,
    );

    let all_posts_result = all_posts_collection.refresh().await;
    assert!(
        all_posts_result.is_ok(),
        "refresh all posts should succeed: {:?}",
        all_posts_result.err()
    );

    let items = all_posts_collection
        .load_items()
        .await
        .expect("load_items should succeed");
    assert!(!items.is_empty(), "should have loaded some items");

    for item in &items {
        assert!(
            matches!(item.state, PostItemState::Fresh { .. }),
            "all items should be Fresh after refresh"
        );
    }
}
