use std::sync::{Arc, Mutex};
use wp_api::posts::PostStatus;
use wp_api::request::endpoint::posts_endpoint::PostEndpointType;
use wp_mobile::filters::PostListFilter;
use wp_mobile_cache::{DatabaseDelegate, UpdateHook};
use wp_mobile_integration_tests::*;

/// Reproduces an issue where refreshing a post collection sends way too many updates.
#[tokio::test]
#[parallel]
async fn test_minimal_updates() {
    let ctx = create_test_context();

    let collection = ctx
        .service
        .posts()
        .create_post_metadata_collection_with_edit_context(
            PostEndpointType::Posts,
            PostListFilter {
                status: vec![PostStatus::Draft],
                ..Default::default()
            },
            10,
        );

    let collector = Arc::new(UpdateCollector::new());
    ctx.cache.start_listening_for_updates(collector.clone());

    let result = collection.refresh().await;
    assert!(result.is_ok(), "refresh should succeed: {:?}", result.err());

    let all_updates = collector.collected_updates();
    let relevant_updates: Vec<_> = all_updates
        .iter()
        .filter(|hook| collection.is_relevant_update(hook))
        .collect();

    // TODO: What's the reasonable amount of updates for the `refresh` call?
    assert!(
        relevant_updates.len() < 5,
        "expected fewer than 5 relevant updates, got {}",
        relevant_updates.len()
    );
}

/// Reproduces an issue where refreshing one post collection trigger updates on an unrelated collection.
#[tokio::test]
#[parallel]
async fn test_update_should_be_isolated() {
    let ctx = create_test_context();

    let draft_collection = ctx
        .service
        .posts()
        .create_post_metadata_collection_with_edit_context(
            PostEndpointType::Posts,
            PostListFilter {
                status: vec![PostStatus::Draft],
                ..Default::default()
            },
            10,
        );

    let collector = Arc::new(UpdateCollector::new());
    ctx.cache.start_listening_for_updates(collector.clone());

    let publish_collection = ctx
        .service
        .posts()
        .create_post_metadata_collection_with_edit_context(
            PostEndpointType::Posts,
            PostListFilter {
                status: vec![PostStatus::Publish],
                ..Default::default()
            },
            10,
        );

    let result = publish_collection.refresh().await;
    assert!(result.is_ok(), "refresh should succeed: {:?}", result.err());

    let all_updates = collector.collected_updates();
    let draft_relevant_updates: Vec<_> = all_updates
        .iter()
        .filter(|hook| draft_collection.is_relevant_update(hook))
        .collect();

    assert_eq!(
        draft_relevant_updates.len(),
        0,
        "expected no updates relevant to draft collection when refreshing publish collection, got {}",
        draft_relevant_updates.len()
    );
}

struct UpdateCollector {
    updates: Mutex<Vec<UpdateHook>>,
}

impl UpdateCollector {
    fn new() -> Self {
        Self {
            updates: Mutex::new(Vec::new()),
        }
    }

    fn collected_updates(&self) -> Vec<UpdateHook> {
        self.updates.lock().unwrap().clone()
    }
}

impl DatabaseDelegate for UpdateCollector {
    fn did_update(&self, update_hook: UpdateHook) {
        self.updates.lock().unwrap().push(update_hook);
    }
}
