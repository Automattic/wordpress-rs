use wp_api::{
    post_revisions::PostRevisionId, posts::PostId,
    request::endpoint::posts_endpoint::PostEndpointType,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[serial]
async fn delete_navigation_revision() {
    let revision_id = revision_id_for_navigation_id();
    let revision_delete_response = api_client()
        .post_revisions()
        .delete(
            &PostEndpointType::Navigation,
            &navigation_id(),
            &revision_id,
        )
        .await;

    assert!(
        revision_delete_response.is_ok(),
        "{revision_delete_response:#?}"
    );

    let delete_response = revision_delete_response.unwrap().data;
    assert!(delete_response.deleted);
    assert_eq!(delete_response.previous.id, revision_id);

    RestoreServer::db().await;
}

fn navigation_id() -> PostId {
    PostId(TestCredentials::instance().navigation_id)
}

fn revision_id_for_navigation_id() -> PostRevisionId {
    PostRevisionId(TestCredentials::instance().revision_id_for_navigation_id)
}
