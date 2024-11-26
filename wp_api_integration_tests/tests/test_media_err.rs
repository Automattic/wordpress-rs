use serial_test::parallel;
use wp_api::{
    media::{MediaId, MediaListParams, MediaUpdateParams},
    posts::WpApiParamPostsOrderBy,
    users::UserId,
    WpErrorCode,
};
use wp_api_integration_tests::{
    api_client, api_client_as_author, api_client_as_subscriber, AssertWpError, MEDIA_ID_611,
};

#[tokio::test]
#[parallel]
async fn delete_media_err_cannot_delete() {
    api_client_as_subscriber()
        .media()
        .delete(&MEDIA_ID_611)
        .await
        .assert_wp_error(WpErrorCode::CannotDelete);
}

#[tokio::test]
#[parallel]
async fn list_err_no_search_term_defined() {
    api_client()
        .media()
        .list_with_edit_context(&MediaListParams {
            orderby: Some(WpApiParamPostsOrderBy::Relevance),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::NoSearchTermDefined);
}

#[tokio::test]
#[parallel]
async fn list_err_order_by_include_missing_include() {
    api_client()
        .media()
        .list_with_edit_context(&MediaListParams {
            orderby: Some(WpApiParamPostsOrderBy::Include),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::OrderbyIncludeMissingInclude);
}

#[tokio::test]
#[parallel]
async fn list_err_media_invalid_page_number() {
    api_client()
        .media()
        .list_with_edit_context(&MediaListParams {
            page: Some(99999999),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::PostInvalidPageNumber);
}

#[tokio::test]
#[parallel]
async fn retrieve_media_err_forbidden_context() {
    api_client_as_subscriber()
        .media()
        .retrieve_with_edit_context(&MEDIA_ID_611)
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
}

#[tokio::test]
#[parallel]
async fn retrieve_media_err_media_invalid_id() {
    api_client()
        .media()
        .retrieve_with_edit_context(&MediaId(99999999))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId);
}

#[tokio::test]
#[parallel]
async fn update_media_err_cannot_edit() {
    api_client_as_author()
        .media()
        .update(&MEDIA_ID_611, &MediaUpdateParams::default())
        .await
        .assert_wp_error(WpErrorCode::CannotEdit);
}

#[tokio::test]
#[parallel]
async fn update_media_err_invalid_author() {
    api_client()
        .media()
        .update(
            &MEDIA_ID_611,
            &MediaUpdateParams {
                author: Some(UserId(99999999)),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::InvalidAuthor);
}

#[tokio::test]
#[parallel]
async fn update_media_err_invalid_template() {
    api_client()
        .media()
        .update(
            &MEDIA_ID_611,
            &MediaUpdateParams {
                template: Some("foo".to_string()),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::InvalidParam);
}

#[tokio::test]
#[parallel]
async fn update_media_err_post_invalid_id() {
    api_client_as_author()
        .media()
        .update(&MediaId(99999999), &MediaUpdateParams::default())
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId);
}
