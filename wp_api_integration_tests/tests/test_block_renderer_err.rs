use wp_api::block_renderer::{BlockName, BlockRendererPostParams};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn render_err_block_invalid() {
    api_client()
        .block_renderer()
        .render(
            &BlockName("nonexistent/nonexistent".to_string()),
            &BlockRendererPostParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::BlockInvalid)
}

#[tokio::test]
#[parallel]
async fn render_err_non_dynamic_block() {
    // `core/paragraph` is a registered but non-dynamic block
    api_client()
        .block_renderer()
        .render(
            &BlockName("core/paragraph".to_string()),
            &BlockRendererPostParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::BlockInvalid)
}

#[tokio::test]
#[parallel]
async fn render_err_cannot_read_as_subscriber() {
    api_client_as_subscriber()
        .block_renderer()
        .render(
            &BlockName("core/latest-posts".to_string()),
            &BlockRendererPostParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::BlockCannotRead)
}
