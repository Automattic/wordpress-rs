use serial_test::serial;
use wp_api::comments::CommentDeleteParams;
use wp_api_integration_tests::{
    api_client,
    backend::{Backend, RestoreServer},
    FIRST_COMMENT_ID,
};

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
        !Backend::comments()
            .await
            .into_iter()
            .any(|c| c.comment_id == FIRST_COMMENT_ID.0),
        "Comment wasn't deleted"
    );

    RestoreServer::db().await;
}
