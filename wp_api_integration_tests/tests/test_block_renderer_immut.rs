use wp_api::block_renderer::{BlockName, BlockRendererPostParams};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn render_dynamic_block() {
    let response = api_client()
        .block_renderer()
        .render(
            &BlockName("core/latest-posts".to_string()),
            &BlockRendererPostParams::default(),
        )
        .await
        .assert_response();
    assert!(
        !response.data.rendered.is_empty(),
        "Rendered block HTML should not be empty"
    );
}

#[tokio::test]
#[parallel]
async fn render_dynamic_block_with_attributes() {
    let params = BlockRendererPostParams {
        attributes: Some(
            [("postsToShow".to_string(), wp_api::JsonValue::Int(1))]
                .into_iter()
                .collect(),
        ),
        ..Default::default()
    };
    let response = api_client()
        .block_renderer()
        .render(&BlockName("core/latest-posts".to_string()), &params)
        .await
        .assert_response();
    assert!(
        !response.data.rendered.is_empty(),
        "Rendered block HTML should not be empty"
    );
}

#[tokio::test]
#[parallel]
async fn render_dynamic_block_with_post_id() {
    let params = BlockRendererPostParams {
        post_id: Some(FIRST_POST_ID),
        ..Default::default()
    };
    let response = api_client()
        .block_renderer()
        .render(&BlockName("core/latest-posts".to_string()), &params)
        .await
        .assert_response();
    assert!(
        !response.data.rendered.is_empty(),
        "Rendered block HTML should not be empty"
    );
}
