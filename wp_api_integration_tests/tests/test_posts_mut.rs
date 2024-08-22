use serial_test::serial;
use wp_api_integration_tests::{
    api_client,
    backend::{Backend, RestoreServer},
    FIRST_POST_ID,
};

#[tokio::test]
#[serial]
async fn delete_post() {
    // Delete the post using the API and ensure it's successful
    let post_delete_response = api_client().posts().delete(&FIRST_POST_ID).await;
    assert!(post_delete_response.is_ok(), "{:#?}", post_delete_response);

    // Assert that the post was deleted
    assert!(
        !Backend::posts(None)
            .await
            .into_iter()
            .any(|u| u.id == FIRST_POST_ID.0 as i64),
        "Post wasn't deleted"
    );

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn trash_post() {
    // Trash the post using the API and ensure it's successful
    let post_trash_response = api_client().posts().trash(&FIRST_POST_ID).await;
    assert!(post_trash_response.is_ok(), "{:#?}", post_trash_response);

    // Assert that the post was trashed
    let trashed_post = Backend::posts(Some("trash"))
        .await
        .into_iter()
        .find(|u| u.id == FIRST_POST_ID.0 as i64);
    assert!(trashed_post.is_some(), "Can't find the trashed post");
    assert_eq!(
        trashed_post.unwrap().post_status,
        "trash",
        "Post wasn't trashed"
    );

    RestoreServer::db().await;
}
