use rstest::*;
use serial_test::parallel;
use wp_api::post_types::PostType;

use crate::integration_test_common::{api_client, AssertResponse};

pub mod integration_test_common;

#[rstest]
#[tokio::test]
#[parallel]
async fn list_post_types_with_edit_context() {
    api_client()
        .post_types()
        .list_with_edit_context()
        .await
        .assert_response();
}

#[rstest]
#[tokio::test]
#[parallel]
async fn list_post_types_with_embed_context() {
    api_client()
        .post_types()
        .list_with_embed_context()
        .await
        .assert_response();
}

#[rstest]
#[tokio::test]
#[parallel]
async fn list_post_types_with_view_context() {
    api_client()
        .post_types()
        .list_with_view_context()
        .await
        .assert_response();
}

#[rstest]
#[tokio::test]
#[parallel]
async fn retrieve_post_types_with_edit_context(
    #[values(
        PostType::Post,
        PostType::Page,
        PostType::Attachment,
        PostType::NavMenuItem,
        PostType::WpBlock,
        PostType::WpTemplate,
        PostType::WpTemplatePart,
        PostType::WpNavigation,
        PostType::WpFontFamily,
        PostType::WpFontFace
    )]
    post_type: PostType,
) {
    api_client()
        .post_types()
        .retrieve_with_edit_context(&post_type)
        .await
        .assert_response();
}

#[rstest]
#[tokio::test]
#[parallel]
async fn retrieve_post_types_with_embed_context(
    #[values(
        PostType::Post,
        PostType::Page,
        PostType::Attachment,
        PostType::NavMenuItem,
        PostType::WpBlock,
        PostType::WpTemplate,
        PostType::WpTemplatePart,
        PostType::WpNavigation,
        PostType::WpFontFamily,
        PostType::WpFontFace
    )]
    post_type: PostType,
) {
    api_client()
        .post_types()
        .retrieve_with_embed_context(&post_type)
        .await
        .assert_response();
}

#[rstest]
#[tokio::test]
#[parallel]
async fn retrieve_post_types_with_view_context(
    #[values(
        PostType::Post,
        PostType::Page,
        PostType::Attachment,
        PostType::NavMenuItem,
        PostType::WpBlock,
        PostType::WpTemplate,
        PostType::WpTemplatePart,
        PostType::WpNavigation,
        PostType::WpFontFamily,
        PostType::WpFontFace
    )]
    post_type: PostType,
) {
    api_client()
        .post_types()
        .retrieve_with_view_context(&post_type)
        .await
        .assert_response();
}
