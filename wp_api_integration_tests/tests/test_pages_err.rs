use wp_api::{
    pages::{
        PageCreateParams, PageId, PageListParams, PageRetrieveParams, PageUpdateParams,
        WpApiParamPagesOrderBy,
    },
    users::UserId,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn create_page_err_cannot_create() {
    api_client_as_subscriber()
        .pages()
        .create(&PageCreateParams {
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::CannotCreate);
}

#[tokio::test]
#[parallel]
async fn create_page_err_cannot_create2() {
    api_client_as_subscriber()
        .pages()
        .create(&PageCreateParams {
            title: Some("foo".to_string()),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::CannotCreate);
}

#[tokio::test]
#[parallel]
async fn delete_page_err_cannot_delete() {
    api_client_as_subscriber()
        .pages()
        .delete(&PageId(TestCredentials::instance().first_page_id))
        .await
        .assert_wp_error(WpErrorCode::CannotDelete);
}

#[tokio::test]
#[parallel]
async fn list_err_no_search_term_defined() {
    api_client()
        .pages()
        .list_with_edit_context(&PageListParams {
            orderby: Some(WpApiParamPagesOrderBy::Relevance),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::NoSearchTermDefined);
}

#[tokio::test]
#[parallel]
async fn list_err_order_by_include_missing_include() {
    api_client()
        .pages()
        .list_with_edit_context(&PageListParams {
            orderby: Some(WpApiParamPagesOrderBy::Include),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::OrderbyIncludeMissingInclude);
}

#[tokio::test]
#[parallel]
async fn list_err_post_invalid_page_number() {
    api_client()
        .pages()
        .list_with_edit_context(&PageListParams {
            page: Some(99999999),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::PostInvalidPageNumber);
}

#[tokio::test]
#[parallel]
async fn retrieve_password_protected_page_err_wrong_password() {
    api_client()
        .pages()
        .retrieve_with_view_context(
            &PageId(TestCredentials::instance().password_protected_page_id),
            &PageRetrieveParams {
                password: Some("wrong_password".to_string()),
            },
        )
        .await
        .assert_wp_error(WpErrorCode::PostIncorrectPassword);
}

#[tokio::test]
#[parallel]
async fn retrieve_page_err_forbidden_context() {
    api_client_as_subscriber()
        .pages()
        .retrieve_with_edit_context(
            &PageId(TestCredentials::instance().first_page_id),
            &PageRetrieveParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
}

#[tokio::test]
#[parallel]
async fn retrieve_page_err_post_invalid_id() {
    api_client()
        .pages()
        .retrieve_with_edit_context(&PageId(99999999), &PageRetrieveParams::default())
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId);
}

#[tokio::test]
#[parallel]
async fn trash_page_err_already_trashed() {
    api_client()
        .pages()
        .trash(&PageId(TestCredentials::instance().trashed_page_id))
        .await
        .assert_wp_error(WpErrorCode::AlreadyTrashed);
}

#[tokio::test]
#[parallel]
async fn update_page_err_cannot_edit() {
    api_client_as_author()
        .pages()
        .update(
            &PageId(TestCredentials::instance().first_page_id),
            &PageUpdateParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotEdit);
}

#[tokio::test]
#[parallel]
async fn update_page_err_invalid_author() {
    api_client()
        .pages()
        .update(
            &PageId(TestCredentials::instance().first_page_id),
            &PageUpdateParams {
                author: Some(UserId(99999999)),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::InvalidAuthor);
}

#[tokio::test]
#[parallel]
async fn update_page_err_invalid_parent() {
    api_client()
        .pages()
        .update(
            &PageId(TestCredentials::instance().first_page_id),
            &PageUpdateParams {
                parent: Some(PageId(99999999)),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId);
}

#[tokio::test]
#[parallel]
async fn update_page_err_invalid_param() {
    api_client()
        .pages()
        .update(
            &PageId(TestCredentials::instance().first_page_id),
            &PageUpdateParams {
                template: Some("nonexistent-template".to_string()),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::InvalidParam);
}
