use rstest::*;
use serial_test::parallel;
use wp_api::post_types::{PostType, PostTypeSupports};

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
    let post_type = api_client()
        .post_types()
        .retrieve_with_edit_context(&post_type)
        .await
        .assert_response();
    // All post types in our current testing sites support `Title`, so we use this assertion
    // to verify that we are able to parse it properly.
    //
    // To be clear, if we can't parse it as expected, we'd get an error back and the previous
    // assertion would fail, but having some defensive validation may help guard against
    // future changes.
    //
    // It's entirely possible that we might have more test sites in the future and some of their
    // post types might not support a `Title` in which case it's perfectly fine to completely
    // remove this assertion.
    assert_eq!(
        post_type.supports.get(&PostTypeSupports::Title),
        Some(true).as_ref()
    );
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
