use serial_test::parallel;
use wp_api::{
    categories::{CategoryCreateParams, CategoryListParams, CategoryUpdateParams},
    WpErrorCode,
};
use wp_api_integration_tests::{
    api_client, api_client_as_subscriber, AssertWpError, CATEGORY_ID_59, CATEGORY_ID_INVALID,
    POST_ID_INVALID,
};

#[tokio::test]
#[parallel]
async fn create_err_cannot_create() {
    api_client_as_subscriber()
        .categories()
        .create(&CategoryCreateParams {
            name: "foo".to_string(),
            description: None,
            slug: None,
            parent: None,
        })
        .await
        .assert_wp_error(WpErrorCode::CannotCreate);
}

#[tokio::test]
#[parallel]
async fn create_err_term_invalid() {
    api_client()
        .categories()
        .create(&CategoryCreateParams {
            name: "foo".to_string(),
            description: None,
            slug: None,
            parent: Some(CATEGORY_ID_INVALID),
        })
        .await
        .assert_wp_error(WpErrorCode::TermInvalid);
}

#[tokio::test]
#[parallel]
async fn delete_err_cannot_delete() {
    api_client_as_subscriber()
        .categories()
        .delete(&CATEGORY_ID_59)
        .await
        .assert_wp_error(WpErrorCode::CannotDelete);
}

#[tokio::test]
#[parallel]
async fn list_err_forbidden_context() {
    api_client_as_subscriber()
        .categories()
        .list_with_edit_context(&CategoryListParams::default())
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
}

#[tokio::test]
#[parallel]
async fn list_err_post_invalid_id() {
    api_client()
        .categories()
        .list_with_edit_context(&CategoryListParams {
            post: Some(POST_ID_INVALID),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId);
}

#[tokio::test]
#[parallel]
async fn retrieve_err_term_invalid() {
    api_client()
        .categories()
        .retrieve_with_edit_context(&CATEGORY_ID_INVALID)
        .await
        .assert_wp_error(WpErrorCode::TermInvalid);
}

#[tokio::test]
#[parallel]
async fn update_err_cannot_update() {
    api_client_as_subscriber()
        .categories()
        .update(&CATEGORY_ID_59, &CategoryUpdateParams::default())
        .await
        .assert_wp_error(WpErrorCode::CannotUpdate);
}

#[tokio::test]
#[parallel]
async fn update_err_term_invalid() {
    api_client()
        .categories()
        .update(
            &CATEGORY_ID_59,
            &CategoryUpdateParams {
                parent: Some(CATEGORY_ID_INVALID),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::TermInvalid);
}
