use rstest::rstest;
use serial_test::parallel;
use wp_api::{
    WpErrorCode,
    comments::{
        CommentCreateParams, CommentCreateParamsBuilder, CommentDeleteParams, CommentListParams,
        CommentRetrieveParams, CommentStatus, CommentType, CommentUpdateParams,
    },
    posts::PostId,
};
use wp_api_integration_tests::{
    AssertWpError, COMMENT_ID_INVALID, FIRST_COMMENT_ID, FIRST_POST_ID, FIRST_USER_ID, POST_ID_555,
    POST_ID_DRAFT, POST_ID_INVALID, TestCredentials, USER_ID_INVALID, api_client,
    api_client_as_author, api_client_as_subscriber,
};

const NUMBER_OF_CHARS_FOR_TOO_LONG_PARAM: usize = 1000000;

#[tokio::test]
#[parallel]
async fn create_err_comment_author_column_length() {
    api_client()
        .comments()
        .create(
            &CommentCreateParamsBuilder::new(FIRST_POST_ID, "foo".to_string())
                .author_email(Some("foo@example.com".to_string()))
                .author_name(Some("x".repeat(NUMBER_OF_CHARS_FOR_TOO_LONG_PARAM)))
                .build(),
        )
        .await
        .assert_wp_error(WpErrorCode::CommentAuthorColumnLength);
}

#[tokio::test]
#[parallel]
async fn create_err_comment_author_data_required() {
    api_client()
        .comments()
        .create(
            &CommentCreateParamsBuilder::new(FIRST_POST_ID, "foo".to_string())
                .author_name(Some("x".repeat(NUMBER_OF_CHARS_FOR_TOO_LONG_PARAM)))
                .build(),
        )
        .await
        .assert_wp_error(WpErrorCode::CommentAuthorDataRequired);
}

#[tokio::test]
#[parallel]
async fn create_err_comment_author_email_column_length() {
    api_client()
        .comments()
        .create(
            &CommentCreateParamsBuilder::new(FIRST_POST_ID, "foo".to_string())
                .author_email(Some(format!(
                    "{}@example.com",
                    "x".repeat(NUMBER_OF_CHARS_FOR_TOO_LONG_PARAM)
                )))
                .author_name(Some("foo".to_string()))
                .build(),
        )
        .await
        .assert_wp_error(WpErrorCode::CommentAuthorEmailColumnLength);
}

#[tokio::test]
#[parallel]
async fn create_err_comment_author_invalid() {
    api_client()
        .comments()
        .create(
            &CommentCreateParamsBuilder::new(FIRST_POST_ID, "foo".to_string())
                .author(Some(USER_ID_INVALID))
                .build(),
        )
        .await
        .assert_wp_error(WpErrorCode::CommentAuthorInvalid);
}

#[tokio::test]
#[parallel]
async fn create_err_comment_author_url_column_length() {
    api_client()
        .comments()
        .create(
            &CommentCreateParamsBuilder::new(FIRST_POST_ID, "foo".to_string())
                .author_email(Some("foo@example.com".to_string()))
                .author_name(Some("foo".to_string()))
                .author_url(Some(format!(
                    "{}.example.com",
                    "x".repeat(NUMBER_OF_CHARS_FOR_TOO_LONG_PARAM)
                )))
                .build(),
        )
        .await
        .assert_wp_error(WpErrorCode::CommentAuthorUrlColumnLength);
}

#[tokio::test]
#[parallel]
async fn create_err_comment_closed() {
    api_client()
        .comments()
        .create(&CommentCreateParams::new(POST_ID_555, "foo".to_string()))
        .await
        .assert_wp_error(WpErrorCode::CommentClosed);
}

#[tokio::test]
#[parallel]
async fn create_err_comment_content_column_length() {
    api_client()
        .comments()
        .create(&CommentCreateParams::new(
            FIRST_POST_ID,
            "x".repeat(NUMBER_OF_CHARS_FOR_TOO_LONG_PARAM),
        ))
        .await
        .assert_wp_error(WpErrorCode::ContentColumnLength);
}

#[tokio::test]
#[parallel]
async fn create_err_comment_content_invalid() {
    api_client()
        .comments()
        .create(&CommentCreateParams::new(FIRST_POST_ID, "".to_string()))
        .await
        .assert_wp_error(WpErrorCode::CommentContentInvalid);
}

#[tokio::test]
#[parallel]
async fn create_err_comment_draft_post() {
    api_client()
        .comments()
        .create(&CommentCreateParams::new(POST_ID_DRAFT, "foo".to_string()))
        .await
        .assert_wp_error(WpErrorCode::CommentDraftPost);
}

#[tokio::test]
#[parallel]
async fn create_err_comment_invalid_author() {
    api_client_as_author()
        .comments()
        .create(
            &CommentCreateParamsBuilder::new(FIRST_POST_ID, "foo".to_string())
                // A different user from the one making the request
                .author(Some(FIRST_USER_ID))
                .build(),
        )
        .await
        .assert_wp_error(WpErrorCode::CommentInvalidAuthor);
}

#[tokio::test]
#[parallel]
async fn create_err_comment_invalid_author_ip() {
    api_client_as_author()
        .comments()
        .create(
            &CommentCreateParamsBuilder::new(FIRST_POST_ID, "foo".to_string())
                // A different IP from the one making the request
                .author_ip(Some("8.8.8.8".to_string()))
                .build(),
        )
        .await
        .assert_wp_error(WpErrorCode::CommentInvalidAuthorIp);
}

#[tokio::test]
#[parallel]
async fn create_err_comment_invalid_post_id() {
    api_client()
        .comments()
        .create(&CommentCreateParams::new(
            POST_ID_INVALID,
            "foo".to_string(),
        ))
        .await
        .assert_wp_error(WpErrorCode::CommentInvalidPostId);
}

#[tokio::test]
#[rstest]
#[parallel]
async fn create_err_comment_invalid_status(
    #[values(
        CommentStatus::Approved,
        CommentStatus::Hold,
        CommentStatus::Spam,
        CommentStatus::Trash
    )]
    status: CommentStatus,
) {
    api_client_as_author()
        .comments()
        .create(
            &CommentCreateParamsBuilder::new(FIRST_POST_ID, "foo".to_string())
                .status(Some(status))
                .build(),
        )
        .await
        .assert_wp_error(WpErrorCode::CommentInvalidStatus);
}

#[tokio::test]
#[parallel]
async fn create_err_comment_trash_post() {
    api_client()
        .comments()
        .create(&CommentCreateParams::new(
            PostId(TestCredentials::instance().trashed_post_id),
            "foo".to_string(),
        ))
        .await
        .assert_wp_error(WpErrorCode::CommentTrashPost);
}

#[tokio::test]
#[parallel]
async fn delete_err_cannot_delete() {
    api_client_as_subscriber()
        .comments()
        .delete(&FIRST_COMMENT_ID, &CommentDeleteParams::default())
        .await
        .assert_wp_error(WpErrorCode::CannotDelete);
}

#[tokio::test]
#[parallel]
async fn list_err_cannot_read() {
    api_client_as_subscriber()
        .comments()
        .list_with_view_context(&CommentListParams {
            post: vec![PostId(0)],
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::CannotRead);
}

#[tokio::test]
#[parallel]
async fn list_err_cannot_read_post() {
    api_client_as_subscriber()
        .comments()
        .list_with_view_context(&CommentListParams {
            post: vec![PostId(
                TestCredentials::instance().password_protected_post_id,
            )],
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::CannotReadPost);
}

#[tokio::test]
#[parallel]
async fn list_err_forbidden_context() {
    api_client_as_subscriber()
        .comments()
        .list_with_edit_context(&CommentListParams {
            post: vec![FIRST_POST_ID],
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
}

#[tokio::test]
#[parallel]
async fn list_err_forbidden_param_author() {
    api_client_as_subscriber()
        .comments()
        .list_with_view_context(&CommentListParams {
            author: vec![FIRST_USER_ID],
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::ForbiddenParam);
}

#[tokio::test]
#[parallel]
async fn list_err_forbidden_param_author_email() {
    api_client_as_subscriber()
        .comments()
        .list_with_view_context(&CommentListParams {
            author_email: Some("foo@example.com".to_string()),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::ForbiddenParam);
}

#[tokio::test]
#[parallel]
async fn list_err_forbidden_param_author_exclude() {
    api_client_as_subscriber()
        .comments()
        .list_with_view_context(&CommentListParams {
            author_exclude: vec![FIRST_USER_ID],
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::ForbiddenParam);
}

#[tokio::test]
#[rstest]
#[parallel]
async fn list_err_forbidden_param_comment_type(
    #[values(CommentType::Pingback, CommentType::Trackback)] comment_type: CommentType,
) {
    api_client_as_subscriber()
        .comments()
        .list_with_view_context(&CommentListParams {
            comment_type: Some(comment_type),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::ForbiddenParam);
}

#[tokio::test]
#[rstest]
#[parallel]
async fn list_err_forbidden_param_status(
    #[values(CommentStatus::Hold, CommentStatus::Spam, CommentStatus::Trash)] status: CommentStatus,
) {
    api_client_as_subscriber()
        .comments()
        .list_with_view_context(&CommentListParams {
            status: Some(status),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::ForbiddenParam);
}

#[tokio::test]
#[parallel]
async fn retrieve_err_comment_invalid_id() {
    api_client()
        .comments()
        .retrieve_with_edit_context(&COMMENT_ID_INVALID, &CommentRetrieveParams::default())
        .await
        .assert_wp_error(WpErrorCode::CommentInvalidId);
}

#[tokio::test]
#[parallel]
async fn update_err_cannot_edit() {
    api_client_as_subscriber()
        .comments()
        .update(&FIRST_COMMENT_ID, &CommentUpdateParams::default())
        .await
        .assert_wp_error(WpErrorCode::CannotEdit);
}

#[tokio::test]
#[parallel]
async fn update_err_comment_invalid_post_id() {
    api_client()
        .comments()
        .update(
            &FIRST_COMMENT_ID,
            &CommentUpdateParams {
                post: Some(POST_ID_INVALID),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::CommentInvalidPostId);
}
