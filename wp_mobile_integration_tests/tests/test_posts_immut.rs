use wp_api::posts::{PostStatus, WpApiParamPostsOrderBy};
use wp_api::prelude::*;
use wp_api::request::endpoint::posts_endpoint::PostEndpointType;
use wp_api::terms::TermId;
use wp_mobile::collection::PostItemState;
use wp_mobile::filters::PostListFilter;
use wp_mobile_integration_tests::*;

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
        order: Some(WpApiParamOrder::Desc),
        orderby: Some(WpApiParamPostsOrderBy::Date),
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

    let list_info = collection
        .list_info()
        .expect("list_info should be available");
    let items = collection
        .load_items()
        .await
        .expect("load_items should succeed");
    assert_eq!(
        items.len(),
        list_info.total_items.unwrap() as usize,
        "items.len() should equal total_items from list_info"
    );

    for item in &items {
        assert!(
            matches!(item.state, PostItemState::Fresh { .. }),
            "all items should be Fresh after refresh: {:?}",
            item.state
        );
    }
}

// Books custom post type has `genre` and `book-author` custom taxonomies
// registered by the books-plugin. These appear as additional fields on book
// posts, keyed by the taxonomy's rest_base.

#[tokio::test]
#[parallel]
async fn test_books_collection_has_custom_taxonomy_terms() {
    let ctx = create_test_context();

    let collection = ctx
        .service
        .posts()
        .create_post_metadata_collection_with_edit_context(
            PostEndpointType::Custom("books".to_string()),
            PostListFilter::default(),
            25,
        );

    let result = collection.refresh().await;
    assert!(result.is_ok(), "refresh should succeed: {:?}", result.err());

    let items = collection
        .load_items()
        .await
        .expect("load_items should succeed");
    assert!(!items.is_empty(), "should have loaded some books");

    // Collect genre IDs from all books that have them
    let mut books_with_genres = 0;
    let mut all_genre_ids: Vec<TermId> = Vec::new();
    for item in &items {
        if let PostItemState::Fresh { data } = &item.state
            && let Some(additional) = &data.data.additional_fields
        {
            let genres = additional.term_ids_for_key("genre");
            if !genres.is_empty() {
                books_with_genres += 1;
                all_genre_ids.extend(genres);
            }
        }
    }
    assert!(
        books_with_genres > 0,
        "expected at least one book with genre terms in additional_fields"
    );
    assert!(
        !all_genre_ids.is_empty(),
        "expected at least one genre term ID"
    );
}

#[tokio::test]
#[parallel]
async fn test_books_collection_has_book_author_terms() {
    let ctx = create_test_context();

    let collection = ctx
        .service
        .posts()
        .create_post_metadata_collection_with_edit_context(
            PostEndpointType::Custom("books".to_string()),
            PostListFilter::default(),
            25,
        );

    let result = collection.refresh().await;
    assert!(result.is_ok(), "refresh should succeed: {:?}", result.err());

    let items = collection
        .load_items()
        .await
        .expect("load_items should succeed");

    // Verify book-author taxonomy terms are accessible via additional_fields
    let mut books_with_authors = 0;
    for item in &items {
        if let PostItemState::Fresh { data } = &item.state
            && let Some(additional) = &data.data.additional_fields
        {
            let authors = additional.term_ids_for_key("book-author");
            if !authors.is_empty() {
                books_with_authors += 1;
            }
        }
    }
    assert!(
        books_with_authors > 0,
        "expected at least one book with book-author terms in additional_fields"
    );
}
