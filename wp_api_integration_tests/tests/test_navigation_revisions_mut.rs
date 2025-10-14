use wp_api::{navigation_revisions::NavigationRevisionId, navigations::NavigationId};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[serial]
async fn delete_navigation_revision() {
    let revision_id = revision_id_for_navigation_id();
    let revision_delete_response = api_client()
        .navigation_revisions()
        .delete(&navigation_id(), &revision_id)
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

fn navigation_id() -> NavigationId {
    NavigationId(TestCredentials::instance().navigation_id)
}

fn revision_id_for_navigation_id() -> NavigationRevisionId {
    NavigationRevisionId(TestCredentials::instance().revision_id_for_navigation_id)
}
