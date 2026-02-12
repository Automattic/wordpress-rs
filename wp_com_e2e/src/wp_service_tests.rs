use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::request::endpoint::posts_endpoint::PostEndpointType;
use wp_mobile::collection::PostItemState;
use wp_mobile::filters::PostListFilter;

use crate::context::TestContext;

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    vec![Trial::test("wp_service::refresh_posts", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let collection = ctx
                    .service
                    .posts()
                    .create_post_metadata_collection_with_edit_context(
                        PostEndpointType::Posts,
                        PostListFilter::default(),
                        10,
                    );

                let result = collection.refresh().await.map_err(|e| format!("{:?}", e))?;
                assert!(result.total_items > 0, "should have fetched some posts");

                let items = collection
                    .load_items()
                    .await
                    .map_err(|e| format!("{:?}", e))?;
                assert!(!items.is_empty(), "should have loaded some items");

                for item in &items {
                    assert!(
                        matches!(item.state, PostItemState::Fresh { .. }),
                        "all items should be Fresh after refresh"
                    );
                }

                Ok(())
            })
        }
    })]
}
