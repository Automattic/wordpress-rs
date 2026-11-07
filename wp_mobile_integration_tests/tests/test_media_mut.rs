use wp_api::media::MediaCreateParams;
use wp_api_integration_tests::MEDIA_TEST_FILE_PATH;
use wp_mobile::collection::MediaItemState;
use wp_mobile::filters::MediaListFilter;
use wp_mobile_integration_tests::*;

#[tokio::test]
#[serial]
async fn test_create_media_inserts_into_media_collection() {
    let ctx = create_test_context();

    let collection = ctx
        .service
        .media()
        .create_media_metadata_collection_with_edit_context(MediaListFilter::default(), 10);

    collection.refresh().await.expect("refresh should succeed");

    let items_before = collection
        .load_items()
        .await
        .expect("load_items should succeed");

    let created = ctx
        .service
        .media()
        .create_media(
            &MediaCreateParams {
                title: Some("Integration Test Media".to_string()),
                file_path: MEDIA_TEST_FILE_PATH.to_string(),
                ..Default::default()
            },
            None,
        )
        .await
        .expect("create_media should succeed");

    // Verify the newly created media was upserted into the cache.
    let cached = ctx
        .service
        .media()
        .read_media_by_ids_from_db(&[created.id.0])
        .expect("read_media_by_ids_from_db should succeed");
    assert_eq!(
        cached.len(),
        1,
        "created media should be written to the cache"
    );
    assert_eq!(cached[0].data.title.rendered, created.title.rendered);

    // Verify the live collection observed the new item without a second refresh.
    let items_after = collection
        .load_items()
        .await
        .expect("load_items should succeed");
    assert_eq!(
        items_after.len(),
        items_before.len() + 1,
        "creating media should add one item to the collection without a refresh"
    );

    // Verify insertion order: default filter sorts newest first.
    assert_eq!(
        items_after[0].id, created.id.0,
        "newly created media should appear at index 0 (newest first)"
    );

    // Verify cached entity fidelity and state: newly created items start in Stale state
    // until verified by the collection's internal sync engine.
    let inserted = items_after
        .iter()
        .find(|item| item.id == created.id.0)
        .expect("new media should appear in load_items without a refresh");
    if let MediaItemState::Stale { data } = &inserted.state {
        assert_eq!(data.data.title.rendered, created.title.rendered);
    } else {
        panic!(
            "newly created media should be Stale (data present in cache, unverified by sync), got {:?}",
            inserted.state
        );
    }

    RestoreServer::all().await;
}
