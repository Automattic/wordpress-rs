use wp_api::{
    request::endpoint::terms_endpoint::TermEndpointType,
    terms::{TermCreateParams, TermListParams, TermUpdateParams},
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn create_err_cannot_create() {
    api_client_as_subscriber()
        .terms()
        .create(
            &TermEndpointType::Categories,
            &TermCreateParams {
                name: "foo".to_string(),
                description: None,
                slug: None,
                parent: None,
            },
        )
        .await
        .assert_wp_error(WpErrorCode::CannotCreate);
}

#[tokio::test]
#[parallel]
async fn create_err_term_invalid() {
    api_client()
        .terms()
        .create(
            &TermEndpointType::Categories,
            &TermCreateParams {
                name: "foo".to_string(),
                description: None,
                slug: None,
                parent: Some(TERM_ID_INVALID),
            },
        )
        .await
        .assert_wp_error(WpErrorCode::TermInvalid);
}

#[tokio::test]
#[parallel]
async fn delete_err_cannot_delete() {
    api_client_as_subscriber()
        .terms()
        .delete(&TermEndpointType::Categories, &CATEGORY_ID_59)
        .await
        .assert_wp_error(WpErrorCode::CannotDelete);
}

#[tokio::test]
#[parallel]
async fn list_err_forbidden_context() {
    api_client_as_subscriber()
        .terms()
        .list_with_edit_context(&TermEndpointType::Categories, &TermListParams::default())
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
}

#[tokio::test]
#[parallel]
async fn list_err_post_invalid_id() {
    api_client()
        .terms()
        .list_with_edit_context(
            &TermEndpointType::Categories,
            &TermListParams {
                post: Some(POST_ID_INVALID),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId);
}

#[tokio::test]
#[parallel]
async fn retrieve_err_term_invalid() {
    api_client()
        .terms()
        .retrieve_with_edit_context(&TermEndpointType::Categories, &TERM_ID_INVALID)
        .await
        .assert_wp_error(WpErrorCode::TermInvalid);
}

#[tokio::test]
#[parallel]
async fn update_err_cannot_update() {
    api_client_as_subscriber()
        .terms()
        .update(
            &TermEndpointType::Categories,
            &CATEGORY_ID_59,
            &TermUpdateParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotUpdate);
}

#[tokio::test]
#[parallel]
async fn update_err_term_invalid() {
    api_client()
        .terms()
        .update(
            &TermEndpointType::Categories,
            &CATEGORY_ID_59,
            &TermUpdateParams {
                parent: Some(TERM_ID_INVALID),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::TermInvalid);
}
