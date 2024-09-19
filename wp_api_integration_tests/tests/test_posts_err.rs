use serial_test::parallel;
use wp_api::{
    posts::{
        PostCreateParams, PostId, PostListParams, PostRetrieveParams, PostUpdateParams,
        WpApiParamPostsOrderBy,
    },
    users::UserId,
    WpErrorCode,
};
use wp_api_integration_tests::{
    api_client, api_client_as_author, api_client_as_subscriber, AssertWpError, TestCredentials,
    FIRST_POST_ID,
};

#[tokio::test]
#[parallel]
async fn create_post_err() {
    api_client()
        .posts()
        .create(&PostCreateParams::default())
        .await
        .assert_wp_error(WpErrorCode::EmptyContent)
}

#[tokio::test]
#[parallel]
async fn create_post_err_cannot_create2() {
    api_client_as_subscriber()
        .posts()
        .create(&PostCreateParams {
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::CannotCreate);
}

#[tokio::test]
#[parallel]
async fn create_post_err_cannot_create() {
    api_client_as_subscriber()
        .posts()
        .create(&PostCreateParams {
            title: Some("foo".to_string()),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::CannotCreate);
}

#[tokio::test]
#[parallel]
async fn delete_post_err_cannot_delete() {
    api_client_as_subscriber()
        .posts()
        .delete(&FIRST_POST_ID)
        .await
        .assert_wp_error(WpErrorCode::CannotDelete);
}

#[tokio::test]
#[parallel]
async fn list_err_no_search_term_defined() {
    api_client()
        .posts()
        .list_with_edit_context(&PostListParams {
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
        .posts()
        .list_with_edit_context(&PostListParams {
            orderby: Some(WpApiParamPostsOrderBy::Include),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::OrderbyIncludeMissingInclude);
}

#[tokio::test]
#[parallel]
async fn list_err_post_invalid_page_number() {
    api_client()
        .posts()
        .list_with_edit_context(&PostListParams {
            page: Some(99999999),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::PostInvalidPageNumber);
}

#[tokio::test]
#[parallel]
async fn retrieve_password_protected_post_err_wrong_password() {
    api_client()
        .posts()
        .retrieve_with_view_context(
            &PostId(TestCredentials::instance().password_protected_post_id),
            &PostRetrieveParams {
                password: Some("wrong_password".to_string()),
            },
        )
        .await
        .assert_wp_error(WpErrorCode::PostIncorrectPassword);
}

#[tokio::test]
#[parallel]
async fn retrieve_post_err_forbidden_context() {
    api_client_as_subscriber()
        .posts()
        .retrieve_with_edit_context(&FIRST_POST_ID, &PostRetrieveParams::default())
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
}

#[tokio::test]
#[parallel]
async fn retrieve_post_err_post_invalid_id() {
    api_client()
        .posts()
        .retrieve_with_edit_context(&PostId(99999999), &PostRetrieveParams::default())
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId);
}

#[tokio::test]
#[parallel]
async fn trash_post_err_already_trashed() {
    api_client()
        .posts()
        .trash(&PostId(TestCredentials::instance().trashed_post_id))
        .await
        .assert_wp_error(WpErrorCode::AlreadyTrashed);
}

#[tokio::test]
#[parallel]
async fn update_post_err_cannot_edit() {
    api_client_as_author()
        .posts()
        .update(
            &FIRST_POST_ID,
            &PostUpdateParams {
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::CannotEdit);
}

#[tokio::test]
#[parallel]
async fn update_post_err_invalid_author() {
    api_client()
        .posts()
        .update(
            &FIRST_POST_ID,
            &PostUpdateParams {
                author: Some(UserId(99999999)),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::InvalidAuthor);
}

#[tokio::test]
#[parallel]
async fn update_post_err_invalid_field() {
    api_client()
        .posts()
        .update(
            &FIRST_POST_ID,
            &PostUpdateParams {
                // A post can not be sticky and have a password.
                password: Some("any_password".to_string()),
                sticky: Some(true),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::InvalidField);
}

#[tokio::test]
#[parallel]
async fn update_post_err_invalid_template() {
    api_client()
        .posts()
        .update(
            &FIRST_POST_ID,
            &PostUpdateParams {
                template: Some("foo".to_string()),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::InvalidParam);
}
