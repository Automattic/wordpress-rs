use serial_test::serial;
use wp_api_integration_tests::backend::{Backend, RestoreServer};
use wp_api_integration_tests::{api_client, TAG_ID_100};

#[tokio::test]
#[serial]
async fn delete_tag() {
    // Delete the tag using the API and ensure it's successful
    let tag_delete_response = api_client().tags().delete(&TAG_ID_100).await;
    assert!(tag_delete_response.is_ok(), "{:#?}", tag_delete_response);
    assert!(tag_delete_response.unwrap().data.deleted);

    // Assert that the tag was deleted
    assert!(
        !Backend::tags()
            .await
            .into_iter()
            .any(|u| u.id == TAG_ID_100.0),
        "Tag wasn't deleted"
    );

    RestoreServer::db().await;
}
