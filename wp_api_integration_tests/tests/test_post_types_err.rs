use rstest::*;
use serial_test::parallel;
use wp_api::{post_types::PostType, WpRestErrorCode};
use wp_api_integration_tests::{api_client_as_subscriber, AssertWpError};

#[rstest]
#[tokio::test]
#[parallel]
async fn list_post_types_err_forbidden_context() {
    api_client_as_subscriber()
        .post_types()
        .list_with_edit_context()
        .await
        .assert_wp_error(WpRestErrorCode::CannotView);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn retrieve_post_types_err_forbidden_context(
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
    api_client_as_subscriber()
        .post_types()
        .retrieve_with_edit_context(&post_type)
        .await
        .assert_wp_error(WpRestErrorCode::ForbiddenContext);
}
