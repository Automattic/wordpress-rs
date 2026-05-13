use wp_mobile::collection::MediaItemState;
use wp_mobile::filters::MediaListFilter;
use wp_mobile_integration_tests::*;

#[tokio::test]
#[parallel]
async fn test_refresh_loads_media_items() {
    let ctx = create_test_context();

    let collection = ctx
        .service
        .media()
        .create_media_metadata_collection_with_edit_context(MediaListFilter::default(), 5);

    let result = collection
        .refresh()
        .await
        .expect("refresh should succeed against the test server");

    assert!(
        result.total_items > 0,
        "expected the test instance to have at least one media item; refresh returned {} total",
        result.total_items
    );

    let items = collection
        .load_items()
        .await
        .expect("load_items should succeed after refresh");

    assert_eq!(
        items.len(),
        (result.total_items as usize).min(5),
        "loaded items should match the first page size or total, whichever is smaller"
    );

    for item in &items {
        assert!(
            matches!(item.state, MediaItemState::Fresh { .. }),
            "all items should be Fresh after refresh, got {:?}",
            item.state
        );
    }
}
