use serial_test::serial;
use wp_api::comments::{
    CommentCreateParams, CommentDeleteParams, CommentStatus, CommentWithEditContext,
};
use wp_api_integration_tests::{
    api_client,
    backend::{Backend, RestoreServer},
    AssertResponse, FIRST_COMMENT_ID, FIRST_POST_ID,
};
use wp_cli::WpCliComment;

#[tokio::test]
#[serial]
async fn create_comment_with_just_content() {
    test_create_comment(
        &CommentCreateParams::new(FIRST_POST_ID, "foo".to_string()),
        |created_comment, comment_from_wp_cli| {
            assert_eq!(created_comment.content.raw, "foo");
            assert_eq!(comment_from_wp_cli.comment_content, "foo");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_comment_with_content_and_status() {
    test_create_comment(
        &CommentCreateParams::with_status(FIRST_POST_ID, "foo".to_string(), CommentStatus::Hold),
        |created_comment, comment_from_wp_cli| {
            assert_eq!(created_comment.content.raw, "foo");
            assert_eq!(created_comment.status, CommentStatus::Hold);
            assert_eq!(comment_from_wp_cli.comment_content, "foo");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn delete_comment() {
    // Delete the comment using the API and ensure it's successful
    let comment_delete_response = api_client()
        .comments()
        .delete(&FIRST_COMMENT_ID, &CommentDeleteParams::default())
        .await;
    assert!(
        comment_delete_response.is_ok(),
        "{:#?}",
        comment_delete_response
    );
    assert!(comment_delete_response.unwrap().data.deleted);

    // Assert that the comment was deleted
    assert!(
        !Backend::comments(None)
            .await
            .into_iter()
            .any(|c| c.comment_id == FIRST_COMMENT_ID.0),
        "Comment wasn't deleted"
    );

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn trash_comment() {
    // Trash the comment using the API and ensure it's successful
    let comment_trash_response = api_client()
        .comments()
        .trash(&FIRST_COMMENT_ID, &CommentDeleteParams::default())
        .await;
    assert!(
        comment_trash_response.is_ok(),
        "{:#?}",
        comment_trash_response
    );

    // Assert that the comment was trashed
    let trashed_comment = Backend::comments(Some("trash"))
        .await
        .into_iter()
        .find(|c| c.comment_id == FIRST_COMMENT_ID.0);
    assert!(trashed_comment.is_some(), "Can't find the trashed comment");

    RestoreServer::db().await;
}

async fn test_create_comment<F>(params: &CommentCreateParams, assert: F)
where
    F: Fn(CommentWithEditContext, WpCliComment),
{
    let created_comment = api_client()
        .comments()
        .create(params)
        .await
        .assert_response()
        .data;
    let created_comment_from_wp_cli = Backend::comment(&created_comment.id).await;
    assert(created_comment, created_comment_from_wp_cli);
    RestoreServer::db().await;
}
